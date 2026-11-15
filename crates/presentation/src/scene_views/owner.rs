//! Move a view's complete scene/target into a private request. Mutation cancels
//! that request and restores the CPU scene without swapping unrelated VMs.
use super::*;
use crate::{
    orthographic_bake::PaintPageResidency,
    raster_paint_frame::{RasterPaintAdvance, RasterPaintFrame},
};
use scenevm::render_output::SceneOutput;
use std::ops::{Deref, DerefMut};

pub(super) struct SceneOwner {
    scene: Option<SceneHandler>,
    pub target: Option<SceneRenderTarget>,
    pub pages: Option<PaintPageResidency>,
    frame: Option<RasterPaintFrame>,
}
impl SceneOwner {
    pub fn new(scene: SceneHandler, target: Option<SceneRenderTarget>) -> Self {
        Self {
            scene: Some(scene),
            target,
            pages: None,
            frame: None,
        }
    }
    pub fn pending(&self) -> bool {
        self.frame.is_some()
    }
    /// Do not recycle unresolved residency: a queued old consumer may still be
    /// reading it. Its scene bindings retain allocation lifetime until replaced;
    /// the next request receives a fresh mutable residency owner.
    pub fn cancel(&mut self) {
        if let Some(frame) = self.frame.take() {
            let (scene, target, pages, _) = frame.into_parts();
            drop(pages);
            self.scene = Some(scene);
            self.target = Some(target);
            self.pages = None;
        }
    }
    pub fn parts_mut(&mut self) -> (&mut SceneHandler, Option<&mut SceneRenderTarget>) {
        self.cancel();
        (self.scene.as_mut().unwrap(), self.target.as_mut())
    }
    pub fn start(&mut self, context: Arc<GpuContext>, revision: u64) -> Result<()> {
        if self.pending() {
            return Ok(());
        }
        let scene = self.scene.as_mut().unwrap();
        let target = self
            .target
            .as_ref()
            .ok_or_else(|| RenderError("private scene target missing".into()))?;
        let pages = self
            .pages
            .as_mut()
            .ok_or_else(|| RenderError("private paint residency missing".into()))?;
        RasterPaintFrame::prepare(&context, scene, target, pages)?;
        self.frame = Some(RasterPaintFrame::from_prepared(
            context,
            self.scene.take().unwrap(),
            self.target.take().unwrap(),
            self.pages.take().unwrap(),
            revision,
        ));
        Ok(())
    }
    pub fn advance(&mut self, available_bytes: usize) -> Result<Option<SceneOutput>> {
        let frame = self
            .frame
            .as_mut()
            .ok_or_else(|| RenderError("private scene frame missing".into()))?;
        if frame.advance_with_source_budget(available_bytes)? == RasterPaintAdvance::Pending {
            return Ok(None);
        }
        let (scene, target, pages, output) = self.frame.take().unwrap().into_parts();
        self.scene = Some(scene);
        self.target = Some(target);
        self.pages = Some(pages);
        Ok(output)
    }
    pub fn residency_usage(&self) -> Option<crate::orthographic_bake::PaintResidencyUsage> {
        self.pages
            .as_ref()
            .map(PaintPageResidency::usage)
            .or_else(|| self.frame.as_ref().map(RasterPaintFrame::residency_usage))
    }
    pub fn has_cpu_target_storage(&self) -> bool {
        self.frame
            .as_ref()
            .is_some_and(RasterPaintFrame::has_cpu_target_storage)
            || self
                .scene
                .as_ref()
                .is_some_and(|scene| scene.vm.render_target_cpu_bytes() > 0)
            || self
                .target
                .as_ref()
                .is_some_and(|target| target.has_cpu_storage() || target.has_readback_storage())
    }
}
impl Deref for SceneOwner {
    type Target = SceneHandler;
    fn deref(&self) -> &SceneHandler {
        self.scene
            .as_ref()
            .unwrap_or_else(|| self.frame.as_ref().unwrap().scene())
    }
}
impl DerefMut for SceneOwner {
    fn deref_mut(&mut self) -> &mut SceneHandler {
        self.cancel();
        self.scene.as_mut().unwrap()
    }
}
