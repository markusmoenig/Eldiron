//! Move a view's complete scene/target into a private request. Mutation cancels
//! that request and restores the CPU scene without swapping unrelated VMs.
use super::*;
use crate::{
    orthographic_bake::{PaintPageResidency, PaintPageResidencyPreparation},
    raster_paint_frame::{RasterPaintAdvance, RasterPaintFrame},
};
use scenevm::render_output::SceneOutput;
use std::ops::{Deref, DerefMut};
#[cfg(test)]
mod tests;

pub(crate) struct GamePaintScope {
    pub description: (Uuid, Uuid, rusterix::PlayerCamera, [u32; 17]),
    pub stamps: Option<Arc<rusterix::stamp_paint::StampPaint>>,
    pub map_revision: u32,
    pub assets_revision: u64,
}
impl PartialEq for GamePaintScope {
    fn eq(&self, other: &Self) -> bool {
        self.description == other.description
            && self.map_revision == other.map_revision
            && self.assets_revision == other.assets_revision
            && match (&self.stamps, &other.stamps) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}

pub(super) struct SceneOwner {
    scene: Option<SceneHandler>,
    pub target: Option<SceneRenderTarget>,
    pub pages: Option<PaintPageResidency>,
    frame: Option<RasterPaintFrame>,
    pub preparation: Option<PaintPageResidencyPreparation>,
}
impl SceneOwner {
    pub fn new(scene: SceneHandler, target: Option<SceneRenderTarget>) -> Self {
        Self {
            scene: Some(scene),
            target,
            pages: None,
            frame: None,
            preparation: None,
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
            let (mut scene, target, pages, _) = frame.into_parts();
            clear_base_paint(&mut scene);
            drop(pages);
            self.scene = Some(scene);
            self.target = Some(target);
            self.pages = None;
        }
    }
    pub fn retire_paint(&mut self) {
        self.cancel();
        clear_base_paint(self.scene.as_mut().unwrap());
        self.preparation = None;
        self.pages = None;
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
    pub fn prepare_sources(
        &mut self,
        budget: &mut scenevm::source_upload::SourceUploadBudget,
    ) -> Result<bool> {
        self.frame
            .as_mut()
            .ok_or_else(|| RenderError("private scene frame missing".into()))?
            .prepare_sources(budget)
    }
    pub fn advance(
        &mut self,
        available_bytes: usize,
        source_budget: &mut scenevm::source_upload::SourceUploadBudget,
    ) -> Result<Option<SceneOutput>> {
        let frame = self
            .frame
            .as_mut()
            .ok_or_else(|| RenderError("private scene frame missing".into()))?;
        if frame.advance_with_budgets(available_bytes, source_budget)?
            == RasterPaintAdvance::Pending
        {
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
            .or_else(|| {
                self.preparation
                    .as_ref()
                    .map(PaintPageResidencyPreparation::usage)
            })
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

impl SceneViews {
    pub(crate) fn keep_game_paint_frame(
        &mut self,
        id: ViewId,
        scope: &GamePaintScope,
    ) -> Result<bool> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let same_scope = view.game_scope.as_ref() == Some(scope);
        if !same_scope {
            view.scene.cancel();
            // Placement changes invalidate their own cursor. Retain unchanged
            // CPU pixel preparation across camera updates instead of restarting it.
            view.pending_content_revision = None;
            view.dirty = true;
        }
        Ok(same_scope
            && (view.scene.pending()
                || !view.scene_sources_ready && view.surface_paint_ready && view.stamp_paint_ready))
    }
    pub(crate) fn set_game_paint_scope(&mut self, id: ViewId, scope: GamePaintScope) -> Result<()> {
        self.views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?
            .game_scope = Some(scope);
        Ok(())
    }
}

fn clear_base_paint(scene: &mut SceneHandler) {
    let active = scene.vm.active_vm_index();
    scene.vm.set_active_vm(0);
    scene.vm.execute(scenevm::Atom::ClearRaster3DPaintOverlay);
    scene.vm.set_active_vm(active);
}
