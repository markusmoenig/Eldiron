use serde::{Deserialize, Serialize};

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;

use crate::asset::tileset::TileUsage;

#[derive(Serialize, Deserialize)]
pub struct GameAreaData {
    #[serde(with = "vectorize")]
    pub tiles           : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    pub instance_ids    : Vec<usize>,
    pub id              : usize,
    pub curr_pos        : (isize, isize),
    pub min_pos         : (isize, isize),
    pub max_pos         : (isize, isize),
}

pub struct GameArea {
    pub name            : String,
    pub path            : PathBuf,
    pub data            : GameAreaData,
}

impl GameArea {
    pub fn new(path: &PathBuf) -> Self {

        let name = path::Path::new(&path).file_stem().unwrap().to_str().unwrap();

        // Gets the content of the settings file
        let json_path = path.join( format!("{}{}", name, ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let data = serde_json::from_str(&contents)
            .unwrap_or(GameAreaData { tiles: HashMap::new(), instance_ids: vec![], id: 0, curr_pos: (0,0), min_pos: (10000,10000), max_pos: (-10000, -10000) });

        Self {
            name        : name.to_string(),
            path        : path.clone(),
            data,
        }
    }

    /// Save the TileAreaData to file
    pub fn save_data(&self) {
        let json_path = self.path.join( format!("{}{}", self.name, ".json"));
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
}