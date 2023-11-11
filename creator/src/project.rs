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
}