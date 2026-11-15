//! Explicit synchronous authoring capture. Staging belongs to this request,
//! including failure/cancellation, and never remains resident on the scene.
use super::*;

struct Staging {
    buffer: wgpu::Buffer,
    lease: Option<renderkit::CaptureLease>,
}
impl Staging {
    fn new(device: &wgpu::Device, context: Option<&GpuContext>, bytes: u64) -> Option<Self> {
        // Charge padded allocation bytes before creating storage. The submitted
        // callback keeps that charge if an error retires the consumer early.
        let lease = match context {
            Some(context) => Some(context.reserve_capture(bytes).ok()?),
            None => None, // Explicit noninjected reference owners stay isolated.
        };
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("requested-paint-coordinate-capture"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self { buffer, lease })
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        // destroy also cancels a pending map and is safe after device loss.
        self.buffer.destroy();
    }
}

impl VM {
    pub fn paint_surface_buffer_gpu_with(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fb_w: u32,
        fb_h: u32,
        include_dynamic: bool,
        dynamic_only: bool,
    ) -> Option<PaintSurfaceBuffer> {
        if fb_w == 0 || fb_h == 0 {
            return Some(PaintSurfaceBuffer::new(fb_w, fb_h));
        }
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = fb_w.checked_mul(16)?.checked_add(align - 1)? / align * align;
        let plane_size = u64::from(padded).checked_mul(u64::from(fb_h))?;
        let needed = plane_size
            .checked_mul(2)
            .filter(|bytes| *bytes <= device.limits().max_buffer_size)?;
        self.encode_surface_gpu(device, queue, fb_w, fb_h, include_dynamic, dynamic_only)?;
        if self.gpu.as_ref()?.paint_surface_vertex_count == 0 {
            return Some(PaintSurfaceBuffer::new(fb_w, fb_h));
        }
        let staging = Staging::new(device, self.target_context.as_deref(), needed)?;
        let readback = &staging.buffer;
        let geo_ids = if include_dynamic {
            self.cached_tri_geo_ids.as_slice()
        } else {
            self.cached_static_tri_geo_ids.as_slice()
        };
        let paint_group_ids = &self.cached_static_paint_group_ids;
        let face_normals: Vec<[f32; 3]> = {
            let (source_vertices, source_indices) = if include_dynamic {
                (self.cached_v3.as_slice(), self.cached_i3.as_slice())
            } else {
                (
                    self.cached_static_v3.as_slice(),
                    self.cached_static_i3.as_slice(),
                )
            };
            source_indices
                .chunks_exact(3)
                .map(|triangle| {
                    let Some(a) = source_vertices.get(triangle[0] as usize) else {
                        return [0.0, 1.0, 0.0];
                    };
                    let Some(b) = source_vertices.get(triangle[1] as usize) else {
                        return [0.0, 1.0, 0.0];
                    };
                    let Some(c) = source_vertices.get(triangle[2] as usize) else {
                        return [0.0, 1.0, 0.0];
                    };
                    let a = Vec3::<f32>::from(a.pos);
                    let b = Vec3::<f32>::from(b.pos);
                    let c = Vec3::<f32>::from(c.pos);
                    let normal = (b - a)
                        .cross(c - a)
                        .try_normalized()
                        .unwrap_or_else(Vec3::unit_y);
                    [normal.x, normal.y, normal.z]
                })
                .collect()
        };
        let c = self.camera3d;
        let g = self.gpu.as_ref()?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-paint-surface-encoder"),
        });
        let copy_layout = |offset| wgpu::TexelCopyBufferLayout {
            offset,
            bytes_per_row: Some(padded),
            rows_per_image: Some(fb_h),
        };
        let extent = wgpu::Extent3d {
            width: fb_w,
            height: fb_h,
            depth_or_array_layers: 1,
        };
        for (texture, offset) in [
            (g.paint_surface_world_tex.as_ref()?, 0),
            (g.paint_surface_meta_tex.as_ref()?, plane_size),
        ] {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: readback,
                    layout: copy_layout(offset),
                },
                extent,
            );
        }
        queue.submit(Some(encoder.finish()));
        if let Some(lease) = &staging.lease {
            lease.retain_for_submission(queue);
        }
        if let Some(context) = &self.target_context {
            context.record_capture(u64::from(fb_w) * u64::from(fb_h) * 32);
        }

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .is_err()
            || !matches!(rx.try_recv(), Ok(Ok(())))
        {
            return None;
        }
        let mapped = slice.get_mapped_range();
        let read_f32 = |offset: usize| -> f32 {
            let bytes: [u8; 4] = mapped[offset..offset + 4].try_into().unwrap_or([0; 4]);
            f32::from_ne_bytes(bytes)
        };
        let mut buffer = PaintSurfaceBuffer::new(fb_w, fb_h);
        for y in 0..fb_h as usize {
            for x in 0..fb_w as usize {
                let base = y * padded as usize + x * 16;
                let world_base = base;
                let meta_base = plane_size as usize + base;
                let valid = read_f32(meta_base + 12) > 0.5;
                if !valid {
                    continue;
                }
                let face_id_f = read_f32(meta_base + 8);
                if !face_id_f.is_finite() || face_id_f < 0.0 {
                    continue;
                }
                let face_id = face_id_f.round() as usize;
                let Some(source_geo_id) = geo_ids.get(face_id).copied() else {
                    continue;
                };
                let geo_id = paint_group_ids
                    .get(&source_geo_id)
                    .copied()
                    .unwrap_or(source_geo_id);
                let index = y * fb_w as usize + x;
                let world = [
                    read_f32(world_base),
                    read_f32(world_base + 4),
                    read_f32(world_base + 8),
                ];
                let normal = face_normals
                    .get(face_id)
                    .copied()
                    .unwrap_or([0.0, 1.0, 0.0]);
                buffer.pixels[index] = crate::core::PaintSurfacePixel {
                    valid: true,
                    geo_id,
                    paint_geo: (if include_dynamic {
                        &self.cached_i3
                    } else {
                        &self.cached_static_i3
                    })
                    .get(face_id * 3)
                    .and_then(|vertex_index| {
                        if include_dynamic {
                            self.cached_v3.get(*vertex_index as usize)
                        } else {
                            self.cached_static_v3.get(*vertex_index as usize)
                        }
                    })
                    .map(|vertex| vertex.paint_geo)
                    .unwrap_or_else(|| Self::paint_geo_id_packed(source_geo_id)),
                    face_id: face_id as u32,
                    depth: (Vec3::from(world) - c.pos).dot(c.forward),
                    world,
                    normal,
                    uv: [read_f32(meta_base), read_f32(meta_base + 4)],
                };
            }
        }
        drop(mapped);
        readback.unmap();
        Some(buffer)
    }
}
