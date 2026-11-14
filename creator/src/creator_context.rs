//! Creator-owned authoring services. Shared runtime/project helpers receive the
//! CPU context through explicit borrows; they never own the editor's camera.
mod controllers;
mod bake;
pub use controllers::{CreatorActions, CreatorBake, CreatorDocks, CreatorTools};
mod history;
mod jobs;
pub use history::CreatorHistory;
mod reference;
mod reference_minimap;
pub use jobs::CreatorJobs;
pub use reference::CreatorReference;

use crate::{configeditor::ConfigEditor, editcamera::EditCamera, sidebar::SidebarMode};
use std::ops::{Deref, DerefMut};

pub struct CreatorContext {
    pub shared: shared::context::ServerContext,
    pub camera: EditCamera,
    pub config: ConfigEditor,
    pub(crate) hud_colors: crate::hud::HudColors,
    pub history: CreatorHistory,
    pub actions: CreatorActions,
    pub tools: CreatorTools,
    pub docks: CreatorDocks,
    pub sidebar_mode: SidebarMode,
    pub jobs: CreatorJobs,
    pub bake: CreatorBake,
    pub reference: CreatorReference,
}
impl Default for CreatorContext {
    fn default() -> Self {
        Self::new()
    }
}
impl CreatorContext {
    pub fn new() -> Self {
        Self {
            shared: shared::context::ServerContext::new(),
            camera: EditCamera::new(),
            config: ConfigEditor::new(),
            hud_colors: crate::hud::HudColors::default(),
            history: CreatorHistory::default(),
            actions: CreatorActions::default(),
            tools: CreatorTools::default(),
            docks: CreatorDocks::default(),
            sidebar_mode: SidebarMode::Region,
            jobs: CreatorJobs::default(),
            bake: CreatorBake::default(),
            reference: CreatorReference::default(),
        }
    }
    pub(crate) fn reset_bake_for_project_switch(&self) {
        if self.bake.initialized() {
            // Discard transient work without publishing a clear into the next
            // document, whose persisted map IDs may match the outgoing project.
            *self.bake.write() = rusterix::OrthographicBakeController::default();
        }
    }
}
// Source compatibility for existing controller field access. The persisted
// project schema and shared CPU context remain unchanged.
impl Deref for CreatorContext {
    type Target = shared::context::ServerContext;
    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}
impl DerefMut for CreatorContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shared
    }
}
