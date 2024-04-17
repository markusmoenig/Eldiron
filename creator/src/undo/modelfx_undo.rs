use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXUndoAtom {
    AddNode(String, String),
    Edit(String, String),
}

impl ModelFXUndoAtom {
    pub fn undo(&self, model: &mut ModelFX) {
        match self {
            ModelFXUndoAtom::AddNode(prev, _) | ModelFXUndoAtom::Edit(prev, _) => {
                *model = ModelFX::from_json(prev);
            }
        }
    }
    pub fn redo(&self, model: &mut ModelFX) {
        match self {
            ModelFXUndoAtom::AddNode(_, next) | ModelFXUndoAtom::Edit(_, next) => {
                *model = ModelFX::from_json(next);
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ModelFXUndo {
    pub stack: Vec<ModelFXUndoAtom>,
    pub index: isize,
}

impl Default for ModelFXUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFXUndo {
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

    pub fn add(&mut self, atom: ModelFXUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, model: &mut ModelFX) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(model);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, model: &mut ModelFX) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(model);
        }
    }
}
