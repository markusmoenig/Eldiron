use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone, Debug)]
pub enum UndoType {
    RegionChanged,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Undo {
    pub undo_type: UndoType,

    pub undo_data: String,
    pub redo_data: String,
}

impl Undo {
    pub fn new(undo_type: UndoType) -> Self {
        Self {
            undo_type,

            undo_data: String::new(),
            redo_data: String::new(),
        }
    }

    pub fn set_undo_region(&mut self, region: &Region) {
        if let Ok(json) = serde_json::to_string(&region) {
            self.undo_data = json;
        }
    }

    pub fn set_redo_region(&mut self, region: &Region) {
        if let Ok(json) = serde_json::to_string(&region) {
            self.redo_data = json;
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct UndoStack {
    pub stack: Vec<Undo>,

    pub index: isize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            index: -1,
        }
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

    pub fn add(&mut self, undo: Undo) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(undo);
        self.index += 1;
    }

    pub fn undo(&mut self) -> (UndoType, String) {
        let rc = (
            self.stack[self.index as usize].undo_type,
            self.stack[self.index as usize].undo_data.clone(),
        );
        self.index -= 1;
        rc
    }

    pub fn redo(&mut self) -> (UndoType, String) {
        self.index += 1;
        (
            self.stack[self.index as usize].undo_type,
            self.stack[self.index as usize].redo_data.clone(),
        )
    }
}
