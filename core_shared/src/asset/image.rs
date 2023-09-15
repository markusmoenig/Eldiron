use crate::prelude::*;

use std::fs;
use std::fs::File;
use std::path;

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

#[derive(Serialize, Deserialize)]
pub struct ImageTile {
    pub usage: TileUsage,
    pub anim_tiles: Vec<(usize, usize)>,
    pub size: Vec<(usize, usize)>,
    pub tags: String,
    pub role: usize,
}

// TileMap implementation

#[derive(Serialize, Deserialize)]
pub struct ImageSettings {
    pub grid_size: usize,
    #[serde(with = "vectorize")]
    pub tiles: HashMap<(usize, usize), ImageTile>,
    pub id: Uuid,
    pub default_tile: Option<(usize, usize)>,
}

pub struct Image {
    pub base_path: PathBuf,
    pub pixels: Vec<u8>,
    pub file_path: PathBuf,
    pub width: usize,
    pub height: usize,
    pub settings: ImageSettings,
}

impl Image {
    pub fn new(file_name: &PathBuf, base_path: &PathBuf) -> Self {
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
        let name = path::Path::new(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let json_path = path::Path::new(base_path)
            .join("assets")
            .join("images")
            .join(format!("{}{}", name, ".json"));
        let contents = fs::read_to_string(json_path).unwrap_or("".to_string());

        // Construct the json settings
        let settings = serde_json::from_str(&contents).unwrap_or(ImageSettings {
            grid_size: 16,
            tiles: HashMap::new(),
            id: Uuid::new_v4(),
            default_tile: None,
        });

        Self {
            base_path: base_path.clone(),
            pixels: info.0,
            file_path: file_name.to_path_buf(),
            width: info.1 as usize,
            height: info.2 as usize,
            settings,
        }
    }

    pub fn new_from_embedded(file_name: &str) -> Self {
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
            (vec![], 0, 0)
        }

        let info = load(file_name);

        // Gets the content of the settings file
        let name = path::Path::new(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let json_path = path::Path::new("")
            .join("assets")
            .join("images")
            .join(format!("{}{}", name, ".json"));

        let mut contents = "".to_string();
        if let Some(bytes) = Embedded::get(json_path.to_str().unwrap()) {
            if let Some(string) = std::str::from_utf8(bytes.data.as_ref()).ok() {
                contents = string.to_string();
            }
        }

        // Construct the json settings
        let settings = serde_json::from_str(&contents).unwrap_or(ImageSettings {
            grid_size: 16,
            tiles: HashMap::new(),
            id: Uuid::new_v4(),
            default_tile: None,
        });

        Self {
            base_path: PathBuf::new(),
            pixels: info.0,
            file_path: std::path::Path::new(file_name).to_path_buf(),
            width: info.1 as usize,  //           : 800,//info.1 as usize,
            height: info.2 as usize, //,//info.2 as usize,
            settings,
        }
    }

    /*
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
    }*/

    /// Returns the name of the tilemap
    pub fn get_name(&self) -> String {
        path::Path::new(&self.file_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    /// Save the TileMapSettings to file
    pub fn save_settings(&self) {
        let name = path::Path::new(&self.file_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let json_path = self
            .base_path
            .join("assets")
            .join("images")
            .join(format!("{}{}", name, ".json"));

        let json = serde_json::to_string(&self.settings).unwrap();
        fs::write(json_path, json).expect("Unable to write file");
    }

    /*
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
    }*/
}
