use crate::prelude::*;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScreenUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
}

impl ScreenUndoAtom {
    pub fn undo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            ScreenUndoAtom::MapEdit(prev, _) => {
                for (_, screen) in project.screens.iter_mut() {
                    if screen.map.id == prev.id {
                        screen.map = *prev.clone();
                        screen.map.clear_temp();
                    }
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Materialpicker"),
                    TheValue::Empty,
                ));
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
        }
    }
    pub fn redo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            ScreenUndoAtom::MapEdit(_, next) => {
                for (_, screen) in project.screens.iter_mut() {
                    if screen.map.id == next.id {
                        screen.map = *next.clone();
                        screen.map.clear_temp();
                    }
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Materialpicker"),
                    TheValue::Empty,
                ));
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScreenUndo {
    pub stack: Vec<ScreenUndoAtom>,
    pub index: isize,
}

impl Default for ScreenUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenUndo {
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

    pub fn add(&mut self, atom: ScreenUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(project, ui, ctx);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(project, ui, ctx);
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
