use crate::{Map, Tile};
use theframework::prelude::*;

/// Holds a map and all its associated meta data (tiles, audio etc).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapMeta {
    pub map: Map,
    pub tiles: FxHashMap<Uuid, Tile>,
}

impl MapMeta {
    pub fn new(map: Map, tiles: FxHashMap<Uuid, Tile>) -> Self {
        Self { map, tiles }
    }
}
