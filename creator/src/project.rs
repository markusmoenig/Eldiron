use crate::{prelude::*, tilemap::Tilemap};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub regions: Vec<Region>,
    pub tilemaps: Vec<Tilemap>,
}

impl Project {
    pub fn default() -> Self {
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
        for t in &mut self.tilemaps {
            if t.id == uuid {
                return Some(t);
            }
        }
        None
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
