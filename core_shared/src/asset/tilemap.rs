use crate::prelude::*;

use std::fs;
use std::fs::File;
use std::path;

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct Tile {
    pub usage               : TileUsage,
    pub anim_tiles          : Vec<(usize, usize)>,
    pub tags                : String,
    pub settings            : Option<PropertySink>,
}

impl Tile {
    pub fn new() -> Self {

        Self {
            usage       : TileUsage::Environment,
            anim_tiles  : vec![],
            tags        : "".to_string(),
            settings    : None
        }
    }
}

// TileMap implementation

#[derive(Serialize, Deserialize)]
pub struct TileMapSettings {
    pub grid_size       : usize,
    #[serde(with = "vectorize")]
    pub tiles           : FxHashMap<(u16, u16), Tile>,
    pub id              : Uuid,
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
    pub fn new(file_name: &PathBuf, base_path: &PathBuf) -> TileMap {

        fn load(file_name: &PathBuf) -> (Vec<u8>, u32, u32) {

            let decoder = png::Decoder::new(File::open(file_name).unwrap());
            if let Ok(mut reader) = decoder.read_info() {
                let mut buf = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut buf).unwrap();
                let bytes = &buf[..info.buffer_size()];

                return (bytes.to_vec(), info.width, info.height);
            }
            (vec![], 0, 0)
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
            .unwrap_or(TileMapSettings {
                grid_size: 16,
                tiles: HashMap::default(),
                id: Uuid::new_v4(),
                default_tile: None } );

        TileMap {
            base_path       : base_path.clone(),
            pixels          : info.0,
            file_path       : file_name.to_path_buf(),
            width           : info.1 as usize,
            height          : info.2 as usize,
            settings
        }
    }

    pub fn new_from_embedded(file_name: &str) -> TileMap {

        fn load(file_name: &str) -> (Vec<u8>, u32, u32) {

            if let Some(file) = Embedded::get(file_name) {

                let data = std::io::Cursor::new(file.data);

                let decoder = png::Decoder::new(data);
                if let Ok(mut reader) = decoder.read_info() {
                    let mut buf = vec![0; reader.output_buffer_size()];
                    let info = reader.next_frame(&mut buf).unwrap();
                    let bytes = &buf[..info.buffer_size()];

                    return (bytes.to_vec(), info.width, info.height);
                }
            }
            (vec![], 0 , 0)
        }

        let info = load(file_name);

        // Gets the content of the settings file
        let name = path::Path::new(&file_name).file_stem().unwrap().to_str().unwrap();
        let json_path = path::Path::new("").join("assets").join("tilemaps").join( format!("{}{}", name, ".json"));

        let mut contents = "".to_string();
        if let Some(bytes) = Embedded::get(json_path.to_str().unwrap()) {
            if let Some(string) = std::str::from_utf8(bytes.data.as_ref()).ok() {
                contents = string.to_string();
            }
        }

        // Construct the json settings
        let settings = serde_json::from_str(&contents)
            .unwrap_or(TileMapSettings {
                grid_size           : 16,
                tiles               : FxHashMap::default(),
                id                  : Uuid::new_v4(),
                default_tile        : None } );

        TileMap {
            base_path       : PathBuf::new(),
            pixels          : info.0,
            file_path       : std::path::Path::new(file_name).to_path_buf(),
            width           : info.1 as usize,//           : 800,//info.1 as usize,
            height          : info.2 as usize,//,//info.2 as usize,
            settings
        }
    }

    /// Get a reference to the tile of the given id
    pub fn get_tile(&self, tile_id: &(usize, usize)) -> Option<&Tile> {
        if let Some(tile) = self.settings.tiles.get(&(tile_id.0 as u16, tile_id.1 as u16)) {
            Some(tile)
        } else {
            None
        }
    }

    /// Get a reference to the tile of the given id
    pub fn get_tile_u16(&self, tile_id: &(u16, u16)) -> Option<&Tile> {
        if let Some(tile) = self.settings.tiles.get(&tile_id) {
            Some(tile)
        } else {
            None
        }
    }

    /// Get a mutable reference to the tile of the given id
    pub fn get_mut_tile(&mut self, tile_id: &(usize, usize)) -> Option<&mut Tile> {
        if let Some(tile) = self.settings.tiles.get_mut(&(tile_id.0 as u16, tile_id.1 as u16)) {
            Some(tile)
        } else {
            None
        }
    }

    /// Set the tile for the given id
    pub fn set_tile(&mut self, tile_id: (usize, usize), tile: Tile) {
        self.settings.tiles.insert((tile_id.0 as u16, tile_id.1 as u16), tile);
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

// Generate tile settings sink

pub fn update_tile_settings_sink(sink: &mut PropertySink) {

    if sink.contains("tags") == false {
        sink.properties.insert(0,Property::new_string("tags".to_string(), "".to_string()));
    }

    if sink.contains("raycaster_wall") == false {
        sink.properties.push(Property::new_string("raycaster_wall".to_string(), "wall".to_string()));
    }

    if sink.contains("raycaster_sprite_shrink") == false {
        sink.properties.push(Property::new_int("raycaster_sprite_shrink".to_string(), 1));
    }

    if sink.contains("raycaster_sprite_move_y") == false {
        sink.properties.push(Property::new_float("raycaster_sprite_move_y".to_string(), 0.0));
    }

}

pub fn generate_tile_settings_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map : FxHashMap<String, Vec<String>> = HashMap::default();

    map.insert("tags".to_string(), vec!["The comma separated tags for this tile, useful for searching for and grouping tiles".to_string()]);
    map.insert("raycaster_wall".to_string(), vec!["Display this tile in the raycaster as a \"wall\" or \"sprite\".".to_string()]);
    map.insert("raycaster_sprite_shrink".to_string(), vec!["The shrink factor for sprites. 1 is the original size, higher values shrink the sprite.".to_string()]);
    map.insert("raycaster_sprite_move_y".to_string(), vec!["Moves the sprite up / down.".to_string()]);


    map
}