//! Producer-owned paint residency on the scene's injected device. The binding
//! retains textures, not a CPU atlas; the producer controls writes and replay.
use crate::SceneVM;
use renderkit::{GpuContext, Result, wgpu};
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

mod binding;
pub use binding::{PaintPageBinding, PaintPageBindingPreparation};

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
