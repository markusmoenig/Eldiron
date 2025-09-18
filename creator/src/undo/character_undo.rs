use crate::editor::{CODEGRIDFX, NODEEDITOR};
use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CharacterUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
    TemplateModuleEdit(Uuid, Module, Module),
    InstanceModuleEdit(Uuid, Uuid, Module, Module),
}

impl CharacterUndoAtom {
    pub fn undo(&self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            CharacterUndoAtom::MapEdit(prev, _) => {
                for character in project.characters.values_mut() {
                    if character.map.id == prev.id {
                        character.map = *prev.clone();
                        character.map.clear_temp();
                        NODEEDITOR
                            .write()
                            .unwrap()
                            .force_update(ctx, &mut character.map);
                        break;
                    }
                }
                NODEEDITOR
                    .write()
                    .unwrap()
                    .set_selected_node_ui(project, ui, ctx, false);
            }
            CharacterUndoAtom::TemplateModuleEdit(id, prev, _) => {
                if let Some(character) = project.characters.get_mut(id) {
                    character.module = prev.clone();
                    character.module.redraw(ui, ctx);
                    character.module.show_settings(ui, ctx);
                    *CODEGRIDFX.write().unwrap() = character.module.clone();
                }
            }
            CharacterUndoAtom::InstanceModuleEdit(region_id, id, prev, _) => {
                if let Some(region) = project.get_region_mut(region_id) {
                    if let Some(character) = region.characters.get_mut(id) {
                        character.module = prev.clone();
                        character.module.redraw(ui, ctx);
                        character.module.show_settings(ui, ctx);
                        *CODEGRIDFX.write().unwrap() = character.module.clone();
                    }
                }
            }
        }
    }
    pub fn redo(&self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            CharacterUndoAtom::MapEdit(_, next) => {
                for character in project.characters.values_mut() {
                    if character.map.id == next.id {
                        character.map = *next.clone();
                        character.map.clear_temp();
                        NODEEDITOR
                            .write()
                            .unwrap()
                            .force_update(ctx, &mut character.map);
                        break;
                    }
                }

                NODEEDITOR
                    .write()
                    .unwrap()
                    .set_selected_node_ui(project, ui, ctx, false);
            }
            CharacterUndoAtom::TemplateModuleEdit(id, _, next) => {
                if let Some(character) = project.characters.get_mut(id) {
                    character.module = next.clone();
                    character.module.redraw(ui, ctx);
                    character.module.show_settings(ui, ctx);
                    *CODEGRIDFX.write().unwrap() = character.module.clone();
                }
            }
            CharacterUndoAtom::InstanceModuleEdit(region_id, id, _, next) => {
                if let Some(region) = project.get_region_mut(region_id) {
                    if let Some(character) = region.characters.get_mut(id) {
                        character.module = next.clone();
                        character.module.redraw(ui, ctx);
                        character.module.show_settings(ui, ctx);
                        *CODEGRIDFX.write().unwrap() = character.module.clone();
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterUndo {
    pub stack: Vec<CharacterUndoAtom>,
    pub index: isize,
}

impl Default for CharacterUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterUndo {
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

    pub fn add(&mut self, atom: CharacterUndoAtom) {
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
