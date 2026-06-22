use crate::{TileMaterialMeta, Value, prelude::*};
use indexmap::IndexMap;
use std::path::Path;
use theframework::prelude::*;
use toml::*;

const DEFAULT_GEOMETRY_TILE_ID: &str = "27826750-a9e7-4346-994b-fb318b238452";

#[derive(Clone)]
pub struct Assets {
    pub map_sources: FxHashMap<String, String>,
    pub maps: FxHashMap<String, Map>,

    pub entities: FxHashMap<String, (String, String)>,
    pub items: FxHashMap<String, (String, String)>,
    pub entity_authoring: FxHashMap<String, String>,
    pub item_authoring: FxHashMap<String, String>,

    pub tiles: IndexMap<Uuid, Tile>,
    pub tile_groups: IndexMap<Uuid, TileGroup>,
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
    pub world_source: String,
    pub rules: String,
    pub default_avatar: Option<String>,
    pub locales_src: String,
    pub audio_fx_src: String,
    pub authoring_src: String,
    pub region_sources: FxHashMap<Uuid, String>,
    pub atlas: Texture,

    pub fonts: FxHashMap<String, fontdue::Font>,
    pub audio: FxHashMap<String, Vec<u8>>,
    pub palette: ThePalette,
    pub palette_materials: Vec<[f32; 4]>,

    /// A map of locale names to their translations.
    pub locales: FxHashMap<String, FxHashMap<String, String>>,

