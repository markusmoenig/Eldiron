use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionUndoAtom {
    ModelFXEdit(Vec3i, Option<ModelFXStore>, Option<ModelFXStore>),
}

impl RegionUndoAtom {
    pub fn undo(&self, region: &mut Region) {
        match self {
            RegionUndoAtom::ModelFXEdit(pos, prev, _) => {
                if let Some(prev) = prev {
                    region.models.insert((pos.x, pos.y, pos.z), prev.clone());
                } else {
                    region.models.remove(&(pos.x, pos.y, pos.z));
                }
            }
        }
    }
    pub fn redo(&self, region: &mut Region) {
        match self {
            RegionUndoAtom::ModelFXEdit(pos, _, next) => {
                if let Some(next) = next {
                    region.models.insert((pos.x, pos.y, pos.z), next.clone());
                } else {
                    region.models.remove(&(pos.x, pos.y, pos.z));
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionUndo {
    pub stack: Vec<RegionUndoAtom>,
    pub index: isize,
}

impl Default for RegionUndo {
    /// Creates a new `TheUndoStack` instance with default values.
    ///
    /// # Returns
    ///
    /// A new instance of `TheUndoStack` with empty stack and index set to -1.
    fn default() -> Self {
        Self::new()
    }
}

impl RegionUndo {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            index: -1,
        }
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

    pub fn add(&mut self, atom: RegionUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, region: &mut Region) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(region);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, region: &mut Region) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(region);
        }
    }

    // pub fn undo(&mut self) -> (TheId, String) {
    //     let rc = (
    //         self.stack[self.index as usize].id.clone(),
    //         self.stack[self.index as usize].undo_data.clone(),
    //     );
    //     self.index -= 1;
    //     rc
    // }

    // pub fn redo(&mut self) -> (TheId, String) {
    //     self.index += 1;
    //     (
    //         self.stack[self.index as usize].id.clone(),
    //         self.stack[self.index as usize].redo_data.clone(),
    //     )
    // }
}
