use crate::{prelude::*, tilemap::Tilemap};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub tilemaps: Vec<Tilemap>,
}

impl Project {
    pub fn default() -> Self {
        Self {
            name: String::new(),

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
}