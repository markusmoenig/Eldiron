use crate::editor::{SCENEMANAGER, SHADEGRIDFX};
use crate::prelude::*;
use crate::undo::character_undo::CharacterUndoAtom;
use crate::undo::item_undo::ItemUndoAtom;
use crate::undo::screen_undo::ScreenUndoAtom;
use codegridfx::Module;
use rusterix::TerrainChunk;
use theframework::prelude::*;

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RegionUndoAtom {
    MapEdit(Box<Map>, Box<Map>),
    SectorShaderEdit(Uuid, u32, Module, Module),
    TerrainEdit(
        Box<FxHashMap<(i32, i32), TerrainChunk>>,
        Box<FxHashMap<(i32, i32), TerrainChunk>>,
    ),
}

impl RegionUndoAtom {
    // pub fn to_material_atom(self) -> Option<MaterialUndoAtom> {
    //     match self {
    //         RegionUndoAtom::MapEdit(map1, map2) => Some(MaterialUndoAtom::MapEdit(map1, map2)),
    //         _ => None,
    //     }
    // }
    pub fn to_character_atom(self) -> Option<CharacterUndoAtom> {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => Some(CharacterUndoAtom::MapEdit(map1, map2)),
            _ => None,
        }
    }
    pub fn to_item_atom(self) -> Option<ItemUndoAtom> {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => Some(ItemUndoAtom::MapEdit(map1, map2)),
            _ => None,
        }
    }
    pub fn to_screen_atom(self) -> Option<ScreenUndoAtom> {
        match self {
            RegionUndoAtom::MapEdit(map1, map2) => Some(ScreenUndoAtom::MapEdit(map1, map2)),
            _ => None, // Return None for unsupported variants
        }
    }

    pub fn undo(&self, region: &mut Region, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            RegionUndoAtom::MapEdit(prev, _) => {
                let map = if region.map.id == prev.id {
                    Some(&mut region.map)
                } else {
                    region.map.profiles.get_mut(&prev.id)
                };
                if let Some(map) = map {
                    *map = *prev.clone();
                    map.clear_temp();
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimaps"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Render SceneManager Map"),
                        TheValue::Empty,
                    ));
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            RegionUndoAtom::TerrainEdit(prev, _) => {
                let array = prev.values().cloned().collect();
                SCENEMANAGER
                    .write()
                    .unwrap()
                    .set_dirty_terrain_chunks(array);
                region.map.terrain.chunks = *prev.clone();
            }
            RegionUndoAtom::SectorShaderEdit(map_id, _, prev, _) => {
                let map = if region.map.id == *map_id {
                    Some(&mut region.map)
                } else {
                    region.map.profiles.get_mut(&prev.id)
                };
                if let Some(map) = map {
                    if let Some(module) = map.shaders.get_mut(&prev.id) {
                        *module = prev.clone();
                        let mut shadergridfx = SHADEGRIDFX.write().unwrap();
                        *shadergridfx = prev.clone();
                        shadergridfx.redraw(ui, ctx);
                        shadergridfx.show_settings(ui, ctx);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Render SceneManager Map"),
                            TheValue::Empty,
                        ));
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    }
                }
            }
        }
    }
    pub fn redo(&self, region: &mut Region, ui: &mut TheUI, ctx: &mut TheContext) {
        match self {
            RegionUndoAtom::MapEdit(_, next) => {
                let map = if region.map.id == next.id {
                    Some(&mut region.map)
                } else {
                    region.map.profiles.get_mut(&next.id)
                };

                if let Some(map) = map {
                    *map = *next.clone();
                    map.clear_temp();
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimaps"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Render SceneManager Map"),
                        TheValue::Empty,
                    ));
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            RegionUndoAtom::TerrainEdit(_, next) => {
                let array = next.values().cloned().collect();
                SCENEMANAGER
                    .write()
                    .unwrap()
                    .set_dirty_terrain_chunks(array);
                region.map.terrain.chunks = *next.clone();
            }
            RegionUndoAtom::SectorShaderEdit(map_id, _, _, next) => {
                let map = if region.map.id == *map_id {
                    Some(&mut region.map)
                } else {
                    region.map.profiles.get_mut(&next.id)
                };
                if let Some(map) = map {
                    if let Some(module) = map.shaders.get_mut(&next.id) {
                        *module = next.clone();
                        let mut shadergridfx = SHADEGRIDFX.write().unwrap();
                        *shadergridfx = next.clone();
                        shadergridfx.redraw(ui, ctx);
                        shadergridfx.show_settings(ui, ctx);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Render SceneManager Map"),
                            TheValue::Empty,
                        ));
                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    }
                }
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
