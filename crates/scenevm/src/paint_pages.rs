//! Producer-owned paint residency on the scene's injected device. The binding
//! retains textures, not a CPU atlas; the producer controls writes and replay.
use crate::{GeoId, Raster3DSurfacePaintEntry, SceneVM};
use renderkit::{GpuContext, GpuImage, RenderError, Result, wgpu};
use std::sync::Arc;

/// Absolute pixel rectangle in one unchanged full-size camera/sample target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintDispatchRegion {
    pub origin: [u32; 2],
    pub size: [u32; 2],
}
impl PaintDispatchRegion {
    pub fn full(size: [u32; 2]) -> Self {
        Self {
            origin: [0; 2],
            size,
        }
    }
    pub fn fits(self, target: [u32; 2]) -> bool {
        (0..2).all(|axis| {
            self.size[axis] > 0
                && self.origin[axis]
                    .checked_add(self.size[axis])
                    .is_some_and(|end| end <= target[axis])
        })
    }
    pub fn split(self) -> Option<[Self; 2]> {
        let axis = if self.size[0] >= self.size[1] && self.size[0] > 1 {
            0
        } else if self.size[1] > 1 {
            1
        } else {
            return None;
        };
        let mut a = self;
        let mut b = self;
        a.size[axis] /= 2;
        b.origin[axis] = b.origin[axis].checked_add(a.size[axis])?;
        b.size[axis] -= a.size[axis];
        Some([a, b])
    }
}

pub struct PaintPageBinding {
    pub(crate) color: Arc<GpuImage>,
    pub(crate) material: Arc<GpuImage>,
    pub(crate) entries: Arc<[Raster3DSurfacePaintEntry]>,
    pub(crate) alpha_geo_ids: Vec<GeoId>,
    pub(crate) has_missing: bool,
}
impl PaintPageBinding {
    /// Submit consumers before the producer rewrites any resident texture slot.
    /// Retaining this binding preserves allocation lifetime, not a pixel copy.
    pub fn new(
        context: &GpuContext,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<[Raster3DSurfacePaintEntry]>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Arc<Self>> {
        if context.is_lost()
            || color.device_generation() != context.generation()
            || material.device_generation() != context.generation()
            || color.descriptor.format != wgpu::TextureFormat::Rgba8Unorm
            || material.descriptor.format != wgpu::TextureFormat::Rgba8Uint
            || color.descriptor.width != material.descriptor.width
            || color.descriptor.height != material.descriptor.height
            || ![&color, &material].iter().all(|image| {
                image
                    .texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
                    && image.texture.sample_count() == 1
                    && image.texture.depth_or_array_layers() == 1
                    && image.texture.dimension() == wgpu::TextureDimension::D2
            })
            || entries.len() < 2
            || !entries.len().is_power_of_two()
            || (size_of_val(entries.as_ref()) as u64 + 20)
                > u64::from(context.device.limits().max_storage_buffer_binding_size)
        {
            return Err(RenderError(
                "invalid paint page binding/device/table".into(),
            ));
        }
        let mut has_missing = false;
        for entry in entries.iter().filter(|entry| entry.uv_size != [0; 2]) {
            if entry.uv_size != [64; 2] {
                return Err(RenderError("invalid paint page UV dimensions".into()));
            }
            if entry.atlas_rect[2..] == [0, 0] {
                has_missing = true;
                continue;
            }
            if entry.atlas_rect[2..] != [64, 64]
                || !entry.atlas_rect[0]
                    .checked_add(64)
                    .is_some_and(|end| end <= color.descriptor.width)
                || !entry.atlas_rect[1]
                    .checked_add(64)
                    .is_some_and(|end| end <= color.descriptor.height)
            {
                return Err(RenderError(
                    "paint page rectangle exceeds its texture".into(),
                ));
            }
        }
        Ok(Arc::new(Self {
            color,
            material,
            entries,
            alpha_geo_ids,
            has_missing,
        }))
    }
}
pub struct LayerHistoryCheckpoint {
    pub(crate) front: usize,
    pub(crate) generation: u64,
    pub(crate) epoch: u64,
    pub(crate) texture: wgpu::Texture,
    pub(crate) _lease: Option<renderkit::TargetLease>,
}
impl SceneVM {
    /// None restores ordinary full-frame clear/dispatch behavior. While Some,
    /// the owner must resolve every region before publishing the working image.
    pub fn set_layer_paint_dispatch(
        &mut self,
        layer: usize,
        region: Option<PaintDispatchRegion>,
    ) -> bool {
        let Some(vm) = self.vm_mut_by_index(layer) else {
            return false;
        };
        vm.set_paint_dispatch(region);
        true
    }
    pub fn take_layer_attempt_history(&mut self, layer: usize) -> Option<LayerHistoryCheckpoint> {
        self.vm_mut_by_index(layer)?.last_compute_history.take()
    }
    pub fn set_paint_page_binding(
        &mut self,
        context: &GpuContext,
        binding: Arc<PaintPageBinding>,
    ) -> Result<()> {
        self.active_vm_mut()
            .set_paint_page_binding(context, binding)
    }
    /// A faulted private bake pass must restore this before replaying the same
    /// sample. The old read texture was not written by that failed dispatch.
    pub fn layer_history_checkpoint(&self, layer: usize) -> Option<LayerHistoryCheckpoint> {
        self.vm_ref_by_index(layer)
            .and_then(|vm| vm.history_checkpoint())
    }
    pub fn restore_layer_history(
        &mut self,
        layer: usize,
        checkpoint: LayerHistoryCheckpoint,
    ) -> bool {
        self.vm_mut_by_index(layer)
            .is_some_and(|vm| vm.restore_history(checkpoint))
    }
}
