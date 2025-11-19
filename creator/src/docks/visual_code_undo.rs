use codegridfx::Module;
use theframework::prelude::*;

/// Undo atoms for visual code editor operations
#[derive(Clone, Debug)]
pub enum VisualCodeUndoAtom {
    /// Module edit: (before_module, after_module)
    ModuleEdit(Module, Module),
}

impl VisualCodeUndoAtom {
    pub fn undo(&self, module: &mut Module, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            VisualCodeUndoAtom::ModuleEdit(prev, _) => {
                *module = prev.clone();
                module.redraw(ui, ctx);
            }
        }
    }

    pub fn redo(&self, module: &mut Module, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            VisualCodeUndoAtom::ModuleEdit(_, next) => {
                *module = next.clone();
                module.redraw(ui, ctx);
            }
        }
    }
}

/// Undo stack for visual code editor
#[derive(Clone, Debug)]
pub struct VisualCodeUndo {
    pub stack: Vec<VisualCodeUndoAtom>,
    pub index: isize,
}

impl Default for VisualCodeUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualCodeUndo {
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
        self.index >= -1 && self.index < self.stack.len() as isize - 1
    }

    pub fn has_changes(&self) -> bool {
        // Has changes if the index is not at the beginning (i.e., not fully undone)
        self.index >= 0
    }

    pub fn add(&mut self, atom: VisualCodeUndoAtom) {
        // Remove any redo history
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, module: &mut Module, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(module, ui, ctx);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, module: &mut Module, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(module, ui, ctx);
        }
    }

    pub fn truncate_to_limit(&mut self, limit: usize) {
        if self.stack.len() > limit {
            let excess = self.stack.len() - limit;
            self.stack.drain(0..excess);
            self.index -= excess as isize;
            if self.index < -1 {
                self.index = -1;
            }
        }
    }
}
