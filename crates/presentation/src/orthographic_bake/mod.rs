//! GPU composition of original orthographic bake tiles on an injected device.
//! Retained outputs have no CPU viewport or readback storage.
use bytemuck::{Pod, Zeroable};
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, RenderError, Result, wgpu,
};
use rusterix::orthographic_bake::{OrthographicBakeLighting, OrthographicBakeSource};
use scenevm::Camera3D;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use theframework_core::Uuid;
use wgpu::util::DeviceExt;

const SOURCE_BUDGET: u64 = 64 * 1024 * 1024;
const PAGE_BYTES: u64 = 8 * 1024 * 1024;
mod paging;
use paging::{FrameKey, Paging};
mod job;
mod overlay;
pub use job::{BakeAdvance, OrthographicBakeJob};
pub use overlay::OrthographicBakeOverlay;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
struct Params {
    size: [u32; 4], // output size, tile size, depth enabled
    tile: [i32; 4], // coordinates, exact bounds, sun enabled
    dispatch: [u32; 4],
    plane: [f32; 4], // center u/v, half width/height
    bounds: [f32; 4],
    source: [f32; 4], // ppu, span, depth rebase, spare
    forward: [f32; 4],
    sun: [f32; 4],
    sun_color: [f32; 4],
    bits: [u32; 4], // infinity, identity zero
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Pixel {
    color: u32,
    depth: f32,
    albedo: u32,
    normal: u32,
    material: u32,
    surface_channels: u32,
}
enum TileState {
    Ready(Arc<renderkit::SourceBuffer>),
    Missing,
    Busy,
}
/// The depth buffer stores camera-linear f32 values (infinity for no surface).
/// It is GPU-only and intended for a subsequent depth-aware overlay pass.
#[derive(Clone)]
pub struct OrthographicBakeOutput {
    image: Arc<GpuImage>,
    depth: wgpu::Buffer,
    source_revision: Uuid,
    request: Option<(Uuid, Uuid)>,
    size: (u32, u32),
    depth_enabled: bool,
}
impl OrthographicBakeOutput {
    pub fn image(&self) -> &Arc<GpuImage> {
        &self.image
    }
    pub fn depth(&self) -> &wgpu::Buffer {
        &self.depth
    }
    /// A paged progressive preview starts at this revision and may include
    /// newer samples in the same request. Committed outputs use an exact source
    /// revision; request() is None for them.
    pub fn source_revision(&self) -> Uuid {
        self.source_revision
    }
    pub fn request(&self) -> Option<(Uuid, Uuid)> {
        self.request
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
    pub fn depth_enabled(&self) -> bool {
        self.depth_enabled
    }
}
pub struct OrthographicBakeRenderer {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    clear: wgpu::ComputePipeline,
    draw: wgpu::ComputePipeline,
    dummy: wgpu::Buffer,
    target: Option<OrthographicBakeOutput>,
    publication: Option<Uuid>,
    tiles: HashMap<(i32, i32), Weak<renderkit::SourceBuffer>>,
    complete: Option<FrameKey>,
    paging: Option<Paging>,
    page_buffer: Weak<renderkit::SourceBuffer>,
    waiting: bool,
    visible_tiles: usize,
}
impl OrthographicBakeRenderer {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("orthographic bake sources"),
                entries: &[
                    buffer(0, wgpu::BufferBindingType::Uniform),
                    buffer(1, wgpu::BufferBindingType::Storage { read_only: true }),
                    buffer(2, wgpu::BufferBindingType::Storage { read_only: false }),
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("orthographic bake composition"),
                source: wgpu::ShaderSource::Wgsl(include_str!("compose.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = |entry| {
            context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
        };
        let clear = pipeline("clear");
        let draw = pipeline("draw");
        let dummy = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("empty bake source"),
            size: std::mem::size_of::<Pixel>() as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Ok(Self {
            context,
            layout,
            clear,
            draw,
            dummy,
            target: None,
            publication: None,
            tiles: HashMap::new(),
            complete: None,
            paging: None,
            page_buffer: Weak::new(),
            waiting: false,
            visible_tiles: 0,
        })
    }
    pub fn source_resident_bytes(&self) -> u64 {
        self.tiles
            .values()
            .filter_map(Weak::upgrade)
            .map(|b| b.bytes())
            .sum::<u64>()
            + self.page_buffer.upgrade().map_or(0, |b| b.bytes())
    }
    pub fn source_budget_bytes(&self) -> u64 {
        SOURCE_BUDGET
    }
    pub fn waiting(&self) -> bool {
        self.waiting
    }
    pub fn visible_tiles(&self) -> usize {
        self.visible_tiles
    }
    pub fn source_pool_usage(&self) -> renderkit::SourcePoolUsage {
        self.context.source_pool_usage()
    }

    fn pack(
        source: rusterix::orthographic_bake::OrthographicBakeTileSource<'_>,
        count: usize,
        pixels: &mut Vec<Pixel>,
    ) -> Result<()> {
        if source.color.len() != count * 4 {
            return Err(RenderError("incomplete bake color tile".into()));
        }
        let word = |channel: &[u8], i: usize| {
            channel
                .get(i * 4..i * 4 + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .unwrap_or(0)
        };
        pixels.extend((0..count).map(|i| {
            Pixel {
                color: word(source.color, i),
                depth: source.depth.get(i).copied().unwrap_or(f32::INFINITY),
                albedo: word(source.albedo, i),
                normal: word(source.normal, i),
                material: word(source.material, i),
                surface_channels: u32::from(
                    [source.albedo, source.normal, source.material]
                        .into_iter()
                        .all(|c| c.len() >= (i + 1) * 4),
                ),
            }
        }));
        Ok(())
    }
    fn tile(
        &mut self,
        at: (i32, i32),
        source: &OrthographicBakeSource<'_>,
        size: u32,
    ) -> Result<TileState> {
        if let Some(buffer) = self.tiles.get(&at).and_then(Weak::upgrade) {
            self.context.touch_source_buffer(&buffer);
            return Ok(TileState::Ready(buffer));
        }
        let count = u64::from(size)
            .checked_mul(u64::from(size))
            .ok_or_else(|| RenderError("invalid bake tile size".into()))?;
        let bytes = count
            .checked_mul(std::mem::size_of::<Pixel>() as u64)
            .ok_or_else(|| RenderError("bake tile size overflow".into()))?;
        let Some(lease) = source.try_tile(at.0, at.1).map_err(RenderError)? else {
            return Ok(TileState::Missing);
        };
        let mut pixels = Vec::with_capacity(count as usize);
        Self::pack(lease.source(), count as usize, &mut pixels)?;
        let Some(buffer) = self
            .context
            .try_source_buffer(bytes, Some(bytemuck::cast_slice(&pixels)))
        else {
            return Ok(TileState::Busy);
        };
        self.context.record_upload(bytes);
        self.tiles.insert(at, Arc::downgrade(&buffer));
        Ok(TileState::Ready(buffer))
    }
    fn target(
        &mut self,
        size: (u32, u32),
        revision: Uuid,
        depth_enabled: bool,
        request: Option<(Uuid, Uuid)>,
    ) -> Result<OrthographicBakeOutput> {
        let limit = self.context.device.limits();
        let bytes = u64::from(size.0) * u64::from(size.1) * 4;
        if size.0 == 0
            || size.1 == 0
            || size.0 > limit.max_texture_dimension_2d
            || size.1 > limit.max_texture_dimension_2d
            || bytes > u64::from(limit.max_storage_buffer_binding_size)
        {
            return Err(RenderError("bake output exceeds GPU target limits".into()));
        }
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            let descriptor = ImageDescriptor {
                width: size.0,
                height: size.1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Premultiplied,
            };
            let texture = self
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("orthographic bake output"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: descriptor.format,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
            let image = Arc::new(GpuImage::from_texture(&self.context, texture, descriptor)?);
            let depth = self.context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("orthographic bake camera depth"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            self.target = Some(OrthographicBakeOutput {
                image,
                depth,
                source_revision: revision,
                request,
                size,
                depth_enabled,
            });
        }
        let target = self.target.as_mut().unwrap();
        target.source_revision = revision;
        target.depth_enabled = depth_enabled;
        target.request = request;
        Ok(target.clone())
    }
    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OrthographicBakeOutput,
        params: &Params,
        source: &wgpu::Buffer,
        clear: bool,
    ) {
        let uniform = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bake projection"),
                contents: bytemuck::bytes_of(params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.context
            .record_upload(std::mem::size_of::<Params>() as u64);
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: source.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: target.depth.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&target.image.view),
                    },
                ],
            });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("orthographic bake compose"),
            timestamp_writes: None,
        });
        pass.set_pipeline(if clear { &self.clear } else { &self.draw });
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(
            (params.dispatch[2] - params.dispatch[0]).div_ceil(8),
            (params.dispatch[3] - params.dispatch[1]).div_ceil(8),
            1,
        );
    }
    pub fn render(
        &mut self,
        source: &OrthographicBakeSource<'_>,
        camera: &Camera3D,
        size: (u32, u32),
        lighting: Option<OrthographicBakeLighting>,
    ) -> Result<Option<OrthographicBakeOutput>> {
        self.waiting = true;
        self.context.poll()?;
        if self.context.is_lost() {
            return Err(RenderError("bake renderer device is lost".into()));
        }
        if !source.matches_camera(camera) {
            return Err(RenderError(
                "bake source camera orientation does not match".into(),
            ));
        }
        let asset = source.asset();
        let ppu = asset.pixels_per_world_unit.max(0.0001);
        let span = asset.tile_size as f32 / ppu;
        let half_h = camera.ortho_half_h.max(0.0001);
        let half_w = half_h * size.0 as f32 / size.1.max(1) as f32;
        let right = scenevm::prelude::Vec3::from(asset.camera_right);
        let up = scenevm::prelude::Vec3::from(asset.camera_up);
        let center = [camera.pos.dot(right), camera.pos.dot(up)];
        if asset.tile_size == 0
            || ![ppu, span, half_w, half_h, center[0], center[1]]
                .into_iter()
                .all(f32::is_finite)
            || !asset.projected_bounds.into_iter().all(f32::is_finite)
        {
            return Err(RenderError("invalid bake projection".into()));
        }
        let [min_u, max_u, min_v, max_v] = asset.projected_bounds;
        let exact = asset.version >= 2 && max_u > min_u && max_v > min_v;
        let origin = if exact { [min_u, min_v] } else { [0.; 2] };
        let sun = lighting.filter(|light| light.sun_enabled);
        let mut params = Params {
            size: [
                size.0,
                size.1,
                asset.tile_size,
                u32::from(asset.version >= 4),
            ],
            tile: [0, 0, i32::from(exact), i32::from(sun.is_some())],
            dispatch: [0, 0, size.0, size.1],
            plane: [center[0], center[1], half_w, half_h],
            bounds: asset.projected_bounds,
            source: [
                ppu,
                span,
                asset.camera_forward_origin
                    - camera
                        .pos
                        .dot(scenevm::prelude::Vec3::from(asset.camera_forward).normalized()),
                0.,
            ],
            forward: [camera.forward.x, camera.forward.y, camera.forward.z, 0.],
            sun: sun.map_or([0.; 4], |s| {
                [
                    s.sun_direction.x,
                    s.sun_direction.y,
                    s.sun_direction.z,
                    s.sun_intensity,
                ]
            }),
            sun_color: sun.map_or([0.; 4], |s| {
                [s.sun_color.x, s.sun_color.y, s.sun_color.z, 0.]
            }),
            bits: [f32::INFINITY.to_bits(), 0, 0, 0],
        };
        let mut visible = Vec::new();
        let tile_bytes = u64::from(asset.tile_size)
            .checked_mul(u64::from(asset.tile_size))
            .and_then(|count| count.checked_mul(std::mem::size_of::<Pixel>() as u64))
            .ok_or_else(|| RenderError("bake tile size overflow".into()))?;
        for at in source.positions() {
            let lo = [
                origin[0] + at.0 as f32 * span,
                origin[1] + at.1 as f32 * span,
            ];
            let hi = [lo[0] + span, lo[1] + span];
            // Conservative destination rectangle; shader performs the exact CPU
            // reference lookup and bounds test for each dispatched pixel.
            let x0 = (((lo[0] - center[0]) / half_w + 1.) * size.0 as f32 * 0.5).floor() - 2.;
            let x1 = (((hi[0] - center[0]) / half_w + 1.) * size.0 as f32 * 0.5).ceil() + 2.;
            let y0 = ((1. - (hi[1] - center[1]) / half_h) * size.1 as f32 * 0.5).floor() - 2.;
            let y1 = ((1. - (lo[1] - center[1]) / half_h) * size.1 as f32 * 0.5).ceil() + 2.;
            params.dispatch = [
                x0.clamp(0., size.0 as f32) as u32,
                y0.clamp(0., size.1 as f32) as u32,
                x1.clamp(0., size.0 as f32) as u32,
                y1.clamp(0., size.1 as f32) as u32,
            ];
            if params.dispatch[0] >= params.dispatch[2] || params.dispatch[1] >= params.dispatch[3]
            {
                continue;
            }
            visible.push((at, params.dispatch));
        }
        if tile_bytes == 0
            || tile_bytes > SOURCE_BUDGET / 2
            || tile_bytes > u64::from(self.context.device.limits().max_storage_buffer_binding_size)
        {
            return Err(RenderError("bake tile exceeds source page limits".into()));
        }
        self.visible_tiles = visible.len();
        params.dispatch = [0, 0, size.0, size.1];
        let key = FrameKey {
            revision: source.revision(),
            request: source.request(),
            params,
        };
        self.cancel_incompatible(&key);
        if self.complete.as_ref() == Some(&key) {
            self.waiting = false;
            return Ok(self.target.clone());
        }
        if (visible.len() as u64).saturating_mul(tile_bytes) > PAGE_BYTES || visible.len() > 64 {
            self.tiles.clear();
            visible.sort_unstable_by_key(|(at, _)| *at);
            return self.advance_paged(source, key, visible, tile_bytes);
        }
        if self.publication != Some(source.revision()) {
            self.tiles.clear();
            self.publication = Some(source.revision());
        }
        let positions: std::collections::HashSet<_> = visible.iter().map(|(at, _)| *at).collect();
        self.tiles
            .retain(|at, b| positions.contains(at) && b.strong_count() > 0);
        // Pin cache hits before reserving initializer staging: pool eviction
        // cannot turn one of those hits into an unaccounted cold upload.
        let pinned: Vec<_> = visible
            .iter()
            .filter_map(|(at, _)| self.tiles.get(at).and_then(Weak::upgrade))
            .collect();
        let upload_bytes = (visible.len() - pinned.len()) as u64 * tile_bytes;
        let Some(submission) = self.context.try_source_submission(upload_bytes) else {
            return Ok(None);
        };
        let mut resolved = Vec::new();
        for (at, dispatch) in visible {
            match self.tile(at, source, asset.tile_size)? {
                TileState::Ready(buffer) => resolved.push((at, dispatch, buffer)),
                TileState::Missing => (),
                TileState::Busy => return Ok(None),
            }
        }
        let output = self.target(
            size,
            source.revision(),
            asset.version >= 4,
            source.request(),
        )?;
        params.dispatch = [0, 0, size.0, size.1];
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("orthographic bake presentation"),
                });
        self.dispatch(&mut encoder, &output, &params, &self.dummy, true);
        let mut leases = Vec::new();
        for (at, dispatch, buffer) in resolved {
            params.dispatch = dispatch;
            params.tile[0] = at.0;
            params.tile[1] = at.1;
            self.dispatch(&mut encoder, &output, &params, buffer.buffer(), false);
            leases.push(buffer.pin());
        }
        self.context.queue.submit(Some(encoder.finish()));
        self.context.queue.on_submitted_work_done(move || {
            drop(leases);
            drop(submission);
        });
        self.complete = Some(key);
        self.waiting = false;
        Ok(Some(output))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod job_tests;
