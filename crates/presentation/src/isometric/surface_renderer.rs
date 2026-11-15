//! Ordered GPU floor/terrain rasterization into the shared deferred canvas.
//! CPU preparation emits geometry/source/mask metadata, never viewport pixels.
use super::{GpuMaterialSource, GpuTerrainPaintSource};
use bytemuck::{Pod, Zeroable};
use renderkit::{GpuContext, RenderError, Result, wgpu};
use rustc_hash::FxHashMap;
use rusterix::isometric::Projection;
use scenevm::{
    isometric::{IsometricCanvas, SpriteContacts},
    prelude::{Vec2, Vec3},
};
use std::{num::NonZeroU64, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Floor,
    Terrain,
}
/// Sources are immutable asset leases. Cell clipping lets arbitrary authored
/// floor polygons share the same cell overrides as the reference rasterizer.
pub struct SurfaceTriangle<'a> {
    pub vertices: [[f32; 3]; 3],
    pub source: &'a GpuMaterialSource,
    pub secondary: Option<(&'a GpuMaterialSource, [f32; 4])>,
    pub paint: Option<&'a GpuTerrainPaintSource>,
    pub kind: SurfaceKind,
    pub repeat: bool,
    pub uv_bounds: [f32; 4],
    pub clip_cell: Option<[i32; 2]>,
    pub contacts: SpriteContacts<'a>,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    projection: [f32; 4],
    canvas: [u32; 4],
    region: [i32; 4],
    vertices: [[f32; 4]; 3],
    normal: [f32; 4],
    uv_bounds: [f32; 4],
    blend: [f32; 4],
    contact: [u32; 4],
    cell: [i32; 4],
}
pub struct IsometricSurfaceRenderer {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    source_layout: wgpu::BindGroupLayout,
    paint_layout: wgpu::BindGroupLayout,
    empty_paint: wgpu::BindGroup,
    _empty_paint_storage: renderkit::WorkingBufferLease,
    pipeline: wgpu::ComputePipeline,
    params: renderkit::WorkingBufferLease,
    capacity: u64,
    stride: u64,
    masks: renderkit::WorkingBufferLease,
    mask_capacity: u64,
}
impl IsometricSurfaceRenderer {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("isometric surface GPU device lost".into()));
        }
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iso-surface-canvas-layout"),
                entries: &[0, 1, 2, 3, 4, 5].map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if binding == 0 {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage {
                                read_only: binding == 5,
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
        let source_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("iso-surface-material-layout"),
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
                });
        let paint_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("iso-paint-source-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let empty = Self::buffer(&context, 4, false)?;
        context.queue.write_buffer(empty.buffer(), 0, &[0; 4]);
        context.record_upload(4);
        let empty_paint = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-empty-paint"),
                layout: &paint_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: empty.buffer().as_entire_binding(),
                }],
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("iso-gpu-surfaces"),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{}\n{}\n{}",
                        include_str!("surface.wgsl"),
                        include_str!("material_sample.wgsl"),
                        include_str!("paint_sample.wgsl")
                    )
                    .into(),
                ),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout), Some(&source_layout), Some(&paint_layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("iso-surface-raster"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("raster"),
                compilation_options: Default::default(),
                cache: None,
            });
        let alignment = u64::from(context.device.limits().min_uniform_buffer_offset_alignment);
        let stride = (std::mem::size_of::<Params>() as u64).div_ceil(alignment) * alignment;
        let params = Self::buffer(&context, stride, true)?;
        let masks = Self::buffer(&context, 4, false)?;
        Ok(Self {
            context,
            layout,
            source_layout,
            paint_layout,
            empty_paint,
            _empty_paint_storage: empty,
            pipeline,
            params,
            capacity: stride,
            stride,
            masks,
            mask_capacity: 4,
        })
    }
    fn buffer(
        context: &GpuContext,
        size: u64,
        uniform: bool,
    ) -> Result<renderkit::WorkingBufferLease> {
        context.acquire_working_buffer(renderkit::WorkingBufferDescriptor {
            bytes: size,
            usage: wgpu::BufferUsages::COPY_DST
                | if uniform {
                    wgpu::BufferUsages::UNIFORM
                } else {
                    wgpu::BufferUsages::STORAGE
                },
        })
    }
    /// Append geometry to a canvas cleared on the GPU by IsometricBlitter. Each
    /// dispatch sees earlier floor/terrain writes; lighting/capture remain separate.
    pub fn append(
        &mut self,
        canvas: &IsometricCanvas,
        projection: Projection,
        triangles: &[SurfaceTriangle<'_>],
    ) -> Result<()> {
        if self.context.is_lost() || canvas.device_generation() != self.context.generation() {
            return Err(RenderError(
                "isometric surface canvas belongs to a lost or different GPU domain".into(),
            ));
        }
        if ![projection.origin.x, projection.origin.y, projection.unit]
            .into_iter()
            .all(f32::is_finite)
            || projection.unit <= 0.
        {
            return Err(RenderError("invalid isometric surface projection".into()));
        }
        if triangles.is_empty() {
            return Ok(());
        }
        let size = (triangles.len() as u64)
            .checked_mul(self.stride)
            .filter(|n| {
                *n <= self.context.device.limits().max_buffer_size && *n <= u64::from(u32::MAX)
            })
            .ok_or_else(|| {
                RenderError("isometric surface parameter arena exceeds device limits".into())
            })?;
        let mask_count = triangles
            .iter()
            .try_fold(0u64, |n, t| {
                n.checked_add(t.contacts.protected.len() as u64)
            })
            .filter(|n| {
                n.max(&1).saturating_mul(4)
                    <= u64::from(self.context.device.limits().max_storage_buffer_binding_size)
            })
            .ok_or_else(|| RenderError("isometric surface masks exceed device limits".into()))?;
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some() {
            eprintln!("Isometric surface work: triangles={} terrain={} arena_bytes={size} mask_words={mask_count}; registry={:?}",
                triangles.len(), triangles.iter().filter(|t| t.kind == SurfaceKind::Terrain).count(), self.context.instance.generate_report());
        }
        let mut masks = Vec::<u32>::with_capacity(mask_count.max(1) as usize);
        let mut bytes = vec![0u8; size as usize];
        let mut regions = Vec::with_capacity(triangles.len());
        let (width, height) = canvas.size();
        for (i, t) in triangles.iter().enumerate() {
            if t.paint
                .is_some_and(|p| p.device_generation() != self.context.generation())
                || t.source.device_generation() != self.context.generation()
                || t.secondary
                    .is_some_and(|(s, _)| s.device_generation() != self.context.generation())
                || !t.vertices.into_iter().flatten().all(f32::is_finite)
                || !t.uv_bounds.into_iter().all(f32::is_finite)
                || t.secondary
                    .is_some_and(|(_, w)| !w.into_iter().all(f32::is_finite))
            {
                return Err(RenderError(
                    "invalid isometric surface source, vertices or GPU domain".into(),
                ));
            }
            let projected = t
                .vertices
                .map(|v| projection.screen(Vec2::new(v[0], v[2]), v[1]));
            if projected
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
            {
                return Err(RenderError("non-finite isometric projected surface".into()));
            }
            let ab = projected[1] - projected[0];
            let ac = projected[2] - projected[0];
            if !(ab.x * ac.y - ab.y * ac.x).is_finite() {
                return Err(RenderError(
                    "non-finite isometric projected triangle area".into(),
                ));
            }
            let min = projected
                .iter()
                .fold(Vec2::broadcast(f32::INFINITY), |a, b| a.map2(*b, f32::min));
            let max = projected
                .iter()
                .fold(Vec2::broadcast(f32::NEG_INFINITY), |a, b| {
                    a.map2(*b, f32::max)
                });
            let region = [
                (min.x.floor() as i32).clamp(0, width as i32),
                (min.y.floor() as i32).clamp(0, height as i32),
                (max.x.ceil() as i32).clamp(0, width as i32),
                (max.y.ceil() as i32).clamp(0, height as i32),
            ];
            let region = super::surface_region::clip_floor(region, t, projection);
            regions.push(region);
            let mut normal = Vec3::unit_y();
            if t.kind == SurfaceKind::Terrain {
                normal = (Vec3::from(t.vertices[1]) - Vec3::from(t.vertices[0]))
                    .cross(Vec3::from(t.vertices[2]) - Vec3::from(t.vertices[0]));
                if normal.magnitude_squared() > 0. {
                    normal = normal.normalized();
                    if normal.y < 0. {
                        normal = -normal;
                    }
                } else {
                    normal = Vec3::unit_y();
                }
                if !normal.into_array().into_iter().all(f32::is_finite) {
                    return Err(RenderError("non-finite isometric surface normal".into()));
                }
            }
            let offset = masks.len() as u32;
            masks.extend(t.contacts.protected.iter().map(|v| u32::from(*v)));
            let cell = t.clip_cell.map_or([0; 4], |p| [p[0], p[1], 1, 0]);
            let params = Params {
                projection: [
                    projection.origin.x,
                    projection.origin.y,
                    projection.unit,
                    0.,
                ],
                canvas: [
                    width,
                    height,
                    u32::from(t.kind == SurfaceKind::Terrain),
                    u32::from(t.repeat),
                ],
                region,
                vertices: t.vertices.map(|p| [p[0], p[1], p[2], 0.]),
                normal: [normal.x, normal.y, normal.z, 0.],
                uv_bounds: t.uv_bounds,
                blend: t.secondary.map_or([0.; 4], |(_, weights)| weights),
                contact: [
                    t.contacts.owner,
                    offset,
                    t.contacts.protected.len() as u32,
                    u32::from(t.secondary.is_some()) | (u32::from(t.paint.is_some()) << 1),
                ],
                cell,
            };
            let start = i * self.stride as usize;
            bytes[start..start + std::mem::size_of::<Params>()]
                .copy_from_slice(bytemuck::bytes_of(&params));
        }
        if masks.is_empty() {
            masks.push(0);
        }
        let mask_bytes = bytemuck::cast_slice(&masks);
        if size > self.capacity {
            self.params = Self::buffer(&self.context, size, true)?;
            self.capacity = size;
        }
        if mask_bytes.len() as u64 > self.mask_capacity {
            self.masks = Self::buffer(&self.context, mask_bytes.len() as u64, false)?;
            self.mask_capacity = mask_bytes.len() as u64;
        }
        self.context
            .queue
            .write_buffer(self.params.buffer(), 0, &bytes);
        self.context.record_upload(bytes.len() as u64);
        self.context
            .queue
            .write_buffer(self.masks.buffer(), 0, mask_bytes);
        self.context.record_upload(mask_bytes.len() as u64);
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-surface-canvas"),
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
                        resource: canvas.terrain().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.masks.buffer().as_entire_binding(),
                    },
                ],
            });
        let mut pairs = FxHashMap::default();
        for t in triangles {
            let other = t.secondary.map_or(t.source, |(source, _)| source);
            let key = (t.source as *const _ as usize, other as *const _ as usize);
            pairs.entry(key).or_insert_with(|| {
                self.context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("iso-surface-sources"),
                        layout: &self.source_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: t.source.buffer().as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: other.buffer().as_entire_binding(),
                            },
                        ],
                    })
            });
        }
        let mut paints = FxHashMap::default();
        for source in triangles.iter().filter_map(|t| t.paint) {
            paints
                .entry(source as *const _ as usize)
                .or_insert_with(|| {
                    self.context
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("iso-surface-paint"),
                            layout: &self.paint_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: source.buffer().as_entire_binding(),
                            }],
                        })
                });
        }
        let mut encoder = renderkit::OrderedCompute::new(&self.context, "iso-surface-raster");
        let dispatch = |pass: &mut wgpu::ComputePass<'_>, i: usize| {
            let t = &triangles[i];
            let region = regions[i];
            let other = t.secondary.map_or(t.source, |(source, _)| source);
            let key = (t.source as *const _ as usize, other as *const _ as usize);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[(i as u64 * self.stride) as u32]);
            pass.set_bind_group(1, &pairs[&key], &[]);
            pass.set_bind_group(
                2,
                t.paint.map_or(&self.empty_paint, |source| {
                    &paints[&(source as *const _ as usize)]
                }),
                &[],
            );
            pass.dispatch_workgroups(
                ((region[2] - region[0]) as u32).div_ceil(8),
                ((region[3] - region[1]) as u32).div_ceil(8),
                1,
            );
        };
        let waves = super::surface_waves::floor_waves(triangles, &regions)
            .unwrap_or_else(|| super::surface_order::waves(&regions, canvas.size()));
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some() {
            eprintln!("Isometric surface waves: {} passes, {} active dispatches", waves.len(), waves.iter().map(Vec::len).sum::<usize>());
        }
        for wave in waves {
            let mut pass = encoder.begin_pass();
            for index in wave { dispatch(&mut pass, index); }
        }
        encoder.finish();
        Ok(())
    }
}
