//! Bounded physical planes for large current live sources. CPU indices are shared;
//! each view owns its mutable residency and frozen request on the host queue.
use super::*;
use crate::orthographic_bake::PaintPageResidencyPreparation;
impl SceneViews {
    /// Configure before requesting sources. This limits color/material planes per
    /// source/view; diagnostics include all views for aggregate resource policy.
    pub fn set_surface_paint_residency_limit(&mut self, bytes: u64) -> Result<()> {
        if bytes < PAGE_BYTES as u64 || !self.surface_paint.bindings.is_empty() {
            return Err(RenderError(
                "configure a nonzero paint residency limit before requesting sources".into(),
            ));
        }
        self.surface_paint.plane_limit = bytes;
        Ok(())
    }
    pub fn private_paint_frame_pending(&self, id: ViewId) -> Result<bool> {
        self.views
            .get(&id)
            .map(|view| view.scene.pending())
            .ok_or_else(|| RenderError("unknown scene view".into()))
    }
    pub(super) fn advance_paged_surface_paint(&mut self) -> Result<()> {
        let keys: Vec<_> = self.views.keys().copied().collect();
        for offset in 0..keys.len() {
            if self.surface_paint.frame_steps >= FRAME_STEPS {
                break;
            }
            let started = self
                .surface_paint
                .frame_started
                .get_or_insert_with(web_time::Instant::now);
            if started.elapsed() >= web_time::Duration::from_millis(2) {
                break;
            }
            let id = keys[(self.surface_paint.view_turn % keys.len() + offset) % keys.len()];
            let view = self.views.get_mut(&id).unwrap();
            if view.size.0 == 0 || view.size.1 == 0 || view.scene.pending() {
                continue;
            }
            let Some(index) = view
                .surface_paint_source
                .as_ref()
                .and_then(|source| self.surface_paint.index(source))
            else {
                continue;
            };
            if !matches!(
                view.scene.vm.vm.render_mode,
                scenevm::RenderMode::Raster3D | scenevm::RenderMode::Compute3D
            ) {
                view.scene.retire_paint();
                view.installed_surface_paint_source = view.surface_paint_source.clone();
                view.surface_paint_ready = true;
                continue;
            }
            if view.scene.pages.is_some() {
                continue;
            }
            view.surface_paint_ready = false;
            if view.scene.preparation.is_none() {
                view.scene.preparation = Some(PaintPageResidencyPreparation::new(
                    self.context.clone(),
                    index,
                    self.surface_paint.plane_limit,
                )?);
            }
            let units = (FRAME_STEPS - self.surface_paint.frame_steps).min(64);
            self.surface_paint.frame_steps += units;
            let preparation = view.scene.preparation.as_mut().unwrap();
            if !preparation.advance(units)? {
                continue;
            }
            let bytes = preparation.upload(self.surface_paint.available_frame_bytes())?;
            self.surface_paint.frame_bytes += bytes;
            if preparation.ready() {
                view.scene.pages = Some(view.scene.preparation.take().unwrap().finish()?);
                let binding = view.scene.pages.as_ref().unwrap().binding()?;
                view.scene.vm.set_active_vm(0);
                view.scene
                    .vm
                    .set_paint_page_binding(&self.context, binding)?;
                view.installed_surface_paint_source = view.surface_paint_source.clone();
                view.surface_paint_ready = true;
                view.dirty = true;
            }
        }
        self.retire_unused_surface_paint();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
