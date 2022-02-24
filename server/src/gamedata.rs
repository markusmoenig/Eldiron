pub mod area;

use std::collections::HashMap;

use crate::gamedata::area::GameArea;
use crate::asset::TileUsage;

use itertools::Itertools;

use std::path;
use std::fs;

pub struct GameData {
    pub areas                   : HashMap<usize, GameArea>,
    pub areas_names             : Vec<String>,
    pub areas_ids               : Vec<usize>,
}

impl GameData {

    pub fn new() -> Self {

        // Create the tile areas
        let mut areas: HashMap<usize, GameArea> = HashMap::new();
        let mut areas_names = vec![];
        let mut areas_ids = vec![];

        let tilemaps_path = path::Path::new("game").join("areas");
        let paths = fs::read_dir(tilemaps_path).unwrap();

        for path in paths {
            let mut area = GameArea::new(&path.unwrap().path());

            areas_names.push(area.name.clone());

            // Make sure we create a unique id (check if the id already exists in the set)
            let mut has_id_already = true;
            while has_id_already {

                has_id_already = false;
                for (key, _value) in &areas {
                    if key == &area.data.id {
                        has_id_already = true;
                    }
                }

                if has_id_already {
                    area.data.id += 1;
                }
            }

            areas_ids.push(area.data.id);
            areas.insert(area.data.id, area);
        }

        let sorted_keys= areas.keys().sorted();
        for key in sorted_keys {
            let area = &areas[key];

            // If the area has no tiles we assume it's new and we save the data
            if area.data.tiles.len() == 0 {
                area.save_data();
            }
        }

        Self {
            areas,
            areas_names,
            areas_ids
        }
    }

    /// Sets a value in the current area
    pub fn save_area(&self, id: usize) {
        let area = &mut self.areas.get(&id).unwrap();
        area.save_data();
    }

    /// Sets a value in the area
    pub fn set_area_value(&mut self, id: usize, pos: (isize, isize), value: (usize, usize, usize, TileUsage)) {
        let area = &mut self.areas.get_mut(&id).unwrap();
        area.set_value(pos, value);
    }
}