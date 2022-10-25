pub mod tilemap;
pub mod image;
pub mod tileset;

use crate::prelude::*;

use std::{path::PathBuf, collections::HashMap};
use fontdue::Font;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct Asset {
    pub tileset                 : TileSet,

    pub game_fonts              : HashMap<String, Font>,
    pub editor_fonts            : HashMap<String, Font>,

    pub audio_names             : Vec<String>,
    pub audio_paths             : Vec<PathBuf>,
}

impl Asset  {

    pub fn new() -> Self where Self: Sized {

        Self {
            tileset             : tileset::TileSet::new(),
            game_fonts          : HashMap::new(),
            editor_fonts        : HashMap::new(),
            audio_names         : vec![],
            audio_paths         : vec![],
        }
    }

    /// Load editor font
    pub fn load_editor_font<'a>(&mut self, name: String, resource_name: String) {
        let path = std::path::Path::new("resources").join(resource_name);

        if let Some(font_bytes) = std::fs::read(path).ok() {
            if let Some(font) = Font::from_bytes(font_bytes, fontdue::FontSettings::default()).ok() {
                self.editor_fonts.insert(name, font);
            }
        }
    }

    pub fn get_editor_font(&self, name: &str) -> &Font {
        self.editor_fonts.get(name).unwrap()
    }

    /// Load from the given file path
    pub fn load_from_path(&mut self, path: PathBuf) {
        self.tileset = tileset::TileSet::load_from_path(path.clone());

        // Collect the fonts

        let font_path = path.join("assets").join("fonts");
        let paths = std::fs::read_dir(font_path).unwrap();

        for path in paths {
            // Generate the tile map for this dir element
            let path = &path.unwrap().path();

            if path.is_file() {//&& path.extension().map(|s| s == "ttf").unwrap_or(false) {

                if let Some(font_bytes) = std::fs::read(path).ok() {
                if let Some(font) = Font::from_bytes(font_bytes, fontdue::FontSettings::default()).ok() {
                        self.game_fonts.insert(path.file_stem().unwrap().to_os_string().into_string().unwrap(), font);
                    }
                }
            }
        }

        // Collect audio files

        let font_path = path.join("assets").join("audio");
        let paths = std::fs::read_dir(font_path).unwrap();

        for path in paths {
            // Generate the tile map for this dir element
            let path = &path.unwrap().path();

            if path.is_file() && path.extension().map(|s| s == "wav" || s == "ogg").unwrap_or(false) {
                let mut name = std::path::Path::new(&path).file_stem().unwrap().to_str().unwrap().to_string();
                name = format!("{}.{}", name, std::path::Path::new(&path).extension().unwrap().to_str().unwrap().to_lowercase());
                self.audio_names.push(name.to_string());
                self.audio_paths.push(path.clone());
            }
        }
    }

    #[cfg(feature = "embed_binaries")]
    /// Load from embedded binaries
    pub fn load_from_embedded(&mut self) {
        self.tileset = tileset::TileSet::load_from_embedded();

        for file in Embedded::iter() {
            let name = file.as_ref();
            if name.starts_with("assets/fonts/") {
                if let Some(font_bytes) = Embedded::get(name) {
                    if let Some(font) = Font::from_bytes(font_bytes.data, fontdue::FontSettings::default()).ok() {
                        let buf = std::path::Path::new(name);
                        self.game_fonts.insert(buf.file_stem().unwrap().to_os_string().into_string().unwrap(), font);
                    }
                }
            } else
            if name.starts_with("assets/audio/") {
                let buf = std::path::Path::new(name);
                let mut cut_out = name.clone().to_string();
                cut_out.replace_range(0..13, "");
                if cut_out.starts_with(".") == false {
                    self.audio_names.push(cut_out);
                    self.audio_paths.push(buf.to_path_buf());
                }
            }
        }
    }

    /// Returns the tilemap of the given id
    pub fn get_map_of_id(&self, id: Uuid) -> Option<&TileMap> {
        self.tileset.maps.get(&id)
    }

    /// Returns a reference to the tile of the given id
    pub fn get_tile(&self, id: &TileId) -> Option<&Tile> {
        if let Some(map) = self.get_map_of_id(id.tilemap) {
            return map.get_tile(&(id.x_off as usize, id.y_off as usize));
        }
        None
    }

    /// Returns a mutable reference to tile of the given id
    pub fn get_mut_tile(&mut self, id: &TileId) -> Option<&mut Tile> {
        if let Some(map) = self.tileset.maps.get_mut(&id.tilemap) {
            return map.get_mut_tile(&(id.x_off as usize, id.y_off as usize));
        }
        None
    }

    /// Add a tilemap from the given path
    pub fn add_audio(&mut self, path: PathBuf) -> bool {
        if path.is_file() && path.extension().map(|s| s == "wav" || s == "ogg").unwrap_or(false) {
            let mut name = std::path::Path::new(&path).file_stem().unwrap().to_str().unwrap().to_string();
            name = format!("{}.{}", name, std::path::Path::new(&path).extension().unwrap().to_str().unwrap().to_lowercase());
            self.audio_names.push(name.to_string());
            self.audio_paths.push(path.clone());
            return true;
        }
        false
    }
}
