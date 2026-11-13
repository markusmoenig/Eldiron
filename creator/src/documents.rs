//! Creator-owned document/tab storage. UI, renderers and runtime clocks do not
//! live here. ProjectSession IDs are ephemeral and never change the file format.
use crate::undo::UndoManager;
#[path = "documents/transfers.rs"]
pub(crate) mod transfers;
use shared::project::Project;
use std::path::PathBuf;
use theframework_core::Uuid;

#[derive(Clone)]
pub(crate) struct ProjectSession {
    pub id: Uuid,
    pub project: Project,
    pub project_path: Option<PathBuf>,
    pub undo: UndoManager,
    pub dirty: bool,
    /// Dock-local undo remains in the workspace adapter. Preserve its unsaved
    /// status when the workspace detaches a document's widgets.
    pub detached_dock_dirty: bool,
}
impl ProjectSession {
    fn new(project: Project, project_path: Option<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4(),
            project,
            project_path,
            undo: UndoManager::default(),
            dirty: false,
            detached_dock_dirty: false,
        }
    }
}

pub(crate) struct Documents {
    /// Working document. Inactive tabs retain their source/history snapshots.
    pub project: Project,
    pub project_path: Option<PathBuf>,
    pub sessions: Vec<ProjectSession>,
    pub active_session: usize,
    pub transfers: transfers::Transfers,
    pub host_io: bool,
    pub host_requests: Vec<(&'static str, Uuid)>,
}

impl Documents {
    pub fn new(project: Project) -> Self {
        let initial = ProjectSession::new(project.clone(), None);
        Self {
            project,
            project_path: None,
            sessions: vec![initial],
            active_session: 0,
            transfers: Default::default(),
            host_io: false,
            host_requests: Vec::new(),
        }
    }

    pub fn active_id(&self) -> Option<Uuid> {
        self.sessions
            .get(self.active_session)
            .map(|session| session.id)
    }

    /// A fresh identity retires delayed callbacks even when the loaded project
    /// reuses every persisted map/widget ID from the outgoing tab.
    pub fn install(&mut self, project: Project, path: Option<PathBuf>, replace: bool) {
        let next = ProjectSession::new(project, path);
        if replace && self.active_session < self.sessions.len() {
            self.transfers
                .cancel_document(self.sessions[self.active_session].id);
            self.sessions[self.active_session] = next;
        } else {
            self.sessions.push(next);
            self.active_session = self.sessions.len() - 1;
        }
    }

    /// The workspace deactivates controllers before detaching their document.
    /// Always retain a valid empty tab when the last document closes.
    pub fn close_active(&mut self, empty: impl FnOnce() -> Project) -> bool {
        if self.active_session >= self.sessions.len() {
            return false;
        }
        self.transfers
            .cancel_document(self.sessions[self.active_session].id);
        self.sessions.remove(self.active_session);
        if self.sessions.is_empty() {
            self.sessions.push(ProjectSession::new(empty(), None));
            self.active_session = 0;
        } else {
            self.active_session = self.active_session.min(self.sessions.len() - 1);
        }
        true
    }

    /// Workspace controllers commit pending authoring edits before this call.
    /// Passing history explicitly keeps the document owner independent of the
    /// transitional process-wide undo/dock adapters.
    pub fn store_active(&mut self, undo: UndoManager, dock_dirty: bool) {
        let Some(session) = self.sessions.get_mut(self.active_session) else {
            return;
        };
        session.project = self.project.clone();
        session.project_path = self.project_path.clone();
        session.detached_dock_dirty |= dock_dirty;
        session.dirty = undo.has_unsaved() || session.detached_dock_dirty;
        session.undo = undo;
    }

    pub fn restore_active(&mut self) -> Option<UndoManager> {
        let session = self.sessions.get(self.active_session)?.clone();
        self.project = session.project;
        self.project_path = session.project_path;
        Some(session.undo)
    }
}
