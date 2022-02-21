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
    pub fn set_area_value(&mut self, id: usize, pos: (isize, isize), value: (u32, u32, u32, TileUsage)) {
        let area = &mut self.areas.get_mut(&id).unwrap();
        area.set_value(pos, value);
    }

    /*
    /// Draw the given area
    pub fn draw_area(&self, frame: &mut [u8], rect: &(u32,u32,u32,u32), anim_counter: u32) {
        let area = self.areas.get(&self.curr_area).unwrap();

        let x_tiles = (rect.2 / self.grid_size) as isize;
        let y_tiles = (rect.3 / self.grid_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                if let Some(value) = area.get_value((x, y)) {
                    let pos = (rect.0 + (x as u32) * self.grid_size, rect.1 + (y as u32) * self.grid_size);
                    self.draw_animated_tile(frame, &pos, value.0, &(value.1, value.2), anim_counter,self.grid_size);
                }
            }
        }
    }*/
}