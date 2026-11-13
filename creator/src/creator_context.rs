//! Creator-owned authoring services. Shared runtime/project helpers receive the
//! CPU context through explicit borrows; they never own the editor's camera.
mod controllers;
pub use controllers::{CreatorActions, CreatorDocks, CreatorTools};
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
            reference: CreatorReference::default(),
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
