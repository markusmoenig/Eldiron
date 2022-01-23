use serde::{Deserialize, Serialize};

use std::fs;
use std::fs::File;
use std::path;

use std::collections::HashMap;

// Tile implementation

#[derive(Serialize, Deserialize)]
enum TileBrand {
    Env,
    EnvBlocking,
    Water
}

#[derive(Serialize, Deserialize)]
pub struct Tile {
    pos                 : [u32; 2],
    brand               : TileBrand,
}

// TileMap implementation

#[derive(Serialize, Deserialize)]
pub struct TileMapSettings {
    pub grid_size       : u32,
    pub tiles           : Vec<Tile>,
}

pub struct TileMap {
    pub pixels          : Vec<u8>,
    pub file_name       : String,
    pub width           : u32,
    pub height          : u32,
    pub settings        : TileMapSettings,
}

impl TileMap {
    fn new(file_name: &str) -> TileMap {

        fn load(file_name: &str) -> (Vec<u8>, u32, u32) {

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
        let json_path = path::Path::new("json").join( format!("{}{}", name, ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let settings = serde_json::from_str(&contents)
            .unwrap_or(TileMapSettings { grid_size: 16, tiles: vec!() } );

        TileMap {
            pixels          : info.0,
            file_name       : file_name.to_string(),
            width           : info.1,
            height          : info.2,
            settings
        }
    }

    /// Save the TileMapSettings to file
    pub fn save_settings(&self) {

        let name = path::Path::new(&self.file_name).file_stem().unwrap().to_str().unwrap();
        let json_path = path::Path::new("json").join( format!("{}{}", name, ".json"));

        let json = serde_json::to_string(&self.settings).unwrap();
        fs::write(json_path, json)
           .expect("Unable to write file");
    }
}

/// The TileSet struct consists of several TileMaps, each representing one atlas and it's tiles.
pub struct TileSet {

    pub maps            : HashMap<u32, TileMap>,
}

impl TileSet {
    pub fn new() -> TileSet {

        let mut maps : HashMap<u32, TileMap> = HashMap::new();
        maps.insert(0, TileMap::new("assets/ts1b.png"));

        maps[&0].save_settings();

        TileSet {
            maps
        }
    }
}