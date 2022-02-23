pub mod area;

use std::collections::HashMap;

use crate::gamedata::area::GameArea;
use crate::asset::TileUsage;

use std::path;
use std::fs;

pub struct GameData {
    pub areas                   : HashMap<usize, GameArea>,
}

impl GameData {

    pub fn new() -> Self {

        // Create the tile areas
        let mut areas: HashMap<usize, GameArea> = HashMap::new();

        let tilemaps_path = path::Path::new("game").join("areas");
        let paths = fs::read_dir(tilemaps_path).unwrap();

        //let mut maps_names : Vec<String> = vec![];

        for path in paths {
            let mut area = GameArea::new(&path.unwrap().path());

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

            areas.insert(area.data.id, area);
        }

        for (_, area) in &areas {
            // If the area has no tiles we assume it's new and we save the data
            if area.data.tiles.len() == 0 {
                area.save_data();
            }
        }

        Self {
            areas
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