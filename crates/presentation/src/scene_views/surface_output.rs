use super::*;
use scenevm::surface_output::{SurfaceOutput, SurfaceSelection};

impl SceneViews {
    /// Query only the geometry of the acknowledged scene output. Stale or
    /// mutated views cannot supply depth for a newer/older visible frame.
    pub fn render_surface_output(
        &mut self,
        displayed: ViewImage,
        selection: SurfaceSelection,
    ) -> Result<Option<SurfaceOutput>> {
        if self.context.is_lost() {
            return Err(RenderError("surface query device is lost".into()));
        }
        let Some(view) = self.views.get_mut(&displayed.view) else {
            return Ok(None);
        };
        if view.dirty
            || view.isometric
            || view.output != Some(displayed)
            || view.size.0 == 0
            || view.size.1 == 0
        {
            return Ok(None);
        }
        let active = view.scene.vm.active_vm_index();
        view.scene.vm.set_active_vm(0);
        let result = view.scene.vm.render_surface_gpu(
            &self.context,
            view.size,
            selection,
            displayed.revision,
        );
        view.scene.vm.set_active_vm(active);
        result.map(Some)
    }
}
