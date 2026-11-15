//! Shared GPU surface pass; CPU capture is a separate optional consumer.
use super::*;
use crate::surface_output::{SurfaceOutput, SurfaceSelection};
use renderkit::{GpuContext, RenderError, Result};
mod capture;
mod targets;

impl VM {
    pub(super) fn encode_surface_gpu(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fb_w: u32,
        fb_h: u32,
        include_dynamic: bool,
        dynamic_only: bool,
    ) -> Option<()> {
        self.ensure_paint_surface_pipeline(device).ok()?;
        self.ensure_paint_surface_targets(device, fb_w, fb_h).ok()?;

        let vertices = self.paint_surface_vertices(include_dynamic, dynamic_only);
        let count = u32::try_from(vertices.len()).ok()?;
        let context = self.target_context.as_ref();
        let g = self.gpu.as_mut()?;
        if !vertices.is_empty() {
            let vertex_bytes = std::mem::size_of_val(vertices.as_slice()) as u64;
            if g.paint_surface_vbuf.is_none() || g.paint_surface_vbuf_capacity < vertex_bytes {
                if let Some(context) = context {
                    let lease = context
                        .acquire_working_buffer(renderkit::WorkingBufferDescriptor {
                            bytes: vertex_bytes,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        })
                        .ok()?;
                    g.paint_surface_vbuf = Some(lease.buffer().clone());
                    g.paint_surface_vbuf_lease = Some(lease);
                } else {
                    // Explicit reference owners have no injected device pool.
                    g.paint_surface_vbuf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("reference-paint-surface-verts"),
                        size: vertex_bytes,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }));
                    g.paint_surface_vbuf_lease = None;
                }
                g.paint_surface_vbuf_capacity = vertex_bytes;
            }
            queue.write_buffer(
                g.paint_surface_vbuf.as_ref()?,
                0,
                bytemuck::cast_slice(&vertices),
            );
        }
        g.paint_surface_vertex_count = count;

        let c = self.camera3d;
        let u = PaintSurfaceUniforms {
            cam_pos: [c.pos.x, c.pos.y, c.pos.z, 0.0],
            cam_fwd: [c.forward.x, c.forward.y, c.forward.z, 0.0],
            cam_right: [c.right.x, c.right.y, c.right.z, 0.0],
            cam_up: [c.up.x, c.up.y, c.up.z, 0.0],
            fb_size: [fb_w as f32, fb_h as f32],
            cam_vfov_deg: c.vfov_deg,
            cam_ortho_half_h: c.ortho_half_h,
            cam_near: c.near,
            cam_far: c.far,
            cam_kind: match c.kind {
                CameraKind::OrthoIso => 0,
                CameraKind::OrbitPersp => 1,
                CameraKind::FirstPersonPersp => 2,
            },
            _pad: [0, 0, 0, 0, 0],
        };
        queue.write_buffer(g.u_paint_surface_buf.as_ref()?, 0, bytemuck::bytes_of(&u));
        if g.u_paint_surface_bg.is_none() {
            g.u_paint_surface_bg = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vm-paint-surface-bg"),
                layout: g.u_paint_surface_bgl.as_ref()?,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: g.u_paint_surface_buf.as_ref()?.as_entire_binding(),
                }],
            }));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vm-paint-surface-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vm-paint-surface-pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: g.paint_surface_world_view.as_ref()?,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: g.paint_surface_meta_view.as_ref()?,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: g.paint_surface_depth_view.as_ref()?,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(g.raster3d_paint_surface_pipeline.as_ref()?);
            pass.set_bind_group(0, g.u_paint_surface_bg.as_ref()?, &[]);
            if g.paint_surface_vertex_count > 0 {
                pass.set_vertex_buffer(0, g.paint_surface_vbuf.as_ref()?.slice(..));
                pass.draw(0..g.paint_surface_vertex_count, 0..1);
            }
        }

        queue.submit(Some(encoder.finish()));
        Some(())
    }

    pub(crate) fn surface_output(
        &mut self,
        context: &GpuContext,
        size: (u32, u32),
        selection: SurfaceSelection,
        revision: u64,
    ) -> Result<SurfaceOutput> {
        if context.is_lost() || self.gpu_domain != Some(context.generation()) {
            return Err(RenderError(
                "surface query belongs to a lost or different GPU domain".into(),
            ));
        }
        let limit = context.device.limits().max_texture_dimension_2d;
        if size.0 == 0 || size.1 == 0 || size.0 > limit || size.1 > limit {
            return Err(RenderError("invalid GPU surface dimensions".into()));
        }
        let (include_dynamic, dynamic_only) = match selection {
            SurfaceSelection::Static => (false, false),
            SurfaceSelection::Dynamic => (true, true),
            SurfaceSelection::All => (true, false),
        };
        self.encode_surface_gpu(
            &context.device,
            &context.queue,
            size.0,
            size.1,
            include_dynamic,
            dynamic_only,
        )
        .ok_or_else(|| RenderError("could not render GPU surface query".into()))?;
        let gpu = self.gpu.as_ref().unwrap();
        context.record_upload(
            gpu.paint_surface_vertex_count as u64
                * std::mem::size_of::<PaintSurfaceVertPod>() as u64
                + std::mem::size_of::<PaintSurfaceUniforms>() as u64,
        );
        Ok(SurfaceOutput {
            _target_leases: gpu
                .paint_surface_targets
                .as_ref()
                .map(|targets| [targets[0].clone(), targets[1].clone()]),
            world_texture: gpu.paint_surface_world_tex.as_ref().unwrap().clone(),
            metadata_texture: gpu.paint_surface_meta_tex.as_ref().unwrap().clone(),
            world: gpu.paint_surface_world_view.as_ref().unwrap().clone(),
            metadata: gpu.paint_surface_meta_view.as_ref().unwrap().clone(),
            size,
            generation: context.generation(),
            camera: self.camera3d,
            revision,
            selection,
        })
    }
}
