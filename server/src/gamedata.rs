pub mod area;
pub mod behavior;

use std::collections::HashMap;
use std::fs::metadata;

use crate::gamedata::area::GameArea;
use crate::gamedata::behavior::GameBehavior;
use crate::asset::TileUsage;

use itertools::Itertools;

use std::path;
use std::fs;

pub struct GameData {
    pub areas                   : HashMap<usize, GameArea>,
    pub areas_names             : Vec<String>,
    pub areas_ids               : Vec<usize>,

    pub behaviors               : HashMap<usize, GameBehavior>,
    pub behaviors_names         : Vec<String>,
    pub behaviors_ids           : Vec<usize>,
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
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_dir() {
                let mut area = GameArea::new(path);

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
        }

        let sorted_keys= areas.keys().sorted();
        for key in sorted_keys {
            let area = &areas[key];

            // If the area has no tiles we assume it's new and we save the data
            if area.data.tiles.len() == 0 {
                area.save_data();
            }
        }

        // Behaviors

        let behavior_path = path::Path::new("game").join("behavior");
        let paths = fs::read_dir(behavior_path).unwrap();

        let mut behaviors: HashMap<usize, GameBehavior> = HashMap::new();
        let mut behaviors_names = vec![];
        let mut behaviors_ids = vec![];

        for path in paths {
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_dir() {
                let mut behavior = GameBehavior::new(path);

                behaviors_names.push(behavior.name.clone());

                // Make sure we create a unique id (check if the id already exists in the set)
                let mut has_id_already = true;
                while has_id_already {

                    has_id_already = false;
                    for (key, _value) in &areas {
                        if key == &behavior.data.id {
                            has_id_already = true;
                        }
                    }

                    if has_id_already {
                        behavior.data.id += 1;
                    }
                }

                behaviors_ids.push(behavior.data.id);
                behaviors.insert(behavior.data.id, behavior);
            }
        }

        let sorted_keys= areas.keys().sorted();
        for key in sorted_keys {
            let area = &areas[key];

            // If the area has no tiles we assume it's new and we save the data
            if area.data.tiles.len() == 0 {
                //area.save_data();
            }
        }

        Self {
            areas,
            areas_names,
            areas_ids,

            behaviors,
            behaviors_names,
            behaviors_ids
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