use serde::{Deserialize, Serialize};

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;

use crate::asset::tileset::TileUsage;
use crate::asset::Asset;

#[derive(Serialize, Deserialize)]
pub struct GameRegionData {
    #[serde(with = "vectorize")]
    pub tiles           : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    pub id              : usize,
    pub curr_pos        : (isize, isize),
    pub min_pos         : (isize, isize),
    pub max_pos         : (isize, isize),
}

pub struct GameRegion {
    pub name            : String,
    pub path            : PathBuf,
    pub data            : GameRegionData,
}

impl GameRegion {
    pub fn new(path: &PathBuf) -> Self {

        let name = path::Path::new(&path).file_stem().unwrap().to_str().unwrap();

        // Gets the content of the settings file
        let json_path = path.join( format!("{}{}", "level0", ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let data = serde_json::from_str(&contents)
            .unwrap_or(GameRegionData { tiles: HashMap::new(), id: 0, curr_pos: (0,0), min_pos: (10000,10000), max_pos: (-10000, -10000) });

        Self {
            name        : name.to_string(),
            path        : path.clone(),
            data,
        }
    }

    /// Save the TileAreaData to file
    pub fn save_data(&self) {
        let json_path = self.path.join( format!("{}{}", "level0", ".json"));
        let json = serde_json::to_string(&self.data).unwrap();
        fs::write(json_path, json)
            .expect("Unable to write area file");
    }

    /// Returns an optional tile value at the given position
    pub fn get_value(&self, pos: (isize, isize)) -> Option<&(usize, usize, usize, TileUsage)> {
        self.data.tiles.get(&pos)
    }

    /// Sets a value at the given position
    pub fn set_value(&mut self, pos: (isize, isize), value: (usize, usize, usize, TileUsage)) {
        self.data.tiles.insert(pos, value);

        if self.data.min_pos.0 > pos.0 {
            self.data.min_pos.0 = pos.0;
        }
        if self.data.min_pos.1 > pos.1 {
            self.data.min_pos.1 = pos.1;
        }
        if self.data.max_pos.0 < pos.0 {
            self.data.max_pos.0 = pos.0;
        }
        if self.data.max_pos.1 < pos.1 {
            self.data.max_pos.1 = pos.1;
        }
    }

    /// Calculates the min / max positions
    pub fn calc_dimensions(&mut self) {
        let mut min_pos = (10000, 10000);
        let mut max_pos = (-10000, -10000);

        for (pos, _tile)  in &self.data.tiles {
            if min_pos.0 > pos.0 {
                min_pos.0 = pos.0;
            }
            if min_pos.1 > pos.1 {
                min_pos.1 = pos.1;
            }
            if max_pos.0 < pos.0 {
                max_pos.0 = pos.0;
            }
            if max_pos.1 < pos.1 {
                max_pos.1 = pos.1;
            }
        }

        self.data.min_pos = min_pos;
        self.data.max_pos = max_pos;
    }

    /// Calculates the offset for center of the area for the given visible size
    pub fn get_center_offset_for_visible_size(&self, visible_tiles: (usize, usize)) -> (isize, isize) {
        let x = self.data.min_pos.0 + (self.data.max_pos.0 - self.data.min_pos.0) / 2 - visible_tiles.0 as isize / 2;
        let y = self.data.min_pos.1 + (self.data.max_pos.1 - self.data.min_pos.1) / 2 - visible_tiles.1 as isize / 2;
        (x, y)
    }

    /// Remaps the TileUsage field of the tiles
    pub fn remap(&mut self, asset: &mut Asset) {
        let mut tiles : HashMap<(isize, isize), (usize, usize, usize, TileUsage)> = HashMap::new();
        let ids: Vec<&(isize, isize)> = self.data.tiles.keys().collect();
        for id in &ids {
            let value = &self.data.tiles[id];
            let tile = asset.get_tile(&(value.0, value.1, value.2));

            tiles.insert(**id, (value.0, value.1, value.2, tile.usage));
        }
        self.data.tiles = tiles;
        self.save_data();
    }
}