use super::*;
use crate::paint_pages::PaintPageBinding;
use std::sync::Arc;

pub(super) fn upload_entries(
    g: &mut VMGpu,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    entries: &[Raster3DSurfacePaintEntry],
) {
    use wgpu::util::DeviceExt;
    let dummy = [Raster3DSurfacePaintEntry::zeroed()];
    let entries = if entries.is_empty() { &dummy } else { entries };
    let bytes = bytemuck::cast_slice(entries);
    if g.raster3d_surface_paint_entries_ssbo.is_none()
        || g.raster3d_surface_paint_entries_size != bytes.len()
    {
        g.raster3d_surface_paint_entries_ssbo = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("vm-raster3d-surface-paint-entries"),
                contents: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        ));
        g.raster3d_surface_paint_entries_size = bytes.len();
    } else {
        queue.write_buffer(
            g.raster3d_surface_paint_entries_ssbo.as_ref().unwrap(),
            0,
            bytes,
        );
    }
}
impl VM {
    pub(crate) fn set_paint_dispatch(
        &mut self,
        region: Option<crate::paint_pages::PaintDispatchRegion>,
    ) {
        if region.is_none() || self.paint_dispatch.is_none() {
            self.partial_target_initialized = false;
            self.last_compute_history = None;
        }
        self.paint_dispatch = region;
    }
    pub(crate) fn set_paint_page_binding(
        &mut self,
        context: &renderkit::GpuContext,
        binding: Arc<PaintPageBinding>,
    ) -> renderkit::Result<()> {
        if context.is_lost()
            || self.gpu_domain != Some(context.generation())
            || binding.color.device_generation() != context.generation()
        {
            return Err(renderkit::RenderError(
                "paint binding belongs to a lost or different scene device".into(),
            ));
        }
        let alpha: FxHashSet<_> = binding.alpha_geo_ids.iter().copied().collect();
        if alpha != self.raster3d_paint_alpha_geo_ids {
            self.cached_static_raster_indices_valid = false;
        }
        self.raster3d_paint_alpha_geo_ids = alpha;
        self.raster3d_paint_overlay = None;
        self.raster3d_surface_paint = None;
        self.paged_surface_paint = Some(binding);
        self.raster3d_paint_overlay_dirty = true;
        Ok(())
    }
    pub(super) fn paint_entries(&self) -> Option<&[Raster3DSurfacePaintEntry]> {
        self.paged_surface_paint
            .as_ref()
            .map(|paint| paint.entries.as_ref())
            .or_else(|| {
                self.raster3d_surface_paint
                    .as_ref()
                    .map(|paint| paint.entries.as_slice())
            })
    }
    pub(super) fn upload_paged_paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> bool {
        let Some(paint) = &self.paged_surface_paint else {
            return false;
        };
        let Some(g) = self.gpu.as_mut() else {
            return false;
        };
        if self.raster3d_paint_overlay_dirty || g.raster3d_paint_color_view.is_none() {
            // Ordinary CPU atlas uploads must allocate their own texture after
            // leaving this binding, never overwrite the producer's atlas.
            g.raster3d_paint_color_tex = None;
            g.raster3d_paint_material_tex = None;
            g.raster3d_paint_color_view = Some(paint.color.view.clone());
            g.raster3d_paint_material_view = Some(paint.material.view.clone());
            g.raster3d_paint_tex_size =
                (paint.color.descriptor.width, paint.color.descriptor.height);
            upload_entries(g, device, queue, &paint.entries);
            self.raster3d_paint_overlay_dirty = false;
        }
        true
    }
    pub(crate) fn history_checkpoint(&self) -> Option<crate::paint_pages::LayerHistoryCheckpoint> {
        if !self.ping_pong_enabled {
            return None;
        }
        let gpu = self.ping_pong_textures.as_ref()?[self.ping_pong_front]
            .gpu
            .as_ref()?;
        Some(crate::paint_pages::LayerHistoryCheckpoint {
            front: self.ping_pong_front,
            generation: self.gpu_domain?,
            epoch: self.history_epoch,
            texture: gpu.texture.clone(),
            _lease: gpu.target_lease.clone(),
        })
    }
    pub(crate) fn restore_history(
        &mut self,
        checkpoint: crate::paint_pages::LayerHistoryCheckpoint,
    ) -> bool {
        if !self.ping_pong_enabled
            || self.gpu_domain != Some(checkpoint.generation)
            || checkpoint.epoch.checked_add(1) != Some(self.history_epoch)
            || self
                .target_context
                .as_ref()
                .is_none_or(|context| context.is_lost())
            || self.ping_pong_front != 1 - checkpoint.front
        {
            return false;
        }
        let Some(gpu) = self
            .ping_pong_textures
            .as_ref()
            .and_then(|pair| pair[checkpoint.front].gpu.as_ref())
        else {
            return false;
        };
        if gpu.texture != checkpoint.texture {
            return false;
        }
        self.ping_pong_front = checkpoint.front;
        true
    }
}
