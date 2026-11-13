//! Each game view owns its GPU paint binding; CPU publications survive devices.
use super::*;
impl SceneViews {
    pub(crate) fn apply_game_surface_paint(
        &mut self,
        id: ViewId,
        source: Option<&Arc<rusterix::surface_paint::SurfacePaint>>,
    ) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let same = match (&view.game_surface_paint, source) {
            (None, None) => true,
            (Some(old), Some(next)) => std::sync::Arc::ptr_eq(old, next),
            _ => false,
        };
        if !same {
            view.scene.vm.set_active_vm(0);
            if let Some(source) = source {
                let limit = self.context.device.limits().max_texture_dimension_2d;
                if source.width > limit || source.height > limit {
                    return Err(RenderError(
                        "Surface paint atlas exceeds this device's texture limits".into(),
                    ));
                }
                view.scene.vm.set_raster3d_surface_paint(
                    source.width,
                    source.height,
                    source.color_rgba.clone(),
                    source.material_rgba.clone(),
                    source.entries.clone(),
                    source.alpha_geo_ids.clone(),
                );
            } else {
                view.scene
                    .vm
                    .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
            }
            view.game_surface_paint = source.cloned();
        }
        Ok(())
    }
}
