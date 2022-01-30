use serde::{Deserialize, Serialize};

use std::fs;
use std::fs::File;
use std::path;

use std::collections::HashMap;

// Tile implementation

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum TileUsage {
    Unused,
    Environment,
    EnvBlocking,
    Character,
    Water,
    Harmful,
}

#[derive(Serialize, Deserialize)]
pub struct Tile {
    pub usage               : TileUsage,
    pub anim_tiles          : Vec<(u32, u32)>
}

// TileMap implementation

#[derive(Serialize, Deserialize)]
pub struct TileMapSettings {
    pub grid_size       : u32,
    pub tiles           : HashMap<(u32, u32), Tile>,
    pub id              : u32,
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
            .unwrap_or(TileMapSettings { grid_size: 16, tiles: HashMap::new(), id: 0 } );

        TileMap {
            pixels          : info.0,
            file_name       : file_name.to_string(),
            width           : info.1,
            height          : info.2,
            settings
        }
    }

    /// Get the tile for the given id
    pub fn get_tile(&self, tile_id: (u32, u32)) -> Tile {
        if let Some(t) = self.settings.tiles.get(&tile_id) {
            Tile { usage: t.usage.clone(), anim_tiles: t.anim_tiles.clone() }
        } else {
            Tile { usage: TileUsage::Environment, anim_tiles: vec![] }
        }
    }

    /// Set the tile for the given id
    pub fn set_tile(&mut self, tile_id: (u32, u32), tile_settings: Tile) {
        //self.settings.tiles[&tile_id] = tile_settings;
        self.settings.tiles.insert(tile_id, tile_settings);
        //let t = &self.settings.tiles[&tile_id];
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

        let ts1b = TileMap::new("assets/ts1b.png");

        maps.insert(ts1b.settings.id, ts1b);

        maps[&0].save_settings();

        TileSet {
            maps
        }
    }

    /*
    pub fn set_tile(&mut self, map_id: u32, tile_id: (u32, u32), tile: Tile) {
        let map = &mut self.maps.get_mut(&map_id).unwrap();        
        map.set_tile(tile_id, tile);
    }*/
}