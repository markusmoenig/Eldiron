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
        || g.raster3d_surface_paint_entries_shared
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
    g.raster3d_surface_paint_entries_shared = false;
}
impl VM {
    /// Compute's existing auxiliary layout embeds the paint table. A consecutive
    /// producer revision mirrors at most two rectangles entirely on the GPU.
    /// First use or skipped revisions copy the complete current GPU table.
    pub(super) fn sync_compute_paint_table(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> crate::SceneVMResult<()> {
        let table = self
            .paged_surface_paint
            .as_ref()
            .and_then(|binding| binding.resident_table.as_ref());
        let gpu = self.gpu.as_mut().unwrap();
        let Some(table) = table else {
            gpu.compute3d_paint_table = None;
            return Ok(());
        };
        let update = table.update.lock().map_err(|_| {
            crate::SceneVMError::InvalidOperation("paint residency update was interrupted".into())
        })?;
        let previous = gpu
            .compute3d_paint_table
            .as_ref()
            .filter(|(buffer, _)| *buffer == table.buffer)
            .map(|(_, revision)| *revision);
        if previous == Some(update.revision) {
            return Ok(());
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("private compute paint lookup mirror"),
        });
        let destination = gpu.compute3d_aux_ssbo.as_ref().unwrap();
        if previous.and_then(|revision| revision.checked_add(1)) == Some(update.revision) {
            for slot in update.rectangles.into_iter().flatten() {
                let offset = u64::from(slot) * size_of::<Raster3DSurfacePaintEntry>() as u64
                    + std::mem::offset_of!(Raster3DSurfacePaintEntry, atlas_rect) as u64;
                encoder.copy_buffer_to_buffer(&table.buffer, offset, destination, 20 + offset, 16);
            }
        } else {
            encoder.copy_buffer_to_buffer(&table.buffer, 0, destination, 20, table.buffer.size());
        }
        queue.submit(Some(encoder.finish()));
        gpu.compute3d_paint_table = Some((table.buffer.clone(), update.revision));
        Ok(())
    }
    pub(super) fn paint_has_missing_pages(&self) -> bool {
        self.paged_surface_paint.as_ref().map_or_else(
            || {
                self.paint_entries().is_some_and(|entries| {
                    entries.iter().any(|entry| {
                        entry.uv_size.iter().all(|size| *size > 0)
                            && (entry.atlas_rect[2] == 0 || entry.atlas_rect[3] == 0)
                    })
                })
            },
            |binding| binding.has_missing,
        )
    }
    pub(super) fn prepare_raster_paint_feedback(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        use wgpu::util::DeviceExt;
        let gpu = self.gpu.as_mut().unwrap();
        let ready = u32::MAX.to_le_bytes();
        if let Some(buffer) = &gpu.raster3d_paint_feedback {
            queue.write_buffer(buffer, 0, &ready);
        } else {
            gpu.raster3d_paint_feedback = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("per-view raster paint dependency"),
                    contents: &ready,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }
        if let Some(context) = &self.target_context {
            context.record_upload(4);
        }
    }
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
        if self
            .paged_surface_paint
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &binding))
        {
            return Ok(());
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
            if let Some(entries) = &paint.raster_entries {
                g.raster3d_surface_paint_entries_ssbo = Some(entries.clone());
                g.raster3d_surface_paint_entries_size = size_of_val(paint.entries.as_ref());
                g.raster3d_surface_paint_entries_shared = true;
            } else {
                // Explicit reference/compute owners may keep their own upload
                // policy. Live SceneViews supply the complete shared buffer.
                upload_entries(g, device, queue, &paint.entries);
            }
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
