use serde::{Deserialize, Serialize};
//use serde_json::to_string;

use std::fs;
use std::fs::File;
use std::path;
use std::fs::metadata;

use std::collections::HashMap;
use std::path::PathBuf;
use rand::prelude::*;

// Tile implementation

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum TileUsage {
    Unused,
    Environment,
    EnvRoad,
    EnvBlocking,
    Character,
    UtilityChar,
    Water,
    Effect,
    Icon,
    UIElement,
}

#[derive(Serialize, Deserialize)]
pub struct Tile {
    pub usage               : TileUsage,
    pub anim_tiles          : Vec<(usize, usize)>,
    pub tags                : String,
    pub role                : usize,
}

// TileMap implementation

#[derive(Serialize, Deserialize)]
pub struct TileMapSettings {
    pub grid_size       : usize,
    #[serde(with = "vectorize")]
    pub tiles           : HashMap<(usize, usize), Tile>,
    pub id              : usize,
    pub default_tile    : Option<(usize, usize)>
}

pub struct TileMap {
    pub base_path       : PathBuf,
    pub pixels          : Vec<u8>,
    pub file_path       : PathBuf,
    pub width           : usize,
    pub height          : usize,
    pub settings        : TileMapSettings,
}

impl TileMap {
    fn new(file_name: &PathBuf, base_path: &PathBuf) -> TileMap {

        fn load(file_name: &PathBuf) -> (Vec<u8>, u32, u32) {

            let decoder = png::Decoder::new(File::open(file_name).unwrap());
            if let Ok(mut reader) = decoder.read_info() {
                let mut buf = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut buf).unwrap();
                let bytes = &buf[..info.buffer_size()];

                return (bytes.to_vec(), info.width, info.height);
            }
            (vec![], 0 , 0)
        }

        // Load the atlas pixels
        let info = load(file_name);

        // Gets the content of the settings file
        let name = path::Path::new(&file_name).file_stem().unwrap().to_str().unwrap();
        let json_path = path::Path::new(base_path).join("assets").join("tilemaps").join( format!("{}{}", name, ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let settings = serde_json::from_str(&contents)
            .unwrap_or(TileMapSettings { grid_size: 16, tiles: HashMap::new(), id: thread_rng().gen_range(1..=u32::MAX) as usize, default_tile: None } );

        TileMap {
            base_path       : base_path.clone(),
            pixels          : info.0,
            file_path       : file_name.to_path_buf(),
            width           : info.1 as usize,
            height          : info.2 as usize,
            settings
        }
    }

    /// Get the tile for the given id
    pub fn get_tile(&self, tile_id: &(usize, usize)) -> Tile {
        if let Some(t) = self.settings.tiles.get(&tile_id) {
            Tile { usage: t.usage.clone(), anim_tiles: t.anim_tiles.clone(), tags: t.tags.clone(), role: t.role.clone() }
        } else {
            Tile { usage: TileUsage::Environment, anim_tiles: vec![], tags: "".to_string(), role: 0 }
        }
    }

    /// Set the tile for the given id
    pub fn set_tile(&mut self, tile_id: (usize, usize), tile: Tile) {
        self.settings.tiles.insert(tile_id, tile);
    }

    /// Returns the name of the tilemap
    pub fn get_name(&self) -> String {
        path::Path::new(&self.file_path).file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Save the TileMapSettings to file
    pub fn save_settings(&self) {
        let name = path::Path::new(&self.file_path).file_stem().unwrap().to_str().unwrap();
        let json_path = self.base_path.join("assets").join("tilemaps").join( format!("{}{}", name, ".json"));

        let json = serde_json::to_string(&self.settings).unwrap();
        fs::write(json_path, json)
           .expect("Unable to write file");
    }

    /// Returns the amount of tiles for this tilemap
    pub fn max_tiles(&self) -> usize {
        (self.width / self.settings.grid_size) * (self.height / self.settings.grid_size)
    }

    /// Returns the amount of tiles per row
    pub fn max_tiles_per_row(&self) -> usize {
        self.width / self.settings.grid_size
    }

    /// Returns the amount of tiles for this tilemap
    pub fn offset_to_id(&self, offset: usize) -> (usize, usize) {
        (offset % (self.width / self.settings.grid_size), offset / (self.width / self.settings.grid_size))
    }
}

/// The TileSet struct consists of several TileMaps, each representing one atlas and it's tiles.
pub struct TileSet {
    pub maps            : HashMap<usize, TileMap>,
    pub maps_names      : Vec<String>,
    pub maps_ids        : Vec<usize>,
}

impl TileSet {

    pub fn load_from_path(base_path: PathBuf) -> TileSet {

        let mut maps : HashMap<usize, TileMap> = HashMap::new();

        let tilemaps_path = base_path.join("assets").join("tilemaps");
        //let paths = fs::read_dir(tilemaps_path).unwrap();

        let mut paths: Vec<_> = fs::read_dir(tilemaps_path).unwrap()
                                                .map(|r| r.unwrap())
                                                .collect();
        paths.sort_by_key(|dir| dir.path());

        let mut maps_names  : Vec<String> = vec![];
        let mut maps_ids    : Vec<usize> = vec![];

        for path in paths {

            // Generate the tile map for this dir element
            let path = &path.path();
            let md = metadata(path).unwrap();

            if md.is_file() {
                if let Some(name) = path::Path::new(&path).extension() {
                    if name == "png" || name == "PNG" {

                        let mut tile_map = TileMap::new(&path, &base_path);
                        if tile_map.width != 0 {
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

                            maps_ids.push(tile_map.settings.id);

                            // If the tilemap has no tiles we assume it's new and we save the settings
                            if tile_map.settings.tiles.len() == 0 {
                                tile_map.save_settings();
                            }

                            // Insert the tilemap
                            maps.insert(tile_map.settings.id, tile_map);
                        }
                    }
                }
            }
        }

        TileSet {
            maps,
            maps_names,
            maps_ids
        }
    }

    pub fn new() -> Self {

        let maps        : HashMap<usize, TileMap> = HashMap::new();
        let maps_names  : Vec<String> = vec![];
        let maps_ids    : Vec<usize> = vec![];

        Self {
            maps,
            maps_names,
            maps_ids
        }
    }
}