pub mod code;
pub mod code_undo;
pub mod data;
pub mod data_undo;
pub mod log;
pub mod tilemap;
pub mod tiles;
pub mod tiles_editor;
pub mod tiles_editor_undo;
pub mod visual_code;
pub mod visual_code_undo;

pub use crate::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum DockDefaultState {
    Minimized,
    Maximized,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DockMaximizedState {
    Maximized,
    Editor,
}

#[allow(unused)]
pub trait Dock: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn setup(&mut self, ctx: &mut TheContext) -> TheCanvas;

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
    }

    fn minimized(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {}

    fn supports_actions(&self) -> bool {
        true
    }

    fn default_state(&self) -> DockDefaultState {
        DockDefaultState::Minimized
    }

    fn maximized_state(&self) -> DockMaximizedState {
        DockMaximizedState::Maximized
    }

    /// Dock supports an import operation from JSON.
    fn import(
        &mut self,
        content: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
    }

    /// Dock supports an export operation to JSON.
    fn export(&self) -> Option<String> {
        None
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    /// Returns true if this dock supports internal undo / redo.
    fn supports_undo(&self) -> bool {
        false
    }

    /// Returns true if this dock has unsaved changes in its undo stack.
    fn has_changes(&self) -> bool {
        false
    }

    /// If the dock supports undo, set its current state to the UI.
    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {}

    /// Undo an action in the current dock
    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
    }

    /// Redo an action in the current dock
    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
    }

    /// Returns the custom editor tools for this dock.
    /// If None, no custom tools are available.
    fn editor_tools(&self) -> Option<Vec<Box<dyn EditorTool>>> {
        None
    }

    /// Draw custom minimap content when this dock is active.
    /// Returns true if the dock drew custom content (minimap should not draw default content).
    /// Returns false to let the default minimap drawing proceed.
    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        false
    }

    /// Returns true if this dock animates minimap content (requires soft updates each tick).
    fn supports_minimap_animation(&self) -> bool {
        false
    }
}
