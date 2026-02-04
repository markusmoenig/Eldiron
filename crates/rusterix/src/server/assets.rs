use crate::{ShapeFXGraph, Value, prelude::*};
use indexmap::IndexMap;
use std::path::Path;
use theframework::prelude::*;
use toml::*;

#[derive(Clone)]
pub struct Assets {
    pub map_sources: FxHashMap<String, String>,
    pub maps: FxHashMap<String, Map>,

    pub entities: FxHashMap<String, (String, String)>,
    pub items: FxHashMap<String, (String, String)>,

    pub tiles: IndexMap<Uuid, Tile>,
    pub materials: FxHashMap<Uuid, Tile>,
    pub textures: FxHashMap<String, Texture>,

    pub tile_list: Vec<Tile>,
    pub tile_indices: FxHashMap<Uuid, u16>,

    pub screens: FxHashMap<String, Map>,

    /// Maps which build character tiles.
    pub character_maps: FxHashMap<String, Map>,

    /// The rendered tiles for a given entity.
    pub entity_tiles: FxHashMap<u32, IndexMap<String, Tile>>,

    /// Maps which build item tiles.
    pub item_maps: FxHashMap<String, Map>,

    /// The rendered tiles for a given item.
    pub item_tiles: FxHashMap<u32, IndexMap<String, Tile>>,

    pub config: String,
    pub atlas: Texture,

    pub fonts: FxHashMap<String, fontdue::Font>,
    pub palette: ThePalette,

    // The global render graph
    pub global: ShapeFXGraph,

