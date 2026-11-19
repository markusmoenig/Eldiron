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
mod batch;
mod bindings;
#[cfg(test)]
mod tests;

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

pub(super) fn terrain_normal(vertices: [Vec3<f32>; 3]) -> Vec3<f32> {
    let normal = (vertices[1] - vertices[0]).cross(vertices[2] - vertices[0]);
    if normal.magnitude_squared() <= 0. {
        return Vec3::unit_y();
    }
    let normal = normal.normalized();
    if normal.y < 0. { -normal } else { normal }
}
pub struct IsometricSurfaceRenderer {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    source_layout: wgpu::BindGroupLayout,
    paint_layout: wgpu::BindGroupLayout,
    empty_paint: wgpu::BindGroup,
    _empty_paint_storage: renderkit::WorkingBufferLease,
    pipeline: wgpu::ComputePipeline,
    batch_pipeline: wgpu::ComputePipeline,
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
                    batch::shader_source(false)
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
        let batch_shader = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iso-batched-gpu-surfaces"),
            source: wgpu::ShaderSource::Wgsl(batch::shader_source(true).into()),
        });
        let batch_pipeline = context.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("iso-batched-surface-raster"), layout: Some(&pipeline_layout),
            module: &batch_shader, entry_point: Some("raster"),
            compilation_options: Default::default(), cache: None,
        });
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
            batch_pipeline,
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
    /// overlap dependencies remain ordered; disjoint triangles may share a
    /// dispatch. Lighting/capture remain separate.
    pub fn append(
        &mut self,
        canvas: &IsometricCanvas,
        projection: Projection,
        triangles: &[SurfaceTriangle<'_>],
    ) -> Result<()> {
        self.append_impl(canvas, projection, triangles, true)
    }
    fn append_impl(
        &mut self, canvas: &IsometricCanvas, projection: Projection,
        triangles: &[SurfaceTriangle<'_>], allow_batch: bool,
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
        let limits = self.context.device.limits();
        let mask_limit = u64::from(limits.max_storage_buffer_binding_size)
            .min(limits.max_buffer_size) / 4;
        let mut masks = Vec::<u32>::new();
        let mut mask_ranges = FxHashMap::default();
        let mut records = Vec::<Params>::with_capacity(triangles.len());
        let mut regions = Vec::with_capacity(triangles.len());
        let (width, height) = canvas.size();
        for t in triangles {
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
                normal = terrain_normal(t.vertices.map(Vec3::from));
                if !normal.into_array().into_iter().all(f32::is_finite) {
                    return Err(RenderError("non-finite isometric surface normal".into()));
                }
            }
            let protected = t.contacts.protected;
            let (offset, len) = if region[0] >= region[2] || region[1] >= region[3] || protected.is_empty() {
                (0, 0)
            } else {
                // Every triangle of a floor borrows the same protection mask.
                // Share its GPU range for this call and omit its false suffix.
                let key = (protected.as_ptr(), protected.len());
                match mask_ranges.entry(key) {
                    std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        let len = protected.iter().rposition(|v| *v).map_or(0, |i| i + 1);
                        if masks.len() as u64 + len as u64 > mask_limit {
                            return Err(RenderError("isometric surface masks exceed device limits".into()));
                        }
                        let range = (masks.len() as u32, len as u32);
                        masks.extend(protected[..len].iter().map(|v| u32::from(*v)));
                        *entry.insert(range)
                    }
                }
            };
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
                    len,
                    u32::from(t.secondary.is_some()) | (u32::from(t.paint.is_some()) << 1),
                ],
                cell,
            };
            records.push(params);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some() {
            eprintln!("Isometric surface work: triangles={} terrain={} record_bytes={} legacy_arena_bytes={size} mask_words={}; registry={:?}",
                triangles.len(), triangles.iter().filter(|t| t.kind == SurfaceKind::Terrain).count(), triangles.len() * std::mem::size_of::<Params>(), masks.len(), self.context.instance.generate_report());
        }
        if masks.is_empty() {
            masks.push(0);
        }
        let waves = super::surface_waves::floor_waves(triangles, &regions)
            .unwrap_or_else(|| super::surface_order::waves(&regions, canvas.size()));
        if allow_batch && batch::append(self, canvas, triangles, &records, &masks, &regions, &waves)? {
            return Ok(());
        }
        // Only the individual fallback needs dynamically aligned uniform slots.
        // Normal batched work keeps tightly packed typed records on the CPU.
        let mut bytes = vec![0u8; size as usize];
        for (i, params) in records.iter().enumerate() {
            let start = i * self.stride as usize;
            bytes[start..start + std::mem::size_of::<Params>()]
                .copy_from_slice(bytemuck::bytes_of(params));
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
        let bind = bindings::canvas(self, canvas);
        let (pairs, paints) = bindings::sources(self, triangles);
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
