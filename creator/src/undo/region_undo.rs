use crate::prelude::*;
use theframework::prelude::*;

use crate::editor::PRERENDERTHREAD;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionUndoAtom {
    GeoFXObjectsDeletion(Vec<GeoFXObject>, Vec<Vec2i>),
    GeoFXObjectEdit(Uuid, Option<GeoFXObject>, Option<GeoFXObject>, Vec<Vec2i>),
    RegionTileEdit(Vec2i, Option<RegionTile>, Option<RegionTile>),
}

impl RegionUndoAtom {
    pub fn undo(&self, region: &mut Region) {
        match self {
            RegionUndoAtom::GeoFXObjectsDeletion(objects, tiles) => {
                for object in objects {
                    region.geometry.insert(object.id, object.clone());
                }
                region.update_geometry_areas();
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region.clone(), Some(tiles.clone()));
            }
            RegionUndoAtom::GeoFXObjectEdit(id, prev, _, tiles) => {
                if let Some(prev) = prev {
                    region.geometry.insert(*id, prev.clone());
                } else {
                    region.geometry.remove(id);
                }
                region.update_geometry_areas();
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region.clone(), Some(tiles.clone()));
            }
            RegionUndoAtom::RegionTileEdit(pos, prev, _) => {
                if let Some(prev) = prev {
                    region.tiles.insert((pos.x, pos.y), prev.clone());
                } else {
                    region.tiles.remove(&(pos.x, pos.y));
                }
            }
        }
    }
    pub fn redo(&self, region: &mut Region) {
        match self {
            RegionUndoAtom::GeoFXObjectsDeletion(objects, tiles) => {
                for object in objects {
                    region.geometry.remove(&object.id);
                }
                region.update_geometry_areas();
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region.clone(), Some(tiles.clone()));
            }

            RegionUndoAtom::GeoFXObjectEdit(id, _, next, tiles) => {
                if let Some(next) = next {
                    region.geometry.insert(*id, next.clone());
                } else {
                    region.geometry.remove(id);
                }
                region.update_geometry_areas();
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region.clone(), Some(tiles.clone()));
            }
            RegionUndoAtom::RegionTileEdit(pos, _, next) => {
                if let Some(next) = next {
                    region.tiles.insert((pos.x, pos.y), next.clone());
                } else {
                    region.tiles.remove(&(pos.x, pos.y));
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
}
