//! Native/browser document I/O uses immutable bytes and addressed completion.
//! Tab activation, migrations, controller cancellation and undo stay in Creator.
use super::*;
pub(crate) use crate::documents::transfers::SaveInfo;
#[derive(serde::Serialize)]
pub(crate) struct DocumentInfo {
    pub document: Uuid,
    pub path: Option<PathBuf>,
    pub title: String,
    pub dirty: bool,
}
#[derive(serde::Serialize)]
pub(crate) struct SaveResult {
    pub document: Uuid,
    pub saved: bool,
    pub clean: bool,
}
// Compare authored content, not ZIP container timestamps or generated projections
// refreshed only on the encoder's clone. Canonical key ordering also survives tab
// snapshots whose hash-map iteration order differs. Retain full bytes rather than
// a collision-prone fingerprint: completion must never clean a different source.
fn source_bytes(project: &Project) -> Result<Vec<u8>, String> {
    let mut source = serde_json::to_value(project).map_err(|e| e.to_string())?;
    source.sort_all_objects();
    serde_json::to_vec(&source).map_err(|e| e.to_string())
}
impl Editor {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn has_document(&self, document: Uuid) -> bool {
        self.documents
            .sessions
            .iter()
            .any(|session| session.id == document)
    }
    pub(crate) fn enable_host_document_io(&mut self) {
        self.documents.host_io = true;
    }
    pub(crate) fn take_host_document_requests(&mut self) -> Vec<(&'static str, Uuid)> {
        std::mem::take(&mut self.documents.host_requests)
    }
    pub(super) fn queue_host_document_request(&mut self, name: &str) -> bool {
        if !self.documents.host_io {
            return false;
        }
        let kind = match name {
            "Open" => "open_document",
            "Save" => "save_document",
            "Save As" => "save_document_as",
            _ => return false,
        };
        if let Some(document) = self.documents.active_id() {
            if self.documents.host_requests.len() < 32
                && !self.documents.host_requests.contains(&(kind, document))
            {
                self.documents.host_requests.push((kind, document));
            }
        }
        true
    }

    pub(crate) fn document_info(&self) -> Option<DocumentInfo> {
        Some(DocumentInfo {
            document: self.documents.active_id()?,
            path: self.documents.project_path.clone(),
            title: self.window_title(),
            dirty: self.active_session_has_changes(),
        })
    }
    fn require_document(&self, document: Uuid) -> Result<(), String> {
        if self.documents.active_id() == Some(document) {
            Ok(())
        } else {
            Err("The document changed before this request completed".into())
        }
    }
    pub(crate) fn load_document_bytes(
        &mut self,
        document: Uuid,
        bytes: &[u8],
        path: Option<PathBuf>,
        replace_clean: bool,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<DocumentInfo, String> {
        self.require_document(document)?;
        // Decode completely before touching the outgoing document/controllers.
        let loaded = Self::load_project_from_bytes(bytes)?;
        if replace_clean {
            // Buffered dock edits may acquire their undo/dirty state on commit.
            self.commit_authoring_views(ctx)?;
            if self.active_session_has_changes() {
                return Err("The active document has unsaved changes".into());
            }
        }
        let mut icons = false;
        let mut redraw = false;
        // An untitled loaded file is not a new template: do not inject starters
        // merely because the host has no filesystem path for its bytes.
        if !self.install_project_session(
            loaded,
            path,
            false,
            replace_clean,
            ui,
            ctx,
            &mut icons,
            &mut redraw,
        ) {
            return Err("Unable to commit the current authoring views".into());
        }
        if icons {
            self.update_server_state_icons(ui);
        }
        ui.clear_dialog();
        ctx.ui.redraw_all = true;
        self.document_info()
            .ok_or_else(|| "No active document after loading".into())
    }
    pub(crate) fn prepare_document_save(
        &mut self,
        document: Uuid,
        ctx: &mut TheContext,
    ) -> Result<SaveInfo, String> {
        self.require_document(document)?;
        if self.documents.transfers.busy() {
            return Err("A document save is already pending".into());
        }
        self.commit_authoring_views(ctx)?;
        self.persist_active_region_view_state();
        let source = source_bytes(&self.documents.project)?;
        let bytes = shared::project_io::encode_project(&self.documents.project)?;
        self.documents.transfers.stage(document, bytes, source)
    }
    pub(crate) fn document_save_bytes(&self, ticket: u64) -> Option<&[u8]> {
        self.documents.transfers.bytes(ticket)
    }
    pub(crate) fn complete_document_save(
        &mut self,
        ticket: u64,
        success: bool,
        path: Option<PathBuf>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<SaveResult, String> {
        let save = self
            .documents
            .transfers
            .take(ticket)
            .ok_or("Document save is no longer pending")?;
        let document = save.info.document;
        let index = self
            .documents
            .sessions
            .iter()
            .position(|session| session.id == document)
            .ok_or("The saved document has closed")?;
        if !success {
            return Ok(SaveResult {
                document,
                saved: false,
                clean: false,
            });
        }
        if index == self.documents.active_session {
            // The write may have overlapped new edits. Mark clean only if the
            // current serializable content is exactly the saved snapshot.
            let committed = self.commit_authoring_views(ctx).is_ok();
            self.persist_active_region_view_state();
            if let Some(path) = path {
                self.documents.project_path = Some(path);
            }
            let clean = committed
                && source_bytes(&self.documents.project)
                    .is_ok_and(|current| current == save.source);
            if clean {
                self.mark_current_document_saved(ui);
            } else {
                self.sync_active_session_from_editor();
                self.rebuild_project_tabs(ui);
            }
            ctx.ui.redraw_all = true;
            Ok(SaveResult {
                document,
                saved: true,
                clean,
            })
        } else {
            let session = &mut self.documents.sessions[index];
            if let Some(path) = path {
                session.project_path = Some(path);
            }
            let clean = source_bytes(&session.project).is_ok_and(|current| current == save.source);
            if clean {
                session.undo.mark_saved();
                session.dirty = false;
                session.detached_dock_dirty = false;
            }
            self.rebuild_project_tabs(ui);
            ctx.ui.redraw_all = true;
            Ok(SaveResult {
                document,
                saved: true,
                clean,
            })
        }
    }
    pub(super) fn mark_current_document_saved(&mut self, ui: &mut TheUI) {
        self.server_ctx.history.write().mark_saved();
        DOCKMANAGER.write().unwrap().mark_saved();
        if let Some(session) = self
            .documents
            .sessions
            .get_mut(self.documents.active_session)
        {
            session.dirty = false;
            session.detached_dock_dirty = false;
        }
        self.sync_active_session_from_editor();
        self.rebuild_project_tabs(ui);
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn check_host_save_conflict_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let document = self.documents.active_id().unwrap();
        let save = self.prepare_document_save(document, ctx).unwrap();
        let name = self.documents.project.name.clone();
        self.documents
            .project
            .name
            .push_str(" / edited while host writes");
        let edited = self.documents.project.name.clone();
        let completed = self
            .complete_document_save(save.ticket, true, None, ui, ctx)
            .unwrap();
        assert!(
            completed.saved && !completed.clean,
            "a successful old snapshot cannot clean newer source"
        );
        assert_eq!(self.documents.project.name, edited);
        self.documents.project.name = name;
    }
}
