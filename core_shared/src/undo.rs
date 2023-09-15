pub struct Undo {
    pub undo_data: String,
    pub redo_data: String,
}

pub struct UndoStack {
    pub stack: Vec<Undo>,

    pub index: isize,
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

    pub fn undo(&mut self) -> String {
        let rc = self.stack[self.index as usize].undo_data.clone();
        self.index -= 1;
        rc
    }

    pub fn redo(&mut self) -> String {
        self.index += 1;
        let rc = self.stack[self.index as usize].redo_data.clone();
        rc
    }

    pub fn add(&mut self, undo: String, redo: String) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(Undo {
            undo_data: undo,
            redo_data: redo,
        });
        self.index += 1;
    }
}
