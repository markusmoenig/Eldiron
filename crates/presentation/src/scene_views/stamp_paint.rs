//! CPU stamp pixels are supplied by the retained controller; this view owns
//! only the exact source/placement upload key on its injected VM.
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{StampSpriteSet, upload_sprite_set};
impl SceneViews {
    pub fn scene_source_uploaded_bytes(&self) -> usize {
        self.source_upload_budget.uploaded_bytes()
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
        view.scene_sources_ready = true;
        Ok(())
    }
    pub fn defer_stamp_paint(&mut self, id: ViewId) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        view.stamp_paint_ready = false;
        view.scene.vm.cancel_source_uploads();
        view.dirty = true;
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
            view.dirty |= result.map_err(RenderError)? || !view.stamp_paint_ready;
        } else if view.stamp_uploaded.take().is_some() || !view.stamp_paint_ready {
            let vm = &mut view.scene.vm;
            let active = vm.active_vm_index();
            vm.set_active_vm(0);
            vm.execute(scenevm::Atom::ClearPaintBillboards);
            vm.set_active_vm(active);
            view.dirty = true;
        }
        view.stamp_paint_ready = true;
        Ok(())
    }
}
