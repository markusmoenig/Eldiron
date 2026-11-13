//! Offscreen geometry queries whose channels stay on the injected device.
use crate::{Camera3D, SceneVM};
use renderkit::{GpuContext, RenderError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceSelection {
    Static,
    Dynamic,
    All,
}
/// World.xyz plus hardware depth.w, and UV.xy/face ID.z/valid.w.
/// Consumers derive camera-linear depth from world position and this snapshot's
/// camera, preserving the existing CPU surface query's depth convention.
/// Views lease their textures across resize/recovery; a later same-size query
/// reuses the producer target, just like the ordinary scene-output contract.
#[derive(Clone)]
pub struct SurfaceOutput {
    pub(crate) world: wgpu::TextureView,
    pub(crate) metadata: wgpu::TextureView,
    pub(crate) size: (u32, u32),
    pub(crate) generation: u64,
    pub(crate) camera: Camera3D,
    pub(crate) revision: u64,
    pub(crate) selection: SurfaceSelection,
}
impl SurfaceOutput {
    pub fn world(&self) -> &wgpu::TextureView {
        &self.world
    }
    pub fn metadata(&self) -> &wgpu::TextureView {
        &self.metadata
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
    pub fn camera(&self) -> Camera3D {
        self.camera
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn selection(&self) -> SurfaceSelection {
        self.selection
    }
}
impl SceneVM {
    /// Render cached geometry from this view's most recently prepared scene.
    /// No CPU target, staging buffer, map request or blocking poll is involved.
    pub fn render_surface_gpu(
        &mut self,
        context: &GpuContext,
        size: (u32, u32),
        selection: SurfaceSelection,
        revision: u64,
    ) -> Result<SurfaceOutput> {
        if self.device_generation() != Some(context.generation()) {
            return Err(RenderError(
                "surface query belongs to another scene device".into(),
            ));
        }
        self.active_vm_mut()
            .surface_output(context, size, selection, revision)
    }
    pub fn has_surface_readback_storage(&self) -> bool {
        self.active_vm()
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.paint_surface_readback.is_some())
    }
}
