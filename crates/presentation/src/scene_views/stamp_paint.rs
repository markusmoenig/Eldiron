//! CPU stamp pixels are supplied by the retained controller; this view owns
//! only the exact source/placement upload key on its injected VM.
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{StampSpriteSet, upload_sprite_set};
impl SceneViews {
    pub fn scene_source_preparation_cpu_ns(&self) -> u64 {
        self.source_upload_budget.preparation_cpu_ns()
    }
    pub fn scene_source_preparation_calls(&self) -> u64 {
        self.source_upload_budget.preparation_calls()
    }
    pub fn scene_source_uploaded_bytes(&self) -> usize {
        self.source_upload_budget.uploaded_bytes()
    }
    pub fn scene_source_gpu_copied_bytes(&self) -> u64 {
        self.source_upload_budget.gpu_copied_bytes()
    }
    pub fn scene_source_packed_cpu_bytes(&self) -> usize {
        self.source_upload_budget.packed_cpu_bytes()
    }
    /// Live owners call after source/placement preparation, before rendering.
    /// Private buffer writes share a budget across every live view in this frame.
    pub fn prepare_scene_source_uploads(&mut self, id: ViewId) -> Result<bool> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.scene.pending() {
            let ready = view.scene.prepare_sources(&mut self.source_upload_budget)?;
            view.scene_sources_ready = ready;
            view.dirty |= !ready;
            return Ok(ready);
        }
        if view.isometric || view.size.0 == 0 || view.size.1 == 0 {
            view.scene.vm.cancel_source_uploads();
            view.scene_sources_ready = true;
            return Ok(true);
        }
        if !view.surface_paint_ready || !view.stamp_paint_ready {
            view.scene.vm.cancel_source_uploads();
            view.scene_sources_ready = false;
            return Ok(false);
        }
        let ready = view
            .scene
            .vm
            .prepare_source_uploads(&mut self.source_upload_budget)?;
        view.dirty |= !view.scene_sources_ready || !ready;
        view.scene_sources_ready = ready;
        Ok(ready)
    }
    /// A prepared pose remains fixed while its private source packet is staged.
    /// CPU producers that have not published their source still need polling.
    pub fn scene_preparation_pending(&self, id: ViewId) -> Result<bool> {
        self.views
            .get(&id)
            .map(|view| {
                view.scene.pending()
                    || !view.scene_sources_ready
                        && view.surface_paint_ready
                        && view.stamp_paint_ready
            })
            .ok_or_else(|| RenderError("unknown scene view".into()))
    }
    pub fn scene_sources_ready(&self, id: ViewId) -> Result<bool> {
        self.views
            .get(&id)
            .map(|view| view.scene_sources_ready)
            .ok_or_else(|| RenderError("unknown scene view".into()))
    }
    pub fn cancel_scene_source_uploads(&mut self, id: ViewId) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        view.scene.vm.cancel_source_uploads();
        view.invalidate_request();
        Ok(())
    }
    pub fn defer_stamp_paint(&mut self, id: ViewId) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        view.stamp_paint_ready = false;
        view.scene.vm.cancel_source_uploads();
        view.invalidate_request();
        Ok(())
    }
    pub fn set_stamp_paint(
        &mut self,
        id: ViewId,
        source: Option<(&IsoPaintLayer, &StampSpriteSet)>,
    ) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let was_frame = view.scene.pending();
        let was_preparing = was_frame || !view.scene_sources_ready;
        let will_mutate =
            source.is_some() || view.stamp_uploaded.is_some() || !view.stamp_paint_ready;
        if was_frame && will_mutate {
            view.invalidate_request();
        }
        let mut changed = false;
        if let Some((layer, pixels)) = source {
            let vm = &mut view.scene.vm;
            let active = vm.active_vm_index();
            vm.set_active_vm(0);
            let camera = vm.active_vm().camera3d;
            let result = upload_sprite_set(
                pixels,
                layer,
                vm,
                camera,
                view.size.1,
                &mut view.stamp_uploaded,
            );
            vm.set_active_vm(active);
            changed = result.map_err(RenderError)?;
            view.dirty |= changed || !view.stamp_paint_ready;
        } else if view.stamp_uploaded.take().is_some() || !view.stamp_paint_ready {
            let vm = &mut view.scene.vm;
            let active = vm.active_vm_index();
            vm.set_active_vm(0);
            vm.execute(scenevm::Atom::ClearPaintBillboards);
            vm.set_active_vm(active);
            changed = true;
            view.dirty = true;
        }
        if changed && was_preparing {
            view.invalidate_request();
        }
        view.stamp_paint_ready = true;
        Ok(())
    }
}
