//! Deterministic base-map overviews on an injected device. CPU work describes
//! ordered geometry/bins and source assets; no destination pixels are produced.
use bytemuck::{Pod, Zeroable};
use renderkit::{GpuContext, GpuImage, ImageDescriptor, RenderError, Result, wgpu};
use rusterix::{Assets, PrimitiveMode, RepeatMode, Scene, Tile};
use scenevm::prelude::{Mat3, Vec3};
use std::{collections::HashMap, sync::Arc};
use theframework_core::Uuid;
mod sources;
use sources::{Source, Sources};
const CELL: u32 = 8;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    size: [u32; 4],
    source: [u32; 4],
    page: [f32; 4],
    period: [f32; 4],
    world: [f32; 4],
    background: [u32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Triangle {
    p: [[f32; 4]; 3],
    edges: [[f32; 4]; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Occlusion {
    bounds: [f32; 4],
    value: [f32; 4],
}
struct Target {
    size: (u32, u32),
    packed: renderkit::WorkingBufferLease,
    image: Arc<GpuImage>,
}
/// A view retains this owner independently of the host surface and other views.
/// Source cache limits apply to original assets; the sole destination is GPU-only.
pub struct Overview2DRenderer {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    clear: wgpu::ComputePipeline,
    draw: wgpu::ComputePipeline,
    resolve: wgpu::ComputePipeline,
    target: Option<Target>,
    sources: Sources,
    empty: renderkit::WorkingBufferLease,
    dummy: Arc<Source>,
}
impl Overview2DRenderer {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        let stage = wgpu::ShaderStages::COMPUTE;
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: stage,
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
                label: Some("overview2d layout"),
                entries: &[
                    buffer(0, wgpu::BufferBindingType::Uniform),
                    buffer(1, wgpu::BufferBindingType::Storage { read_only: false }),
                    buffer(2, wgpu::BufferBindingType::Storage { read_only: true }),
                    buffer(3, wgpu::BufferBindingType::Storage { read_only: true }),
                    buffer(4, wgpu::BufferBindingType::Storage { read_only: true }),
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: stage,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    buffer(6, wgpu::BufferBindingType::Storage { read_only: true }),
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: stage,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
        let module = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("overview2d"),
                source: wgpu::ShaderSource::Wgsl(include_str!("overview.wgsl").into()),
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
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
        };
        let empty =
            context.acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(96))?;
        context.queue.write_buffer(empty.buffer(), 0, &[0; 96]);
        context.record_upload(96);
        let mut sources = Sources::new(context.clone());
        let dummy = sources.dummy()?;
        Ok(Self {
            clear: pipeline("clear"),
            draw: pipeline("draw"),
            resolve: pipeline("resolve"),
            context,
            layout,
            target: None,
            sources,
            empty,
            dummy,
        })
    }
    pub fn source_resident_bytes(&self) -> usize {
        self.sources.bytes()
    }
    fn buffer(
        &self,
        bytes: &[u8],
        usage: wgpu::BufferUsages,
    ) -> Result<renderkit::WorkingBufferLease> {
        let lease = self
            .context
            .acquire_working_buffer(renderkit::WorkingBufferDescriptor {
                bytes: bytes.len() as u64,
                usage: usage | wgpu::BufferUsages::COPY_DST,
            })?;
        self.context.queue.write_buffer(lease.buffer(), 0, bytes);
        self.context.record_upload(bytes.len() as u64);
        Ok(lease)
    }
    fn target(&mut self, size: (u32, u32)) -> Result<()> {
        if self.target.as_ref().is_some_and(|t| t.size == size) {
            return Ok(());
        }
        let limit = self.context.device.limits().max_texture_dimension_2d;
        let bytes = u64::from(size.0) * u64::from(size.1) * 4;
        if size.0 == 0
            || size.1 == 0
            || size.0 > limit
            || size.1 > limit
            || bytes > u64::from(self.context.device.limits().max_storage_buffer_binding_size)
        {
            return Err(RenderError("invalid overview target size".into()));
        }
        let lease = self.context.acquire_target(renderkit::TargetDescriptor {
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            ..renderkit::TargetDescriptor::color(size.0, size.1, wgpu::TextureFormat::Rgba8Unorm)
        })?;
        let image = GpuImage::from_target(
            &self.context,
            lease,
            ImageDescriptor {
                width: size.0,
                height: size.1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: renderkit::ColorSpace::Srgb,
                alpha_mode: renderkit::AlphaMode::Opaque,
            },
        )?;
        let packed = self
            .context
            .acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(bytes))?;
        self.target = Some(Target {
            size,
            packed,
            image,
        });
        Ok(())
    }
    fn bindings(
        &self,
        params: &wgpu::Buffer,
        triangles: &wgpu::Buffer,
        bins: &wgpu::Buffer,
        occlusions: &wgpu::Buffer,
        source: &Source,
    ) -> wgpu::BindGroup {
        let target = self.target.as_ref().unwrap();
        self.context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("overview batch"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: target.packed.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: triangles.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: bins.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: occlusions.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&source.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: source.sizes.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(&target.image.view),
                    },
                ],
            })
    }
    fn dispatch(
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        bindings: &wgpu::BindGroup,
        size: (u32, u32),
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("overview ordered pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bindings, &[]);
        pass.dispatch_workgroups(size.0.div_ceil(CELL), size.1.div_ceil(CELL), 1);
    }
    /// Stable encoded-byte source-over, first-match occlusion and original tile
    /// addressing. Occlusion is an explicit caller policy, just like installing
    /// Rasterizer::mapmini; Scene::mapmini alone did not enable it in the legacy
    /// overview. Geometry bins bound each pixel's work without CPU coverage.
    pub fn render(
        &mut self,
        scene: &Scene,
        assets: &Assets,
        size: (u32, u32),
        transform: Mat3<f32>,
        namespace: Uuid,
        revision: u64,
        occlusion_regions: &[(rusterix::BBox, f32)],
    ) -> Result<Arc<GpuImage>> {
        self.render_with_background(
            scene,
            assets,
            size,
            transform,
            namespace,
            revision,
            occlusion_regions,
            [42; 3],
        )
    }
    /// Render an opaque base with the caller's canvas clear. Custom authoring
    /// maps use black, matching the legacy rasterizer's initially empty RGB.
    pub fn render_with_background(
        &mut self,
        scene: &Scene,
        assets: &Assets,
        size: (u32, u32),
        transform: Mat3<f32>,
        namespace: Uuid,
        revision: u64,
        occlusion_regions: &[(rusterix::BBox, f32)],
        background: [u8; 3],
    ) -> Result<Arc<GpuImage>> {
        if self.context.is_lost() {
            return Err(RenderError("overview device lost".into()));
        }
        if !scene.lights.is_empty()
            || !scene.dynamic_lights.is_empty()
            || !scene.d3_static.is_empty()
            || !scene.d3_dynamic.is_empty()
            || !scene.d3_overlay.is_empty()
            || !scene.chunks.is_empty()
        {
            return Err(RenderError(
                "overview requires base unlit 2D geometry".into(),
            ));
        }
        if transform.into_col_array().iter().any(|v| !v.is_finite())
            || transform[(0, 0)] <= 0.
            || transform[(1, 1)] <= 0.
            || transform[(0, 1)] != 0.
            || transform[(1, 0)] != 0.
        {
            return Err(RenderError(
                "overview requires a finite fitted axis transform".into(),
            ));
        }
        self.target(size)?;
        let occlusions: Vec<_> = occlusion_regions
            .iter()
            .map(|(r, v)| Occlusion {
                bounds: [r.min.x, r.min.y, r.max.x, r.max.y],
                value: [*v, 0., 0., 0.],
            })
            .collect();
        if occlusions.iter().any(|v| {
            v.bounds
                .iter()
                .chain(v.value.iter())
                .any(|v| !v.is_finite())
        }) {
            return Err(RenderError("nonfinite overview occlusion".into()));
        }
        let occlusion_data = if occlusions.is_empty() {
            vec![Occlusion::zeroed()]
        } else {
            occlusions.clone()
        };
        let occlusion_buffer = self.buffer(
            bytemuck::cast_slice(&occlusion_data),
            wgpu::BufferUsages::STORAGE,
        )?;
        let mut params = Params {
            size: [size.0, size.1, size.0.div_ceil(CELL), 0],
            source: [0; 4],
            page: [0.; 4],
            period: [0., 0., 0., occlusions.len() as f32],
            // Preserve Rasterizer's historical X-scale convention for ambient
            // occlusion under an anisotropic overview transform.
            world: [
                transform[(0, 2)],
                transform[(1, 2)],
                transform[(0, 0)],
                transform[(0, 0)],
            ],
            background: [
                background[0] as u32,
                background[1] as u32,
                background[2] as u32,
                255,
            ],
        };
        let clear_params = self.buffer(bytemuck::bytes_of(&params), wgpu::BufferUsages::UNIFORM)?;
        let clear_bindings = self.bindings(
            clear_params.buffer(),
            self.empty.buffer(),
            self.empty.buffer(),
            occlusion_buffer.buffer(),
            &self.dummy,
        );
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("overview frame"),
                });
        Self::dispatch(&mut encoder, &self.clear, &clear_bindings, size);
        // Raw bind-group/encoder handles do not pin pooled allocations. Keep
        // every batch and source lease until all ordered passes are submitted.
        let mut batch_leases = Vec::new();
        for batch in scene.d2_static.iter().chain(&scene.d2_dynamic) {
            if batch.mode != PrimitiveMode::Triangles {
                return Err(RenderError(
                    "overview contains non-triangle primitives".into(),
                ));
            }
            if batch.vertices.len() != batch.uvs.len() {
                return Err(RenderError("overview position/UV count differs".into()));
            }
            let tile = match batch.source {
                rusterix::PixelSource::StaticTileIndex(i) => assets.tile_list.get(i as usize),
                rusterix::PixelSource::DynamicTileIndex(i) => {
                    scene.dynamic_textures.get(i as usize)
                }
                _ => batch.source.tile_ref_from_tile_list(assets),
            };
            let Some(tile) = tile.filter(|tile| !tile.textures.is_empty()) else {
                continue;
            };
            let mut triangles = Vec::new();
            for &(a, b, c) in &batch.indices {
                if [a, b, c].into_iter().any(|i| i >= batch.vertices.len()) {
                    return Err(RenderError("overview triangle index out of bounds".into()));
                }
                let p = [a, b, c].map(|i| {
                    let v = batch.vertices[i];
                    let v = transform * Vec3::new(v[0], v[1], 1.);
                    [v.x, v.y, batch.uvs[i][0], batch.uvs[i][1]]
                });
                if p.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(RenderError("nonfinite overview triangle".into()));
                }
                let edges = std::array::from_fn(|i| {
                    let a = p[i];
                    let b = p[(i + 1) % 3];
                    [b[1] - a[1], a[0] - b[0], b[0] * a[1] - b[1] * a[0], 0.]
                });
                triangles.push(Triangle { p, edges });
            }
            if triangles.is_empty() {
                continue;
            }
            let bins = bins(
                &triangles,
                size,
                self.context.device.limits().max_storage_buffer_binding_size as usize / 4,
            )?;
            let triangle_buffer = self.buffer(
                bytemuck::cast_slice(&triangles),
                wgpu::BufferUsages::STORAGE,
            )?;
            let bin_buffer =
                self.buffer(bytemuck::cast_slice(&bins), wgpu::BufferUsages::STORAGE)?;
            let base = tile.frame_index(scene.animation_frame);
            let pages = tile.uv_pages.as_ref().filter(|pages| {
                batch.repeat_mode == RepeatMode::RepeatXY && pages.size.iter().all(|n| *n > 0)
            });
            let count = if let Some(pages) =
                pages.filter(|pages| !pages.repeat_period.iter().all(|n| n.is_finite() && *n > 0.))
            {
                pages.size[0]
                    .checked_mul(pages.size[1])
                    .ok_or_else(|| RenderError("overview page count overflow".into()))?
                    as usize
            } else {
                1
            };
            if base >= tile.textures.len() {
                continue;
            }
            let source = self
                .sources
                .prepare(tile, namespace, revision, base, count)?;
            params.size[3] = triangles.len() as u32;
            params.source = [
                match batch.repeat_mode {
                    RepeatMode::ClampXY => 0,
                    RepeatMode::RepeatXY => 1,
                    RepeatMode::RepeatX => 2,
                    RepeatMode::RepeatY => 3,
                },
                0,
                source.layers,
                0,
            ];
            params.page = [0.; 4];
            params.period[0] = 0.;
            params.period[1] = 0.;
            params.period[2] = 0.;
            if let Some(pages) = &tile.uv_pages
                && batch.repeat_mode == RepeatMode::RepeatXY
                && pages.size.iter().all(|v| *v > 0)
            {
                params.source[3] = pages.size[0] as u32;
                params.page = [
                    pages.origin[0] as f32,
                    pages.origin[1] as f32,
                    pages.units_per_page.max(1) as f32,
                    pages.size[1] as f32,
                ];
                if pages.repeat_period.iter().all(|n| n.is_finite() && *n > 0.) {
                    params.period[0] = pages.repeat_period[0];
                    params.period[1] = pages.repeat_period[1];
                }
                params.period[2] = 1.;
            }
            let uniform = self.buffer(bytemuck::bytes_of(&params), wgpu::BufferUsages::UNIFORM)?;
            let bindings = self.bindings(
                uniform.buffer(),
                triangle_buffer.buffer(),
                bin_buffer.buffer(),
                occlusion_buffer.buffer(),
                &source,
            );
            Self::dispatch(&mut encoder, &self.draw, &bindings, size);
            batch_leases.push((source, uniform, triangle_buffer, bin_buffer));
        }
        Self::dispatch(&mut encoder, &self.resolve, &clear_bindings, size);
        self.context.queue.submit([encoder.finish()]);
        Ok(self.target.as_ref().unwrap().image.clone())
    }
}
fn bins(triangles: &[Triangle], size: (u32, u32), limit: usize) -> Result<Vec<u32>> {
    let cols = size.0.div_ceil(CELL);
    let rows = size.1.div_ceil(CELL);
    let count = (u64::from(cols) * u64::from(rows)) as usize;
    if count.checked_add(1).is_none_or(|n| n > limit) {
        return Err(RenderError("overview bins exceed metadata budget".into()));
    }
    let mut budget = count + 1;
    let mut cells = vec![Vec::new(); count];
    for (index, t) in triangles.iter().enumerate() {
        let min = std::array::from_fn::<_, 2, _>(|a| {
            t.p.iter()
                .map(|p| p[a])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(0.) as u32
        });
        let max = std::array::from_fn::<_, 2, _>(|a| {
            t.p.iter()
                .map(|p| p[a])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .max(0.) as u32
        });
        let max = [max[0].min(size.0), max[1].min(size.1)];
        if min[0] >= max[0] || min[1] >= max[1] {
            continue;
        }
        let index =
            u32::try_from(index).map_err(|_| RenderError("too many overview triangles".into()))?;
        let touched = ((max[0] - 1) / CELL - min[0] / CELL + 1) as usize
            * ((max[1] - 1) / CELL - min[1] / CELL + 1) as usize;
        budget = budget
            .checked_add(touched)
            .filter(|n| *n <= limit)
            .ok_or_else(|| RenderError("overview bins exceed metadata budget".into()))?;
        for y in min[1] / CELL..=(max[1] - 1) / CELL {
            for x in min[0] / CELL..=(max[0] - 1) / CELL {
                cells[(y * cols + x) as usize].push(index);
            }
        }
    }
    let mut offsets = vec![0u32; count + 1];
    let mut indices = Vec::new();
    for (index, cell) in cells.into_iter().enumerate() {
        offsets[index] = u32::try_from(count + 1 + indices.len())
            .map_err(|_| RenderError("overview bins exceed addressable metadata".into()))?;
        indices.extend(cell);
    }
    offsets[count] = u32::try_from(count + 1 + indices.len())
        .map_err(|_| RenderError("overview bins exceed addressable metadata".into()))?;
    offsets.extend(indices);
    Ok(offsets)
}
