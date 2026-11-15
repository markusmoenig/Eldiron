//! Document service facade for foreign/browser hosts. It carries CPU bytes;
//! presentation and filesystem operations stay with their existing owners.
use super::*;
use crate::editor::{DocumentInfo, SaveInfo, SaveResult};
use std::path::PathBuf;
impl NativeCreatorApplication {
    pub(crate) fn host_creator_theme(&self) -> u32 {
        match self.editor.native_creator_theme() {
            crate::editor::CreatorTheme::Brown => 0,
            crate::editor::CreatorTheme::Black => 1,
            crate::editor::CreatorTheme::Light => 2,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn host_has_document(&self, document: Uuid) -> bool {
        self.editor.has_document(document)
    }
    #[cfg(any(target_arch = "wasm32", feature = "staticlib"))]
    pub(crate) fn host_view_diagnostics(&self) -> serde_json::Value {
        self.editor.native_view_diagnostics()
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn host_runtime_status(&self) -> Option<(Uuid, u64, rusterix::ServerState)> {
        self.editor.native_runtime_status()
    }

    pub(crate) fn host_enable_document_io(&mut self) {
        self.editor.enable_host_document_io();
    }
    pub(crate) fn host_document_requests(&mut self) -> Vec<(&'static str, Uuid)> {
        self.editor.take_host_document_requests()
    }

    pub(crate) fn host_document_info(&self) -> Option<DocumentInfo> {
        self.editor.document_info()
    }
    pub(crate) fn host_load_document(
        &mut self,
        session: &mut UiSession,
        document: Uuid,
        bytes: &[u8],
        path: Option<PathBuf>,
        replace_clean: bool,
    ) -> std::result::Result<DocumentInfo, String> {
        let info = self.editor.load_document_bytes(
            document,
            bytes,
            path,
            replace_clean,
            &mut session.ui,
            &mut session.context,
        )?;
        self.pending = None;
        self.editor.invalidate_native_views(&mut session.ui);
        Ok(info)
    }
    pub(crate) fn host_prepare_document_save(
        &mut self,
        session: &mut UiSession,
        document: Uuid,
    ) -> std::result::Result<SaveInfo, String> {
        self.editor
            .prepare_document_save(document, &mut session.context)
    }
    pub(crate) fn host_document_save_bytes(&self, ticket: u64) -> Option<&[u8]> {
        self.editor.document_save_bytes(ticket)
    }
    pub(crate) fn host_complete_document_save(
        &mut self,
        session: &mut UiSession,
        ticket: u64,
        success: bool,
        path: Option<PathBuf>,
    ) -> std::result::Result<SaveResult, String> {
        self.editor.complete_document_save(
            ticket,
            success,
            path,
            &mut session.ui,
            &mut session.context,
        )
    }
}