    /// The avatars
    pub avatars: FxHashMap<String, Avatar>,
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl Assets {
    fn collect_locale_entries(
        prefix: Option<&str>,
        table: &toml::value::Table,
        out: &mut FxHashMap<String, String>,
    ) {
        for (key, value) in table.iter() {
            let full_key = match prefix {
                Some(prefix) if !prefix.is_empty() => format!("{}.{}", prefix, key),
                _ => key.clone(),
            };

            if let Some(text) = value.as_str() {
                out.insert(full_key, text.to_string());
            } else if let Some(child) = value.as_table() {
                Self::collect_locale_entries(Some(&full_key), child, out);
            }
        }
    }

    pub fn new() -> Self {
        Self {
            map_sources: FxHashMap::default(),
            maps: FxHashMap::default(),
            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            entity_authoring: FxHashMap::default(),
            item_authoring: FxHashMap::default(),
            tiles: IndexMap::default(),
            tile_groups: IndexMap::default(),
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
            world_source: String::new(),
            rules: String::new(),
            default_avatar: None,
            locales_src: String::new(),
            audio_fx_src: String::new(),
            authoring_src: String::new(),
            region_sources: FxHashMap::default(),
            atlas: Texture::default(),
            fonts: FxHashMap::default(),
            audio: FxHashMap::default(),
            palette: ThePalette::default(),
            palette_materials: vec![[0.5, 0.0, 1.0, 0.0]; 256],
            locales: FxHashMap::default(),
            avatars: FxHashMap::default(),
        }
    }

    /// Reads all locale tables from the dedicated locales source.
    pub fn read_locales(&mut self) {
        self.locales.clear();
        if let Ok(table) = self.locales_src.parse::<Table>() {
            for (locale_name, value) in table.iter() {
                if let Some(locales) = value.as_table() {
                    let mut translations = FxHashMap::default();
                    Self::collect_locale_entries(None, locales, &mut translations);
                    self.locales.insert(locale_name.to_string(), translations);
                }
            }
        }
    }

    /// Reads lightweight metadata from the effective ruleset.
    pub fn read_rules_metadata(&mut self) {
        self.default_avatar = None;
        let Ok(table) = self.rules.parse::<Table>() else {
            return;
        };
        self.default_avatar = table
            .get("visuals")
            .and_then(toml::Value::as_table)
            .and_then(|visuals| visuals.get("defaults"))
            .and_then(toml::Value::as_table)
            .and_then(|defaults| defaults.get("avatar"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|avatar| !avatar.is_empty())
            .map(str::to_string);
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
        crate::server::data::rebuild_tile_alias_lookup(&self.tiles);

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

    pub fn object_material_meta(properties: &ValueContainer) -> Option<TileMaterialMeta> {
        let preset = properties
            .get_str_default("material_preset", "default".to_string())
            .trim()
            .to_ascii_lowercase();
        if preset.is_empty() || preset == "default" {
            return None;
        }

        let finish = properties
            .get_str_default("material_finish", "natural".to_string())
            .trim()
            .to_ascii_lowercase();
        Some(TileMaterialMeta {
            preset,
            finish: if finish.is_empty() {
                "natural".to_string()
            } else {
                finish
            },
        })
    }

    pub fn material_variant_tile_id(base_id: Uuid, material: &TileMaterialMeta) -> Option<Uuid> {
        if material.is_default() {
            return None;
        }

        Some(TileMaterialMeta::variant_tile_id(
            base_id,
            &material.normalized_preset(),
            &material.normalized_finish(),
        ))
    }

    pub fn materialized_tile_id(
        &self,
        source: Option<&PixelSource>,
        material: Option<&TileMaterialMeta>,
    ) -> Uuid {
        let base_id = source
            .and_then(|source| source.render_tile_id(self))
            .unwrap_or_else(Self::default_geometry_tile_id);

        material
            .and_then(|material| Self::material_variant_tile_id(base_id, material))
            .filter(|variant_id| self.tile_index(variant_id).is_some())
            .unwrap_or(base_id)
    }

    pub fn materialize_geometry_material_tiles(&self, tiles: &mut IndexMap<Uuid, Tile>) {
        self.materialize_geometry_material_tiles_for_maps(tiles, self.maps.values());
    }

    pub fn materialize_geometry_material_tiles_for_maps<'a, I>(
        &self,
        tiles: &mut IndexMap<Uuid, Tile>,
        maps: I,
    ) where
        I: IntoIterator<Item = &'a Map>,
    {
        let mut required = Vec::new();

        for map in maps {
            for object in &map.geometry_objects {
                let Some(material) = Self::object_material_meta(&object.properties) else {
                    continue;
                };

                for face in &object.faces {
                    required.push((
                        self.source_material_base_tile_id(face.tile.as_ref()),
                        material.clone(),
                    ));
                    for source in face.tiles.values() {
                        required.push((
                            self.source_material_base_tile_id(Some(source)),
                            material.clone(),
                        ));
                    }
                }
            }
        }

        for (base_id, material) in required {
            let Some(variant_id) = Self::material_variant_tile_id(base_id, &material) else {
                continue;
            };
            if tiles.contains_key(&variant_id) {
                continue;
            }

            let Some(mut tile) = tiles
                .get(&base_id)
                .cloned()
                .or_else(|| self.tiles.get(&base_id).cloned())
                .or_else(|| self.materials.get(&base_id).cloned())
            else {
                continue;
            };

            tile.id = variant_id;
            tile.material = material;
            tiles.insert(variant_id, tile);
        }
    }

    fn default_geometry_tile_id() -> Uuid {
        Uuid::parse_str(DEFAULT_GEOMETRY_TILE_ID).unwrap_or_else(|_| Uuid::nil())
    }

    fn source_material_base_tile_id(&self, source: Option<&PixelSource>) -> Uuid {
        source
            .and_then(|source| source.render_tile_id(self))
            .unwrap_or_else(Self::default_geometry_tile_id)
    }

    pub fn set_tile_groups(&mut self, tile_groups: IndexMap<Uuid, TileGroup>) {
        self.tile_groups = tile_groups;
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
                        // Audio
                        "wav" | "WAV" | "ogg" | "OGG" => {
                            if let Ok(bytes) = std::fs::read(file_path) {
                                if let Some(base_name) =
                                    file_path.file_stem().and_then(|stem| stem.to_str())
                                {
                                    self.audio.insert(base_name.to_string(), bytes);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeometryObject, PixelSource, Value};
    use vek::Vec3;

    #[test]
    fn materialize_geometry_material_tiles_adds_object_variant() {
        let base_id = Uuid::new_v4();
        let mut base_tile = Tile::from_texture(Texture::from_color([100, 100, 100, 255]));
        base_tile.id = base_id;

        let mut tiles = IndexMap::new();
        tiles.insert(base_id, base_tile);

        let mut object = GeometryObject::box_("test", Vec3::zero(), Vec3::broadcast(1.0));
        object
            .properties
            .set("material_preset", Value::Str("stone".to_string()));
        object
            .properties
            .set("material_finish", Value::Str("wet".to_string()));
        object.faces[0].tile = Some(PixelSource::TileId(base_id));

        let mut map = Map::default();
        map.geometry_objects.push(object);

        let assets = Assets::new();
        assets.materialize_geometry_material_tiles_for_maps(&mut tiles, [&map]);

        let material = TileMaterialMeta {
            preset: "stone".to_string(),
            finish: "wet".to_string(),
        };
        let variant_id = Assets::material_variant_tile_id(base_id, &material).unwrap();
        let variant = tiles.get(&variant_id).expect("material variant tile");

        assert_eq!(variant.id, variant_id);
        assert_eq!(variant.material.normalized_preset(), "stone");
        assert_eq!(variant.material.normalized_finish(), "wet");
    }
}
