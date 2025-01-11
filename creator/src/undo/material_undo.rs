use crate::prelude::*;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MaterialUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
}

impl MaterialUndoAtom {
    pub fn undo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            MaterialUndoAtom::MapEdit(prev, _) => {
                if let Some(material) = project.materials.get_mut(&prev.id) {
                    *material = *prev.clone();
                    material.clear_temp();
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Materialpicker"),
                    TheValue::Empty,
                ));
                crate::editor::RUSTERIX.lock().unwrap().set_dirty();
            }
        }
    }
    pub fn redo(&self, project: &mut Project, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            MaterialUndoAtom::MapEdit(_, next) => {
                if let Some(material) = project.materials.get_mut(&next.id) {
                    *material = *next.clone();
                    material.clear_temp();
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Materialpicker"),
                    TheValue::Empty,
                ));
                crate::editor::RUSTERIX.lock().unwrap().set_dirty();
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaterialUndo {
    pub stack: Vec<MaterialUndoAtom>,
    pub index: isize,
}

impl Default for MaterialUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialUndo {
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

    pub fn add(&mut self, atom: MaterialUndoAtom) {
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
}
