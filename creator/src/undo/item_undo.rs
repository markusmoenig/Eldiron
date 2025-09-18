use crate::editor::{CODEGRIDFX, NODEEDITOR};
use crate::prelude::*;
use codegridfx::Module;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ItemUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
    TemplateModuleEdit(Uuid, Module, Module),
    InstanceModuleEdit(Uuid, Uuid, Module, Module),
}

impl ItemUndoAtom {
    pub fn undo(&self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            ItemUndoAtom::MapEdit(prev, _) => {
                for item in project.items.values_mut() {
                    if item.map.id == prev.id {
                        item.map = *prev.clone();
                        item.map.clear_temp();
                    }
                }
                NODEEDITOR
                    .write()
                    .unwrap()
                    .set_selected_node_ui(project, ui, ctx, false);
            }
            ItemUndoAtom::TemplateModuleEdit(id, prev, _) => {
                if let Some(item) = project.items.get_mut(id) {
                    item.module = prev.clone();
                    item.module.redraw(ui, ctx);
                    item.module.show_settings(ui, ctx);
                    *CODEGRIDFX.write().unwrap() = item.module.clone();
                }
            }
            ItemUndoAtom::InstanceModuleEdit(region_id, id, prev, _) => {
                if let Some(region) = project.get_region_mut(region_id) {
                    if let Some(item) = region.items.get_mut(id) {
                        item.module = prev.clone();
                        item.module.redraw(ui, ctx);
                        item.module.show_settings(ui, ctx);
                        *CODEGRIDFX.write().unwrap() = item.module.clone();
                    }
                }
            }
        }
    }
    pub fn redo(&self, project: &mut Project, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            ItemUndoAtom::MapEdit(_, next) => {
                for item in project.items.values_mut() {
                    if item.map.id == next.id {
                        item.map = *next.clone();
                        item.map.clear_temp();
                    }
                }
                NODEEDITOR
                    .write()
                    .unwrap()
                    .set_selected_node_ui(project, ui, ctx, false);
            }
            ItemUndoAtom::TemplateModuleEdit(id, _, next) => {
                if let Some(item) = project.items.get_mut(id) {
                    item.module = next.clone();
                    item.module.redraw(ui, ctx);
                    item.module.show_settings(ui, ctx);
                    *CODEGRIDFX.write().unwrap() = item.module.clone();
                }
            }
            ItemUndoAtom::InstanceModuleEdit(region_id, id, _, next) => {
                if let Some(region) = project.get_region_mut(region_id) {
                    if let Some(item) = region.items.get_mut(id) {
                        item.module = next.clone();
                        item.module.redraw(ui, ctx);
                        item.module.show_settings(ui, ctx);
                        *CODEGRIDFX.write().unwrap() = item.module.clone();
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemUndo {
    pub stack: Vec<ItemUndoAtom>,
    pub index: isize,
}

impl Default for ItemUndo {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemUndo {
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

    pub fn add(&mut self, atom: ItemUndoAtom) {
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
