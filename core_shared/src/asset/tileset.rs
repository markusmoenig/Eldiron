use crate::prelude::*;

use std::fs;
use std::path;
use std::fs::metadata;

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

/// The TileSet struct consists of several TileMaps, each representing one atlas and it's tiles.
pub struct TileSet {
    pub path            : PathBuf,

    pub maps            : HashMap<Uuid, TileMap>,
    pub maps_names      : Vec<String>,
    pub maps_ids        : Vec<Uuid>,

    pub images          : HashMap<Uuid, Image>,
    pub images_names    : Vec<String>,
    pub images_ids      : Vec<Uuid>,
}

impl TileSet {

    pub fn load_from_path(base_path: PathBuf) -> TileSet {

        let mut maps : HashMap<Uuid, TileMap> = HashMap::new();

        let tilemaps_path = base_path.join("assets").join("tilemaps");
        let mut paths: Vec<_> = fs::read_dir(tilemaps_path.clone()).unwrap()
                                                .map(|r| r.unwrap())
                                                .collect();
        paths.sort_by_key(|dir| dir.path());

        let mut maps_names  : Vec<String> = vec![];
        let mut maps_ids    : Vec<Uuid> = vec![];

        for path in paths {

            // Generate the tile map for this dir element
            let path = &path.path();
            let md = metadata(path).unwrap();

            if md.is_file() {
                if let Some(name) = path::Path::new(&path).extension() {
                    if name == "png" || name == "PNG" {

                        let tile_map = TileMap::new(&path, &base_path);
                        if tile_map.width != 0 {
                            maps_names.push(tile_map.get_name());
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

        let mut images : HashMap<Uuid, Image> = HashMap::new();

        let mut images_names  : Vec<String> = vec![];
        let mut images_ids    : Vec<Uuid> = vec![];

        let images_path = base_path.join("assets").join("images");
        if let Some(p) = fs::read_dir(images_path.clone()).ok() {

            let mut paths : Vec<_> = p.map(|r| r.unwrap())
                            .collect();
            paths.sort_by_key(|dir| dir.path());

            for path in paths {

                // Generate the tile map for this dir element
                let path = &path.path();
                let md = metadata(path).unwrap();

                if md.is_file() {
                    if let Some(name) = path::Path::new(&path).extension() {
                        if name == "png" || name == "PNG" {

                            let image = Image::new(&path, &base_path);
                            if image.width != 0 {
                                images_names.push(image.get_name());
                                images_ids.push(image.settings.id);

                                // If the tilemap has no tiles we assume it's new and we save the settings
                                if image.settings.tiles.len() == 0 {
                                    image.save_settings();
                                }

                                // Insert the tilemap
                                images.insert(image.settings.id, image);
                            }
                        }
                    }
                }
            }
        }

        TileSet {
            path        : base_path,
            maps,
            maps_names,
            maps_ids,
            images,
            images_names,
            images_ids
        }
    }

    #[cfg(feature = "embed_binaries")]
    pub fn load_from_embedded() -> TileSet {

        let mut maps : HashMap<Uuid, TileMap> = HashMap::new();
        let mut maps_names  : Vec<String> = vec![];
        let mut maps_ids    : Vec<Uuid> = vec![];

        for file in Embedded::iter() {
            let name = file.as_ref();
            let path = std::path::Path::new(name);
            if let Some(extension) = path.extension() {

                if name.starts_with("assets/tilemaps/") && (extension == "png" || extension == "PNG") {

                    let tile_map = TileMap::new_from_embedded(name);
                    if tile_map.width != 0 {
                        maps_names.push(tile_map.get_name());
                        maps_ids.push(tile_map.settings.id);

                        maps.insert(tile_map.settings.id, tile_map);
                    }
                }
            }
        }

        let mut images : HashMap<Uuid, Image> = HashMap::new();
        let mut images_names  : Vec<String> = vec![];
        let mut images_ids    : Vec<Uuid> = vec![];

        for file in Embedded::iter() {
            let name = file.as_ref();
            let path = std::path::Path::new(name);
            if let Some(extension) = path.extension() {

                if name.starts_with("assets/images/") && (extension == "png" || extension == "PNG") {

                    let image = Image::new_from_embedded(name);
                    if image.width != 0 {
                        images_names.push(image.get_name());
                        images_ids.push(image.settings.id);

                        images.insert(image.settings.id, image);
                    }
                }
            }
        }

        TileSet {
            path            : PathBuf::new(),
            maps,
            maps_names,
            maps_ids,
            images,
            images_names,
            images_ids
        }
    }

    pub fn new() -> Self {

        let maps            : HashMap<Uuid, TileMap> = HashMap::new();
        let maps_names      : Vec<String> = vec![];
        let maps_ids        : Vec<Uuid> = vec![];

        let images          : HashMap<Uuid, Image> = HashMap::new();
        let images_names    : Vec<String> = vec![];
        let images_ids      : Vec<Uuid> = vec![];

        Self {
            path            : PathBuf::new(),
            maps,
            maps_names,
            maps_ids,
            images,
            images_names,
            images_ids
        }
    }

    /// Add a tilemap from the given path
    pub fn add_tilemap(&mut self, path: PathBuf) -> bool {
        // Generate the tile map for this dir element
        let md = metadata(path.clone()).unwrap();

        if md.is_file() {
            if let Some(name) = path::Path::new(&path).extension() {
                if name == "png" || name == "PNG" {

                    let tile_map = TileMap::new(&path, &self.path);
                    if tile_map.width != 0 {
                        self.maps_names.push(tile_map.get_name());
                        self.maps_ids.push(tile_map.settings.id);

                        // If the tilemap has no tiles we assume it's new and we save the settings
                        if tile_map.settings.tiles.len() == 0 {
                            tile_map.save_settings();
                        }

                        // Insert the tilemap
                        self.maps.insert(tile_map.settings.id, tile_map);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Add a tilemap from the given path
    pub fn add_image(&mut self, path: PathBuf) -> bool {
        let md = metadata(path.clone()).unwrap();
        if md.is_file() {
            if let Some(name) = path::Path::new(&path).extension() {
                if name == "png" || name == "PNG" {

                    let image = Image::new(&path, &self.path);
                    if image.width != 0 {
                        self.images_names.push(image.get_name());
                        self.images_ids.push(image.settings.id);

                        // If the tilemap has no tiles we assume it's new and we save the settings
                        if image.settings.tiles.len() == 0 {
                            image.save_settings();
                        }

                        // Insert the tilemap
                        self.images.insert(image.settings.id, image);
                        return true;
                    }
                }
            }
        }
        false
    }

}