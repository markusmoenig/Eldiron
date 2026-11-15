//! Canonical geometry-query targets retain the same device pool leases as other
//! view outputs. Every query clears all attachments before exposing any texels.
use super::*;

impl VM {
    pub(in crate::vm) fn ensure_paint_surface_targets(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> crate::SceneVMResult<()> {
        let gpu = self.gpu.as_mut().unwrap();
        if gpu.paint_surface_tex_size == (width, height)
            && gpu.paint_surface_world_tex.is_some()
            && gpu.paint_surface_meta_tex.is_some()
            && gpu.paint_surface_depth_tex.is_some()
        {
            return Ok(());
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING;
        let (world, metadata, depth, leases) = if let Some(context) = &self.target_context {
            let color = renderkit::TargetDescriptor {
                usage,
                ..renderkit::TargetDescriptor::color(
                    width,
                    height,
                    wgpu::TextureFormat::Rgba32Float,
                )
            };
            let acquire = |descriptor| {
                context
                    .acquire_target(descriptor)
                    .map_err(|error| crate::SceneVMError::BufferAllocationFailed(error.to_string()))
            };
            let leases = [
                acquire(color)?,
                acquire(color)?,
                acquire(renderkit::TargetDescriptor::attachment(
                    width,
                    height,
                    wgpu::TextureFormat::Depth32Float,
                    1,
                ))?,
            ];
            (
                leases[0].texture().clone(),
                leases[1].texture().clone(),
                leases[2].texture().clone(),
                Some(leases),
            )
        } else {
            let descriptor = |label, format, usage| wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            };
            (
                device.create_texture(&descriptor(
                    "reference-surface-world",
                    wgpu::TextureFormat::Rgba32Float,
                    usage,
                )),
                device.create_texture(&descriptor(
                    "reference-surface-meta",
                    wgpu::TextureFormat::Rgba32Float,
                    usage,
                )),
                device.create_texture(&descriptor(
                    "reference-surface-depth",
                    wgpu::TextureFormat::Depth32Float,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                )),
                None,
            )
        };
        gpu.paint_surface_world_view = Some(leases.as_ref().map_or_else(
            || world.create_view(&Default::default()),
            |targets| targets[0].view().clone(),
        ));
        gpu.paint_surface_meta_view = Some(leases.as_ref().map_or_else(
            || metadata.create_view(&Default::default()),
            |targets| targets[1].view().clone(),
        ));
        gpu.paint_surface_depth_view = Some(leases.as_ref().map_or_else(
            || depth.create_view(&Default::default()),
            |targets| targets[2].view().clone(),
        ));
        gpu.paint_surface_world_tex = Some(world);
        gpu.paint_surface_meta_tex = Some(metadata);
        gpu.paint_surface_depth_tex = Some(depth);
        gpu.paint_surface_targets = leases;
        gpu.paint_surface_normal_view = None;
        gpu.paint_surface_normal_tex = None;
        // Only the explicit CPU query allocates staging storage.
        gpu.paint_surface_readback = None;
        gpu.paint_surface_readback_size = 0;
        gpu.paint_surface_tex_size = (width, height);
        Ok(())
    }
}