    /// A map of locale names to their translations.
    pub locales: FxHashMap<String, FxHashMap<String, String>>,
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl Assets {
    pub fn new() -> Self {
        Self {
            map_sources: FxHashMap::default(),
            maps: FxHashMap::default(),
            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            tiles: IndexMap::default(),
            textures: FxHashMap::default(),
            tile_list: vec![],
            tile_indices: FxHashMap::default(),
            materials: FxHashMap::default(),
            screens: FxHashMap::default(),
            character_maps: FxHashMap::default(),
            entity_tiles: FxHashMap::default(),
            item_maps: FxHashMap::default(),
            item_tiles: FxHashMap::default(),
            config: String::new(),
            atlas: Texture::default(),
            fonts: FxHashMap::default(),
            palette: ThePalette::default(),
            global: ShapeFXGraph::default(),
            locales: FxHashMap::default(),
        }
    }

    /// Reads all locale tables (locale_*) from the config file.
    pub fn read_locales(&mut self) {
        self.locales.clear();
        if let Ok(table) = self.config.parse::<Table>() {
            for (key, value) in table.iter() {
                if let Some(locale_name) = key.strip_prefix("locale_") {
                    if let Some(locales) = value.as_table() {
                        let mut translations = FxHashMap::default();
                        for (name, value) in locales.iter() {
                            if let Some(locale_map) = value.as_str() {
                                translations.insert(name.clone(), locale_map.to_string());
                            }
                        }
                        // println!("Found locale: {} {:?}", locale_name, translations);
                        self.locales.insert(locale_name.to_string(), translations);
                    }
                }
            }
        }
    }

    /// Clears the tile list.
    pub fn clean_tile_list(&mut self) {
        self.tile_list.clear();
        self.tile_indices.clear();
    }

    /// Returns the index into the tile_list for the given Tile Id
    pub fn tile_index(&self, id: &Uuid) -> Option<u16> {
        self.tile_indices.get(id).copied()
    }

    /// Set the tiles and atlas from a list of RGBA tiles.
    pub fn set_tiles(&mut self, tiles: IndexMap<Uuid, Tile>) {
        /*
        let mut tiles: FxHashMap<Uuid, Tile> = FxHashMap::default();

        for (id, t) in textures.iter() {
            let mut texture_array: Vec<Texture> = vec![];
            for b in &t.buffer {
                let mut texture = Texture::new(
                    b.pixels().to_vec(),
                    b.dim().width as usize,
                    b.dim().height as usize,
                );
                texture.generate_normals(true);
                texture_array.push(texture);
            }
            let tile = Tile {
                id: t.id,
                textures: texture_array.clone(),
                module: None,
                blocking: t.blocking,
                scale: t.scale,
                tags: t.name.clone(),
            };
            tiles.insert(*id, tile);
        }*/

        self.tiles = tiles;

        // Update tile_list and tile_indices
        for (id, tile) in &self.tiles {
            if let Some(&index) = self.tile_indices.get(id) {
                self.tile_list[index as usize] = tile.clone();
            } else {
                let index = self.tile_list.len() as u16;
                self.tile_indices.insert(*id, index);
                self.tile_list.push(tile.clone());
            }
        }
    }

    /// Compile the materials.
    pub fn set_materials(&mut self, materials: FxHashMap<Uuid, Map>) {
        let mut tiles = FxHashMap::default();
        for (uuid, map) in materials.iter() {
            if let Some(Value::Texture(texture)) = map.properties.get("material") {
                let mut tile = Tile::from_texture(texture.clone());
                tile.id = *uuid;
                tiles.insert(map.id, tile.clone());

                // Add it to the tile_list
                if let Some(&index) = self.tile_indices.get(&tile.id) {
                    self.tile_list[index as usize] = tile;
                } else {
                    let index = self.tile_list.len() as u16;
                    self.tile_indices.insert(tile.id, index);
                    self.tile_list.push(tile);
                }
            }
        }
        self.materials = tiles;
    }

    /// Returns an FxHashSet of Uuid representing the blocking tiles and materials.
    pub fn blocking_tiles(&self) -> FxHashSet<Uuid> {
        let mut blocking_tiles = FxHashSet::default();
        for tile in self.tiles.values() {
            if tile.blocking {
                blocking_tiles.insert(tile.id);
            }
        }
        for mat in self.materials.values() {
            if mat.blocking {
                blocking_tiles.insert(mat.id);
            }
        }
        blocking_tiles
    }

    /// Collects the assets from a directory.
    pub fn collect_from_directory(&mut self, dir_path: String) {
        let path = Path::new(&dir_path);

        if !path.is_dir() {
            eprintln!("Error: '{}' is not a directory.", path.display());
            return;
        }

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
                    match extension {
                        // Texture
                        "png" | "PNG" => {
                            if let Some(tex) = Texture::from_image_safe(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.textures.insert(base_name.to_string(), tex);
                                }
                            }
                        }
                        // Entity
                        "rxe" => {
                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.entities
                                        .insert(base_name.to_string(), (source, String::new()));
                                }
                            }
                        }
                        // Map
                        "rxm" => {
                            if let Ok(source) = std::fs::read_to_string(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.map_sources.insert(base_name.to_string(), source);
                                }
                            }
                        }
                        _ => {
                            // println!("Unsupported file extension: {:?}", extension)
                        }
                    }
                }
            }
        }
    }

    /*
    /// Compile all source maps
    pub fn compile_source_maps(&mut self) {
        let keys = self.map_sources.keys().cloned().collect::<Vec<String>>();
        for name in keys {
            let _ = self.compile_source_map(name);
        }
    }*/

    /*
    /// Compile the given source map
    pub fn compile_source_map(&mut self, name: String) -> Result<(), Vec<String>> {
        if let Some(source) = self.map_sources.get(&name) {
            let mut mapscript = MapScript::default();
            match mapscript.compile(source, &self.textures, None, None, None) {
                Ok(meta) => {
                    self.maps.insert(name, meta.map);
                    for (id, tile) in meta.tiles {
                        self.tiles.insert(id, tile);
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }*/

    /// Get a map by name.
    pub fn get_map(&self, name: &str) -> Option<&Map> {
        self.maps.get(name)
    }

    /// Add an entity.
    pub fn add_entity(&mut self, name: String, code: String, data: String) {
        self.entities.insert(name, (code, data));
    }

    /// Sets textures using the builder pattern.
    pub fn textures(mut self, textures: Vec<Tile>) -> Self {
        self.tile_list = textures;
        self
    }
}
