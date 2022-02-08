use serde::{Deserialize, Serialize};
//use serde_json::to_string;

use std::fs;
use std::fs::File;
use std::path;

use std::collections::HashMap;
use std::path::PathBuf;

// Tile implementation

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum TileUsage {
    Unused,
    Environment,
    EnvBlocking,
    Character,
    UtilityChar,
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
    #[serde(with = "vectorize")]
    pub tiles           : HashMap<(u32, u32), Tile>,
    pub id              : u32,
}

pub struct TileMap {
    pub pixels          : Vec<u8>,
    pub file_path       : PathBuf,
    pub width           : u32,
    pub height          : u32,
    pub settings        : TileMapSettings,
}

impl TileMap {
    fn new(file_name: &PathBuf) -> TileMap {

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

        TileMap {
            pixels          : info.0,
            file_path       : file_name.to_path_buf(),
            width           : info.1,
            height          : info.2,
            settings
        }
    }

    /// Get the tile for the given id
    pub fn get_tile(&self, tile_id: &(u32, u32)) -> Tile {
        if let Some(t) = self.settings.tiles.get(&tile_id) {
            Tile { usage: t.usage.clone(), anim_tiles: t.anim_tiles.clone() }
        } else {
            Tile { usage: TileUsage::Environment, anim_tiles: vec![] }
        }
    }

    /// Set the tile for the given id
    pub fn set_tile(&mut self, tile_id: (u32, u32), tile: Tile) {
        self.settings.tiles.insert(tile_id, tile);
    }

    /// Returns the name of the tilemap
    pub fn get_name(&self) -> String {
        path::Path::new(&self.file_path).file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Save the TileMapSettings to file
    pub fn save_settings(&self) {

        let name = path::Path::new(&self.file_path).file_stem().unwrap().to_str().unwrap();
        let json_path = path::Path::new("assets").join("json").join( format!("{}{}", name, ".json"));

        let json = serde_json::to_string(&self.settings).unwrap();
        fs::write(json_path, json)
           .expect("Unable to write file");
    }

    /// Returns the amount of tiles for this tilemap
    pub fn max_tiles(&self) -> u32 {
        (self.width / self.settings.grid_size) * (self.height / self.settings.grid_size)
    }

    /// Returns the amount of tiles for this tilemap
    pub fn offset_to_id(&self, offset: u32) -> (u32, u32) {
        (offset % (self.width / self.settings.grid_size), offset / (self.width / self.settings.grid_size))
    }
}

/// The TileSet struct consists of several TileMaps, each representing one atlas and it's tiles.
pub struct TileSet {
    pub maps            : HashMap<u32, TileMap>,
    pub maps_names      : Vec<String>,
}

impl TileSet {
    pub fn new() -> TileSet {

        let mut maps : HashMap<u32, TileMap> = HashMap::new();

        let tilemaps_path = path::Path::new("assets").join("tilemaps");
        let paths = fs::read_dir(tilemaps_path).unwrap();

        let mut maps_names : Vec<String> = vec![];

        for path in paths {
            // Generate the tile map for this dir element
            let mut tile_map = TileMap::new(&path.unwrap().path());
            maps_names.push(tile_map.get_name());

            // Make sure we create a unique id (check if the id already exists in the set)
            let mut has_id_already = true;
            while has_id_already {

                has_id_already = false;
                for (key, _value) in &maps {
                    if key == &tile_map.settings.id {
                        has_id_already = true;
                    }
                }

                if has_id_already {
                    tile_map.settings.id += 1;
                }
            }

            // If the tilemap has no tiles we assume it's new and we save the settings
            if tile_map.settings.tiles.len() == 0 {
                tile_map.save_settings();
            }

            // Insert the tilemap
            maps.insert(tile_map.settings.id, tile_map);
        }

        TileSet {
            maps,
            maps_names,
        }
    }

    /*
    /// Returns the tile of the given map id
    pub fn get_tile(&self, map_id: (u32, u32, u32)) -> Option<Tile> {
        if let Some(map) = self.maps.get(&map_id.0) {
            return Some(map.get_tile((map_id.1, map_id.2)));
        }
        None
    }*/
}