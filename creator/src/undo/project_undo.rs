// use crate::editor::{SCENEMANAGER, SHADEGRIDFX};
use crate::prelude::*;
use crate::undo::project_atoms::ProjectUndoAtom;
use theframework::prelude::*;

#[derive(Clone, Debug)]
pub struct ProjectUndo {
    pub stack: Vec<ProjectUndoAtom>,
    pub index: isize,
}

impl Default for ProjectUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectUndo {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            index: -1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.stack = vec![];
        self.index = -1;
    }

    pub fn has_undo(&self) -> bool {
        self.index >= 0
    }

    pub fn has_redo(&self) -> bool {
        if self.index >= -1 && self.index < self.stack.len() as isize - 1 {
            return true;
        }
        false
    }

    pub fn add(&mut self, atom: ProjectUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(project, ui, ctx, server_ctx);
            self.index -= 1;
        }
    }

    pub fn redo(
        &mut self,
        region: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(region, ui, ctx, server_ctx);
        }
    }

    pub fn truncate_to_limit(&mut self, limit: usize) {
        if self.stack.len() > limit {
            let excess = self.stack.len() - limit;

            // Remove the oldest `excess` entries from the front
            self.stack.drain(0..excess);

            // Adjust the index accordingly
            self.index -= excess as isize;

            // Clamp to -1 minimum in case we truncated everything
            if self.index < -1 {
                self.index = -1;
            }
        }
    }
}
