use serde::{Deserialize, Serialize};

// use std::fs;
// use std::fs::File;
// use std::path;

use std::collections::HashMap;
//use std::path::PathBuf;

use crate::asset::tileset::TileUsage;

#[derive(Serialize, Deserialize)]
pub struct TileArea {
    pub tiles           : HashMap<(isize, isize), (u32, u32, u32, TileUsage)>,
    pub curr_pos        : (isize, isize),
    pub min_pos         : (isize, isize),
    pub max_pos         : (isize, isize),
}

impl TileArea {
    pub fn new() -> TileArea {

        /*
        fn load(file_name: &PathBuf) -> (Vec<u8>, u32, u32) {

            let decoder = png::Decoder::new(File::open(file_name).unwrap());
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            let bytes = &buf[..info.buffer_size()];
    
            (bytes.to_vec(), info.width, info.height)
        }

        // Load the atlas pixels
        let info = load(file_name);

        // Gets the content of the settings file
        let name = path::Path::new(&file_name).file_stem().unwrap().to_str().unwrap();
        let json_path = path::Path::new("assets").join("json").join( format!("{}{}", name, ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let settings = serde_json::from_str(&contents)
            .unwrap_or(TileMapSettings { grid_size: 16, tiles: HashMap::new(), id: 0 } );
        */

        let tiles = HashMap::new();

        TileArea {
            tiles,
            curr_pos            : (0, 0),
            min_pos             : (0, 0),
            max_pos             : (0, 0),
        }
    }

    /// Returns an optional tile value at the given position
    pub fn get_value(&self, pos: (isize, isize)) -> Option<&(u32, u32, u32, TileUsage)> {
        self.tiles.get(&pos)
    }

    /// Sets a value at the given position
    pub fn set_value(&mut self, pos: (isize, isize), value: (u32, u32, u32, TileUsage)) {
        self.tiles.insert(pos, value);

        if self.min_pos.0 > pos.0 {
            self.min_pos.0 = pos.0;
        }
        if self.min_pos.1 > pos.1 {
            self.min_pos.1 = pos.1;
        }
        if self.max_pos.0 < pos.0 {
            self.max_pos.0 = pos.0;
        }    
        if self.max_pos.1 < pos.1 {
            self.max_pos.1 = pos.1;
        }              
    }
}