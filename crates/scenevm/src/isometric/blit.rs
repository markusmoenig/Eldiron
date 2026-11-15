//! Source-asset GPU blits into deferred isometric canvases. CPU work describes
//! instances and uploads immutable sources, never rasterizes viewport pixels.
use super::IsometricCanvas;
use bytemuck::{Pod, Zeroable};
use renderkit::{GpuContext, RenderError, Result};
use std::{num::NonZeroU64, sync::Arc};

fn source_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("iso-source-layout"),
        entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }),
    })
}
/// One immutable source asset on one device. Keep this lease in the application's
/// versioned source cache; moving/animating an instance causes no asset upload.
pub struct GpuSpriteSource {
    bind: wgpu::BindGroup,
    // Bind groups keep raw handles alive; these leases also prevent pool reuse.
    _source_leases: [renderkit::WorkingBufferLease; 2],
    size: (u32, u32),
    surface: bool,
    generation: u64,
    bytes: u64,
}
impl GpuSpriteSource {
    pub fn new(
        context: &GpuContext,
        width: u32,
        height: u32,
        rgba: &[u8],
        surface: Option<&[u8]>,
    ) -> Result<Arc<Self>> {
        let pixels = u64::from(width) * u64::from(height);
        let limit = u64::from(context.device.limits().max_storage_buffer_binding_size);
        if context.is_lost()
            || width == 0
            || height == 0
            || pixels > limit / 8
            || rgba.len() as u64 != pixels * 4
            || surface.is_some_and(|s| s.len() as u64 != pixels * 8)
        {
            return Err(RenderError("invalid isometric sprite source".into()));
        }
        let geometry = surface.unwrap_or(&[0; 8]);
        let make = |data: &[u8]| -> Result<renderkit::WorkingBufferLease> {
            let lease = context.acquire_working_buffer(
                renderkit::WorkingBufferDescriptor::storage(data.len() as u64),
            )?;
            context.queue.write_buffer(lease.buffer(), 0, data);
            Ok(lease)
        };
        let albedo = make(rgba)?;
        let surfaces = make(geometry)?;
        let bytes = (rgba.len() + geometry.len()) as u64;
        context.record_upload(bytes);
        let bind = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-source"),
                layout: &source_layout(&context.device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: albedo.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: surfaces.buffer().as_entire_binding(),
                    },
                ],
            });
        Ok(Arc::new(Self {
            bind,
            _source_leases: [albedo, surfaces],
            size: (width, height),
            surface: surface.is_some(),
            generation: context.generation(),
            bytes,
        }))
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
    pub fn resident_bytes(&self) -> u64 {
        self.bytes
    }
}
/// A source placement in the shared CPU scene plan. The two scales differ:
/// pixel_scale places source texels on screen; world_scale scales source height.
#[derive(Clone, Copy)]
pub struct SpriteBlit<'a> {
    pub source: &'a GpuSpriteSource,
    pub origin: [f32; 2],
    pub pixel_scale: f32,
    pub base_height: f32,
    pub world_scale: f32,
    pub opacity: f32,
    pub bitmap: bool,
    pub prelit: bool,
    pub ground: bool,
    pub anchor_y: f32,
    pub pixels_per_unit: f32,
}
/// Instance-indexed support policy from the shared CPU scene plan. No pixel
/// ownership or destination-color array is uploaded from the CPU.
#[derive(Clone, Copy, Default)]
pub struct SpriteContacts<'a> {
    pub owner: u32,
    pub protected: &'a [bool],
}
/// Reference terrain occlusion uses the instance's foot in the same projection
/// as the GPU-produced ground depth/height, not a CPU ownership image.
#[derive(Clone, Copy)]
pub struct SpriteTerrainClip {
    pub foot_y: f32,
    pub foot_height: f32,
    pub foot_depth: f32,
    pub unit: f32,
}
#[derive(Clone, Copy, Default)]
pub struct SpriteEnvironment<'a> {
    pub contacts: SpriteContacts<'a>,
    pub terrain: Option<SpriteTerrainClip>,
    /// Shared ground layers use flat geometry instead of the source's normals.
    pub flat_surface: bool,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    canvas: [u32; 4],
    screen: [i32; 4],
    image: [u32; 4],
    scale: [f32; 4],
    contact: [u32; 4],
    bitmap: [f32; 4],
    foot: [f32; 4],
    policy: [u32; 4],
}
/// Retained clear/blit pipelines and a bounded-by-instance-count parameter arena.
/// Ordered compute passes provide cross-sprite storage visibility and preserve
/// the reference integer alpha blending without a CPU intermediate.
pub struct IsometricBlitter {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    clear: wgpu::ComputePipeline,
    blit: wgpu::ComputePipeline,
    params: renderkit::WorkingBufferLease,
    capacity: u64,
    stride: u64,
    protection: renderkit::WorkingBufferLease,
    protection_capacity: u64,
}
impl IsometricBlitter {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("isometric GPU device lost".into()));
        }
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iso-blit-canvas-layout"),
                entries: &[0, 1, 2, 3, 4, 5].map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if binding == 0 {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage {
                                read_only: binding == 4,
                            }
                        },
                        has_dynamic_offset: binding == 0,
                        min_binding_size: if binding == 0 {
                            NonZeroU64::new(std::mem::size_of::<Params>() as u64)
                        } else {
                            None
                        },
                    },
                    count: None,
                }),
            });
        let source_layout = source_layout(&context.device);
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("iso-source-blit"),
                source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
            });
        let pipeline = |entry, layouts: &[Option<&wgpu::BindGroupLayout>]| {
            let p = context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: layouts,
                    immediate_size: 0,
                });
            context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&p),
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
        };
        let clear = pipeline("clear", &[Some(&layout)]);
        let blit = pipeline("blit", &[Some(&layout), Some(&source_layout)]);
        let stride = (std::mem::size_of::<Params>() as u64).div_ceil(u64::from(
            context.device.limits().min_uniform_buffer_offset_alignment,
        )) * u64::from(context.device.limits().min_uniform_buffer_offset_alignment);
        let params = Self::arena(&context, stride)?;
        let protection = Self::mask_arena(&context, 4)?;
        Ok(Self {
            protection,
            protection_capacity: 4,
            context,
            layout,
            clear,
            blit,
            params,
            capacity: stride,
            stride,
        })
    }
    fn arena(context: &GpuContext, size: u64) -> Result<renderkit::WorkingBufferLease> {
        context.acquire_working_buffer(renderkit::WorkingBufferDescriptor {
            bytes: size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
    fn mask_arena(context: &GpuContext, size: u64) -> Result<renderkit::WorkingBufferLease> {
        context.acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(size))
    }
    pub fn render(
        &mut self,
        canvas: &IsometricCanvas,
        background: [u8; 4],
        sprites: &[SpriteBlit<'_>],
    ) -> Result<()> {
        self.render_impl(canvas, background, sprites, None, true)
    }
    pub fn render_with_contacts(
        &mut self,
        canvas: &IsometricCanvas,
        background: [u8; 4],
        sprites: &[SpriteBlit<'_>],
        contacts: &[SpriteContacts<'_>],
    ) -> Result<()> {
        if contacts.len() != sprites.len() {
            return Err(RenderError(
                "isometric contact/instance counts do not match".into(),
            ));
        }
        let environments = contacts
            .iter()
            .map(|contacts| SpriteEnvironment {
                contacts: *contacts,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        self.render_impl(canvas, background, sprites, Some(&environments), true)
    }
    /// Compose source sprites over an already GPU-produced floor/terrain canvas.
    pub fn append(&mut self, canvas: &IsometricCanvas, sprites: &[SpriteBlit<'_>]) -> Result<()> {
        self.render_impl(canvas, [0; 4], sprites, None, false)
    }
    pub fn append_with_contacts(
        &mut self,
        canvas: &IsometricCanvas,
        sprites: &[SpriteBlit<'_>],
        contacts: &[SpriteContacts<'_>],
    ) -> Result<()> {
        if contacts.len() != sprites.len() {
            return Err(RenderError(
                "isometric contact/instance counts do not match".into(),
            ));
        }
        let environments = contacts
            .iter()
            .map(|contacts| SpriteEnvironment {
                contacts: *contacts,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        self.render_impl(canvas, [0; 4], sprites, Some(&environments), false)
    }
    pub fn append_with_environment(
        &mut self,
        canvas: &IsometricCanvas,
        sprites: &[SpriteBlit<'_>],
        environments: &[SpriteEnvironment<'_>],
    ) -> Result<()> {
        if environments.len() != sprites.len() {
            return Err(RenderError(
                "isometric environment/instance counts do not match".into(),
            ));
        }
        self.render_impl(canvas, [0; 4], sprites, Some(environments), false)
    }
    fn render_impl(
        &mut self,
        canvas: &IsometricCanvas,
        background: [u8; 4],
        sprites: &[SpriteBlit<'_>],
        environments: Option<&[SpriteEnvironment<'_>]>,
        clear: bool,
    ) -> Result<()> {
        if self.context.is_lost() || canvas.device_generation() != self.context.generation() {
            return Err(RenderError(
                "isometric blit canvas belongs to a lost or different GPU domain".into(),
            ));
        }
        if !clear && sprites.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(sprites.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(self.stride))
            .filter(|n| {
                *n <= self.context.device.limits().max_buffer_size && *n <= u64::from(u32::MAX)
            })
            .ok_or_else(|| RenderError("isometric instance arena exceeds device limits".into()))?;
        let mask_words = environments
            .map_or(Some(0u64), |c| {
                c.iter().try_fold(0u64, |n, c| {
                    n.checked_add(c.contacts.protected.len() as u64)
                })
            })
            .filter(|n| {
                n.max(&1).saturating_mul(4)
                    <= u64::from(self.context.device.limits().max_storage_buffer_binding_size)
            })
            .ok_or_else(|| {
                RenderError("isometric contact masks exceed device storage limits".into())
            })?;
        let mut masks = Vec::<u32>::with_capacity(mask_words.max(1) as usize);
        let (width, height) = canvas.size();
        let base = Params {
            canvas: [
                width,
                height,
                u32::from_le_bytes(background),
                f32::NEG_INFINITY.to_bits(),
            ],
            ..Zeroable::zeroed()
        };
        let mut params = vec![0u8; count as usize];
        let mut put = |i: usize, p: &Params| {
            let start = i * self.stride as usize;
            params[start..start + std::mem::size_of::<Params>()]
                .copy_from_slice(bytemuck::bytes_of(p));
        };
        put(0, &base);
        for (i, s) in sprites.iter().enumerate() {
            if s.source.generation != self.context.generation()
                || !s.origin.into_iter().all(f32::is_finite)
                || ![
                    s.pixel_scale,
                    s.base_height,
                    s.world_scale,
                    s.opacity,
                    s.anchor_y,
                    s.pixels_per_unit,
                ]
                .into_iter()
                .all(f32::is_finite)
                || s.pixel_scale <= 0.
                || !(0. ..=1.).contains(&s.opacity)
            {
                return Err(RenderError(
                    "invalid isometric sprite placement or GPU domain".into(),
                ));
            }
            let environment =
                environments.map_or(SpriteEnvironment::default(), |records| records[i]);
            if environment.terrain.is_some_and(|t| {
                ![t.foot_y, t.foot_height, t.foot_depth, t.unit]
                    .into_iter()
                    .all(f32::is_finite)
                    || t.unit <= 0.
            }) {
                return Err(RenderError(
                    "invalid isometric terrain clip metadata".into(),
                ));
            }
            let foot = environment
                .terrain
                .map_or([0.; 4], |t| [t.foot_y, t.foot_height, t.foot_depth, t.unit]);
            let policy = [
                u32::from(environment.terrain.is_some())
                    | (u32::from(environment.flat_surface) << 1),
                0,
                0,
                0,
            ];
            let contact = environments.map_or([0; 4], |records| {
                let c = &records[i].contacts;
                let offset = masks.len() as u32;
                masks.extend(c.protected.iter().map(|v| u32::from(*v)));
                [c.owner, offset, c.protected.len() as u32, 0]
            });
            let (w, h) = s.source.size;
            let at = s.origin.map(|v| v.round() as i32);
            let size = [w, h].map(|v| (v as f32 * s.pixel_scale).round() as i32);
            put(
                i + 1,
                &Params {
                    screen: [
                        at[0],
                        at[1],
                        at[0].saturating_add(size[0]),
                        at[1].saturating_add(size[1]),
                    ],
                    image: [
                        w,
                        h,
                        u32::from(s.bitmap)
                            | (u32::from(s.prelit) << 1)
                            | (u32::from(s.ground) << 2),
                        u32::from(s.source.surface),
                    ],
                    scale: [s.pixel_scale, s.base_height, s.world_scale, s.opacity],
                    contact,
                    bitmap: [s.anchor_y, s.pixels_per_unit, 0., 0.],
                    foot,
                    policy,
                    ..base
                },
            );
        }
        if masks.is_empty() {
            masks.push(0);
        }
        let mask_bytes = bytemuck::cast_slice(&masks);
        if mask_bytes.len() as u64 > self.protection_capacity {
            self.protection = Self::mask_arena(&self.context, mask_bytes.len() as u64)?;
            self.protection_capacity = mask_bytes.len() as u64;
        }
        self.context
            .queue
            .write_buffer(self.protection.buffer(), 0, mask_bytes);
        self.context.record_upload(mask_bytes.len() as u64);
        if count > self.capacity {
            self.params = Self::arena(&self.context, count)?;
            self.capacity = count;
        }
        self.context
            .queue
            .write_buffer(self.params.buffer(), 0, &params);
        self.context.record_upload(params.len() as u64);
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-blit-canvas"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: self.params.buffer(),
                            offset: 0,
                            size: NonZeroU64::new(std::mem::size_of::<Params>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: canvas.albedo().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: canvas.surfaces().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: canvas.owners().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.protection.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: canvas.terrain().as_entire_binding(),
                    },
                ],
            });
        let mut encoder = renderkit::OrderedCompute::new(&self.context, "iso-source-blits");
        if clear {
            let mut p = encoder.begin_pass();
            p.set_pipeline(&self.clear);
            p.set_bind_group(0, &bind, &[0]);
            p.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        for (i, s) in sprites.iter().enumerate() {
            let (w, h) = s.source.size;
            let at = s.origin.map(|v| v.round() as i32);
            let size = [w, h].map(|v| (v as f32 * s.pixel_scale).round() as i32);
            let end = [
                at[0].saturating_add(size[0]).min(width as i32),
                at[1].saturating_add(size[1]).min(height as i32),
            ];
            let extent = [end[0] - at[0].max(0), end[1] - at[1].max(0)];
            if extent.into_iter().any(|v| v <= 0) {
                continue;
            }
            let mut p = encoder.begin_pass();
            p.set_pipeline(&self.blit);
            p.set_bind_group(0, &bind, &[((i as u64 + 1) * self.stride) as u32]);
            p.set_bind_group(1, &s.source.bind, &[]);
            p.dispatch_workgroups(
                (extent[0] as u32).div_ceil(8),
                (extent[1] as u32).div_ceil(8),
                1,
            );
        }
        encoder.finish();
        Ok(())
    }
}
