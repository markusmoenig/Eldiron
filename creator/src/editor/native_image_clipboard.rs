//! Addressed CPU asset results reach the existing authoring owner. The host
//! transports bytes; it never clears a selection or installs a paste itself.
use super::*;
use theframework::platform::{ImageClipboardResult, InputResponse, UiSession};
impl Editor {
    pub(crate) fn native_image_clipboard(
        &mut self,
        request: u64,
        result: &ImageClipboardResult,
        session: &mut UiSession,
    ) -> InputResponse {
        let redraw = DOCKMANAGER.write().unwrap().handle_event(
            &TheEvent::ImageClipboardResult(request, result.clone()),
            &mut session.ui,
            &mut session.context,
            &mut self.documents.project,
            &mut self.server_ctx,
        );
        InputResponse {
            consumed: true,
            redraw,
        }
    }
}
