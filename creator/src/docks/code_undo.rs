use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;

/// Undo atoms for code editor operations
#[derive(Clone)]
pub enum CodeUndoAtom {
    /// Text edit: (before_state, after_state)
    TextEdit(TheTextEditState, TheTextEditState),
}

impl CodeUndoAtom {
    pub fn undo(&self, edit: &mut dyn TheTextAreaEditTrait) {
        match self {
            CodeUndoAtom::TextEdit(prev, _) => {
                TheTextAreaEditTrait::set_state(edit, prev.clone());
            }
        }
    }

    pub fn redo(&self, edit: &mut dyn TheTextAreaEditTrait) {
        match self {
            CodeUndoAtom::TextEdit(_, next) => {
                TheTextAreaEditTrait::set_state(edit, next.clone());
            }
        }
    }
}

/// Undo stack for code editor
#[derive(Clone)]
pub struct CodeUndo {
    pub stack: Vec<CodeUndoAtom>,
    pub index: isize,
}

impl Default for CodeUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeUndo {
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

    pub fn add(&mut self, atom: CodeUndoAtom) {
        // Remove any redo history
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, edit: &mut dyn TheTextAreaEditTrait) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(edit);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, edit: &mut dyn TheTextAreaEditTrait) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(edit);
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
