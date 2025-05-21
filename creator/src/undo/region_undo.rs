use crate::editor::{SCENEMANAGER, WORLDEDITOR};
use crate::prelude::*;
use crate::undo::material_undo::MaterialUndoAtom;
use crate::undo::screen_undo::ScreenUndoAtom;
use rusterix::TerrainChunk;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RegionUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
    TerrainEdit(
        Box<FxHashMap<(i32, i32), TerrainChunk>>,
        Box<FxHashMap<(i32, i32), TerrainChunk>>,
    ),
}

impl RegionUndoAtom {
    pub fn only_selection_changed(&self) -> bool {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => {
                map1.selected_vertices != map2.selected_vertices
                    || map1.selected_linedefs != map2.selected_linedefs
                    || map1.selected_sectors != map2.selected_sectors
            }
            _ => false,
        }
    }

    pub fn to_material_atom(self) -> Option<MaterialUndoAtom> {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => Some(MaterialUndoAtom::MapEdit(map1, map2)),
            _ => None, // Return None for unsupported variants
        }
    }
    pub fn to_screen_atom(self) -> Option<ScreenUndoAtom> {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => Some(ScreenUndoAtom::MapEdit(map1, map2)),
            _ => None, // Return None for unsupported variants
        }
    }

    pub fn undo(&self, region: &mut Region, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            RegionUndoAtom::MapEdit(prev, _) => {
                region.map = *prev.clone();
                region.map.clear_temp();
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimaps"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));

                if !self.only_selection_changed() {
                    SCENEMANAGER.write().unwrap().set_map(region.map.clone());
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            RegionUndoAtom::TerrainEdit(prev, _) => {
                region.map.terrain.chunks = *prev.clone();
                region.map.terrain.mark_dirty();

                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                WORLDEDITOR.write().unwrap().first_draw = true;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            }
        }
    }
    pub fn redo(&self, region: &mut Region, _ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            RegionUndoAtom::MapEdit(_, next) => {
                region.map = *next.clone();
                region.map.clear_temp();
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimaps"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));

                if !self.only_selection_changed() {
                    SCENEMANAGER.write().unwrap().set_map(region.map.clone());
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            RegionUndoAtom::TerrainEdit(_, next) => {
                region.map.terrain.chunks = *next.clone();
                region.map.terrain.mark_dirty();

                crate::editor::RUSTERIX.write().unwrap().set_dirty();
                WORLDEDITOR.write().unwrap().first_draw = true;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionUndo {
    pub stack: Vec<RegionUndoAtom>,
    pub index: isize,
}

impl Default for RegionUndo {
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

    pub fn add(&mut self, atom: RegionUndoAtom) {
        let to_remove = self.stack.len() as isize - self.index - 1;
        for _i in 0..to_remove {
            self.stack.pop();
        }
        self.stack.push(atom);
        self.index += 1;
    }

    pub fn undo(&mut self, region: &mut Region, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index >= 0 {
            self.stack[self.index as usize].undo(region, ui, ctx);
            self.index -= 1;
        }
    }

    pub fn redo(&mut self, region: &mut Region, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.index < self.stack.len() as isize - 1 {
            self.index += 1;
            self.stack[self.index as usize].redo(region, ui, ctx);
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
