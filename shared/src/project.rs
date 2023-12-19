use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub regions: Vec<Region>,
    pub tilemaps: Vec<Tilemap>,

    #[serde(default)]
    pub characters: FxHashMap<Uuid, TheCodeBundle>,
    #[serde(default)]
    pub items: FxHashMap<Uuid, TheCodeBundle>,
    #[serde(default)]
    pub codes: FxHashMap<Uuid, TheCodeBundle>
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        Self {
            name: String::new(),

            regions: vec![],
            tilemaps: vec![],

            characters: FxHashMap::default(),
            items: FxHashMap::default(),
            codes: FxHashMap::default()
        }
    }

    /// Add Character
    pub fn add_character(&mut self, character: TheCodeBundle) {
        self.characters.insert(character.uuid, character);
    }

    /// Removes the given character from the project.
    pub fn remove_character(&mut self, id: &Uuid) {
        self.characters.remove(id);
    }

    /// Add Item
    pub fn add_item(&mut self, item: TheCodeBundle) {
        self.items.insert(item.uuid, item);
    }

    /// Removes the given item from the project.
    pub fn remove_item(&mut self, id: &Uuid) {
        self.items.remove(id);
    }

    /// Add a tilemap
    pub fn add_tilemap(&mut self, tilemap: Tilemap) {
        self.tilemaps.push(tilemap)
    }

    /// Get the tilemap of the given uuid.
    pub fn get_tilemap(&mut self, uuid: Uuid) -> Option<&mut Tilemap> {
        self.tilemaps.iter_mut().find(|t| t.id == uuid)
    }

    /// Removes the given tilemap from the project.
    pub fn remove_tilemap(&mut self, id: TheId) {
        self.tilemaps.retain(|item| item.id != id.uuid);
    }

    /// Get the region of the given uuid.
    pub fn get_region(&self, uuid: &Uuid) -> Option<&Region> {
        self.regions.iter().find(|t| t.id == *uuid)
    }

    /// Get the region of the given uuid as mutable.
    pub fn get_region_mut(&mut self, uuid: &Uuid) -> Option<&mut Region> {
        self.regions.iter_mut().find(|t| t.id == *uuid)
    }

    /// Add Code
    pub fn add_code(&mut self, code: TheCodeBundle) {
        self.codes.insert(code.uuid, code);
    }

    /// Removes the given code from the project.
    pub fn remove_code(&mut self, id: &Uuid) {
        self.codes.remove(id);
    }

    /// Extract all tiles from all tilemaps and store them in a hash.
    pub fn extract_tiles(&self) -> FxHashMap<Uuid, TheRGBATile> {
        let mut tiles = FxHashMap::default();
        for tilemap in &self.tilemaps {
            for tile in &tilemap.tiles {
                let mut rgba_tile = TheRGBATile::new();
                rgba_tile.id = tile.id;
                rgba_tile.buffer = tilemap.buffer.extract_sequence(&tile.sequence);
                rgba_tile.role = tile.role as u8;
                rgba_tile.blocking = tile.blocking;
                tiles.insert(tile.id, rgba_tile);
            }
        }
        tiles
    }
}
