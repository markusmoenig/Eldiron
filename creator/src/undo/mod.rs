pub mod project_atoms;
pub mod project_helper;
pub mod project_undo;

use crate::prelude::*;
use project_undo::*;

#[derive(Clone, Debug)]
pub struct UndoManager {
    pub max_undo: usize,

    project: ProjectUndo,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            max_undo: 30,

            project: ProjectUndo::default(),
        }
    }

    pub fn add_undo(&mut self, atom: ProjectUndoAtom, ctx: &mut TheContext) {
        self.project.add(atom);
        self.project.truncate_to_limit(self.max_undo);
        ctx.ui.set_enabled("Undo");
        self.can_save(ctx);
    }

    pub fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if !self.project.has_undo() {
            ctx.ui.set_disabled("Undo");
        } else {
            ctx.ui.set_enabled("Undo");
        }

        if !self.project.has_redo() {
            ctx.ui.set_disabled("Redo");
        } else {
            ctx.ui.set_enabled("Redo");
        }
    }

    pub fn undo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.project.has_undo() {
            self.project.undo(project, ui, ctx, server_ctx);
        }

        self.set_undo_state_to_ui(ctx);
        self.can_save(ctx);
    }

    pub fn redo(
        &mut self,
        server_ctx: &mut ServerContext,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if self.project.has_redo() {
            self.project.redo(project, ui, ctx, server_ctx);
        }

        self.set_undo_state_to_ui(ctx);
        self.can_save(ctx);
    }

    /// Checks if the undo manager is empty and disables the save buttons if it is.
    pub fn can_save(&self, ctx: &mut TheContext) {
        if self.has_undo() {
            // ctx.ui.set_disabled("Save");
            // ctx.ui.set_disabled("Save As");
        } else {
            ctx.ui.set_enabled("Save");
            ctx.ui.set_enabled("Save As");
        }
    }

    /// Checks if the undo manager has any undoable actions.
    pub fn has_undo(&self) -> bool {
        if self.project.has_undo() {
            return true;
        }
        false
    }
}
