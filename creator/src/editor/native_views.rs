//! Narrow bridge while Creator's document/world/tool owners are extracted.
use super::*;
pub(crate) struct NativeViewTokens {
    pub(crate) geometry: Option<crate::geometry_editor::PreviewToken>,
    pub(crate) appearance: Option<crate::appearance_workspace::PreviewToken>,
    pub(crate) material: Option<crate::procedural_material::PreviewToken>,
}
impl NativeViewTokens {
    pub(crate) fn retain_painted(&mut self, list: &theframework::paint::PaintList) {
        let receipts = list.receipts().collect::<std::collections::HashSet<_>>();
        self.geometry = self
            .geometry
            .filter(|token| receipts.contains(&token.receipt()));
        self.appearance = self
            .appearance
            .filter(|token| receipts.contains(&token.receipt()));
        self.material = self
            .material
            .filter(|token| receipts.contains(&token.receipt()));
    }
}
impl Editor {
    pub(crate) fn enable_native_views(&mut self) {
        self.geometry_editor.enable_gpu_preview();
        self.appearance_workspace.enable_gpu_preview();
        self.procedural_material.enable_gpu_preview();
    }
    pub(crate) fn prepare_native_views(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<Option<NativeViewTokens>, String> {
        // Layout is current here; source/pose requests remain CPU preparation.
        if self.geometry_editor.wants_realtime() {
            self.geometry_editor
                .update(&self.project, ui, ctx, &self.server_ctx);
        }
        let geometry = match self.geometry_editor.prepare_gpu_preview(ui, ctx, images) {
            Ok(token) => token,
            Err(error) => {
                self.geometry_editor.report_gpu_preview_error(error, ctx);
                self.geometry_editor.native_preview_fallback_token()
            }
        };
        // An already queued appearance description is refreshed only after the
        // current layout, so physical output and view bounds use one frame.
        if self.appearance_workspace.visible(ui) {
            self.appearance_workspace.update(
                &self.project,
                ui,
                ctx,
                self.procedural_appearance_revision,
            );
        }
        let appearance = self
            .appearance_workspace
            .prepare_gpu_preview(ui, ctx, images);
        let material = self.procedural_material.prepare_gpu_preview(
            ui,
            ctx,
            images,
            self.server_ctx.animation_counter,
        );
        Ok(Some(NativeViewTokens {
            geometry,
            appearance,
            material,
        }))
    }
    pub(crate) fn native_views_waiting(&self) -> bool {
        self.geometry_editor.gpu_preview_waiting()
            || self.appearance_workspace.gpu_preview_waiting()
            || self.procedural_material.gpu_preview_waiting()
    }
    pub(crate) fn presented_native_views(
        &mut self,
        tokens: NativeViewTokens,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if let Some(token) = tokens.geometry {
            self.geometry_editor.presented_gpu_preview(token);
        }
        if let Some(token) = tokens.appearance {
            self.appearance_workspace
                .presented_gpu_preview(token, ui, ctx);
        }
        if let Some(token) = tokens.material {
            self.procedural_material.presented_gpu_preview(token);
        }
    }
    pub(crate) fn invalidate_native_views(&mut self, ui: &mut TheUI) {
        self.geometry_editor.invalidate_gpu_preview(ui);
        self.appearance_workspace.invalidate_gpu_preview(ui);
        self.procedural_material.invalidate_gpu_preview(ui);
    }
}
#[cfg(test)]
impl Editor {
    pub(crate) fn project_for_native_test(&self) -> &Project {
        &self.project
    }
}
