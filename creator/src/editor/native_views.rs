//! Narrow bridge while Creator's document/world/tool owners are extracted.
use super::*;
impl Editor {
    pub(crate) fn enable_native_views(&mut self) {
        self.geometry_editor.enable_gpu_preview();
    }
    pub(crate) fn prepare_native_views(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<Option<crate::geometry_editor::PreviewToken>, String> {
        // Layout is current here; source/pose requests remain CPU preparation.
        if self.geometry_editor.wants_realtime() {
            self.geometry_editor
                .update(&self.project, ui, ctx, &self.server_ctx);
        }
        match self.geometry_editor.prepare_gpu_preview(ui, ctx, images) {
            Ok(token) => Ok(token),
            Err(error) => {
                self.geometry_editor.report_gpu_preview_error(error, ctx);
                Ok(self.geometry_editor.native_preview_fallback_token())
            }
        }
    }
    pub(crate) fn native_views_waiting(&self) -> bool {
        self.geometry_editor.gpu_preview_waiting()
    }
    pub(crate) fn presented_native_views(&mut self, token: crate::geometry_editor::PreviewToken) {
        self.geometry_editor.presented_gpu_preview(token);
    }
    pub(crate) fn invalidate_native_views(&mut self, ui: &mut TheUI) {
        self.geometry_editor.invalidate_gpu_preview(ui);
    }
}
#[cfg(test)]
impl Editor {
    pub(crate) fn project_for_native_test(&self) -> &Project {
        &self.project
    }
}
