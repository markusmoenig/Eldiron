use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub regions: Vec<Region>,
    pub tilemaps: Vec<Tilemap>,
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
        }
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
    pub fn get_region(&mut self, uuid: Uuid) -> Option<&mut Region> {
        self.regions.iter_mut().find(|t| t.id == uuid)
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
