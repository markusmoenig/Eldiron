use crate::{TileMaterialMeta, Value, prelude::*};
use indexmap::IndexMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use theframework::prelude::*;
use toml::*;

const DEFAULT_GEOMETRY_TILE_ID: &str = "27826750-a9e7-4346-994b-fb318b238452";

struct ItemAliasCache {
    sources: Vec<(String, String)>,
    aliases: FxHashMap<String, String>,
}
impl ItemAliasCache {
    fn matches(&self, items: &FxHashMap<String, (String, String)>) -> bool {
        self.sources.len() == items.len()
            && self
                .sources
                .iter()
                .all(|(name, data)| items.get(name).is_some_and(|(_, current)| current == data))
    }
}

pub(crate) struct DisplayDefaults {
    pub attributes: crate::ValueContainer,
    pub equipped: indexmap::IndexMap<String, crate::Item>,
    pub dirty_flags: u8,
    pub dirty_attributes: FxHashSet<String>,
}

struct ParsedRulesCache {
    source: String,
    table: Option<Arc<Table>>,
}

/// A startup asset snapshot shared by region runtimes. Region-local edits detach
/// their copy so live rule/graph updates do not mutate another region's assets.
#[derive(Clone, Default)]
pub struct RegionAssets(Arc<Assets>);

impl RegionAssets {
    pub fn new(assets: &Assets) -> Self {
        Self(Arc::new(assets.clone()))
    }

    /// World-map lookup belongs to the server/client registry; each region owns
    /// its own runtime map. Exclude the registry without first cloning its geometry.
    pub fn for_regions(assets: &mut Assets) -> Self {
        let maps = std::mem::take(&mut assets.maps);
        let snapshot = Self::new(assets);
        assets.maps = maps;
        snapshot
    }
}

impl std::ops::Deref for RegionAssets {
    type Target = Assets;

    fn deref(&self) -> &Assets {
        &self.0
    }
}

impl std::ops::DerefMut for RegionAssets {
    fn deref_mut(&mut self) -> &mut Assets {
        Arc::make_mut(&mut self.0)
    }
}

#[derive(Clone)]
pub struct Assets {
    pub node_behaviors: Option<super::nodes::region::BehaviorAssets>,
    pub maps: FxHashMap<String, Map>,

    pub entities: FxHashMap<String, (String, String)>,
    pub items: FxHashMap<String, (String, String)>,
    /// Project item identities resolve to their current template name.
    pub item_template_ids: FxHashMap<Uuid, String>,
    pub entity_authoring: FxHashMap<String, String>,
    pub item_authoring: FxHashMap<String, String>,
    /// Resolved bundled and project-owned item icon animation frames, keyed by
    /// normalized item template/ruleset identifiers.
    pub item_icons: FxHashMap<String, Vec<Texture>>,
    /// Keys in `item_icons` that came from explicit project-owned frames rather
    /// than bundled ruleset defaults.
    pub project_item_icon_keys: FxHashSet<String>,

    pub tiles: IndexMap<Uuid, Tile>,
    pub block_props: IndexMap<Uuid, BlockPropAsset>,
    pub materials: FxHashMap<Uuid, Tile>,
    pub textures: FxHashMap<String, Texture>,
    /// Resolved ruleset MZ PBR maps shared by sprites and native meshes.
    pub ruleset_materials: FxHashMap<String, Texture>,
    /// Baked surface animation, in seconds; shared by 2D and 3D appearances.
    pub material_animations: FxHashMap<String, (Vec<Texture>, f32)>,
    pub item_models: FxHashMap<String, String>,
    pub item_wearables: FxHashMap<String, Option<crate::procedural_avatar::Wearable>>,

    pub tile_list: Vec<Tile>,
    pub tile_indices: FxHashMap<Uuid, u16>,

    pub screens: FxHashMap<String, Map>,
    /// Screen-level TOML keyed by screen/map name. Kept separate from the map
    /// so presentation settings do not leak into authored map properties.
    pub screen_settings: FxHashMap<String, String>,

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
    parsed_rules: Arc<RwLock<Option<ParsedRulesCache>>>,
    animation_tick: Arc<RwLock<Option<(String, f32)>>>,
    item_aliases: Arc<RwLock<Option<ItemAliasCache>>>,
    pub(crate) display_defaults: Arc<RwLock<FxHashMap<u64, Arc<DisplayDefaults>>>>,
    pub default_avatar: Option<String>,
    pub locales_src: String,
    pub audio_fx_src: String,
    pub authoring_src: String,
    pub region_sources: FxHashMap<Uuid, String>,
    pub atlas: Texture,

    pub fonts: FxHashMap<String, fontdue::Font>,
    pub audio: FxHashMap<String, Vec<u8>>,
    /// Project palette shared by world art, avatars, items and game UI.
    pub palette: ThePalette,
    /// Palette-fitted runtime copies of canonical ruleset tile artwork.
    pub palette_tile_overrides: IndexMap<Uuid, Tile>,
    /// Legacy/compat material values for palette-index sources.
    pub palette_materials: Vec<[f32; 4]>,
    /// Semantic material ids for palette-index sources.
    pub palette_material_ids: Vec<u8>,
    /// A map of locale names to their translations.
    pub locales: FxHashMap<String, FxHashMap<String, String>>,

    /// The avatars
    pub avatars: super::avatar_assets::AvatarAssets,
    pub procedural_avatars: FxHashMap<String, std::sync::Arc<crate::procedural_avatar::Appearance>>,
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl Assets {
    /// Immutable appearance resources only; never clone maps, audio or fonts for
    /// a preparation job. Resource changes replace the registered Appearance Arcs.
    pub(crate) fn appearance_preparation_snapshot(&self) -> Self {
        let mut snapshot = Self::default();
        snapshot.palette = self.palette.clone();
        snapshot.rules = self.rules.clone();
        snapshot.default_avatar = self.default_avatar.clone();
        snapshot.procedural_avatars = self.procedural_avatars.clone();
        snapshot.item_models = self.item_models.clone();
        snapshot.item_wearables = self.item_wearables.clone();
        snapshot.item_template_ids = self.item_template_ids.clone();
        snapshot.ruleset_materials = self.ruleset_materials.clone();
        snapshot.material_animations = self.material_animations.clone();
        snapshot.item_icons = self.item_icons.clone();
        snapshot.textures = self.textures.clone();
        snapshot.tiles = self.tiles.clone();
        snapshot
    }
}

impl Assets {
    pub fn resolve_item_class_name(&self, requested: &str) -> Option<String> {
        let requested = requested.trim();
        if requested.is_empty() {
            return None;
        }

        if let Ok(id) = Uuid::parse_str(requested) {
            if let Some(name) = self
                .item_template_ids
                .get(&id)
                .filter(|name| self.items.contains_key(*name))
            {
                return Some(name.clone());
            }
        }
        if self.items.contains_key(requested) {
            return Some(requested.to_string());
        }

        // Accept case-insensitive class/template ids.
        if let Some((class_name, _)) = self
            .items
            .iter()
            .find(|(class_name, _)| class_name.eq_ignore_ascii_case(requested))
        {
            return Some(class_name.clone());
        }

        // Equipment previews resolve the same display names every frame. Parse
        // aliases once, checking public item data for in-place edits before reuse.
        let requested = requested.to_ascii_lowercase();
        if let Ok(cache) = self.item_aliases.read()
            && let Some(cache) = cache.as_ref()
            && cache.matches(&self.items)
        {
            return cache.aliases.get(&requested).cloned();
        }
        let mut cache = ItemAliasCache {
            sources: vec![],
            aliases: FxHashMap::default(),
        };
        for (class_name, (_, item_data)) in &self.items {
            cache.sources.push((class_name.clone(), item_data.clone()));
            let Ok(table) = item_data.parse::<toml::Table>() else {
                continue;
            };
            let attrs = table.get("attributes").and_then(toml::Value::as_table);
            for name in [
                attrs
                    .and_then(|a| a.get("name"))
                    .and_then(toml::Value::as_str),
                attrs
                    .and_then(|a| a.get("ruleset_id"))
                    .and_then(toml::Value::as_str),
                attrs
                    .and_then(|a| a.get("source_id"))
                    .and_then(toml::Value::as_str),
                attrs
                    .and_then(|a| a.get("ruleset_path"))
                    .and_then(toml::Value::as_str)
                    .and_then(|p| p.rsplit('.').next()),
                table.get("name").and_then(toml::Value::as_str),
            ]
            .into_iter()
            .flatten()
            {
                cache
                    .aliases
                    .entry(name.to_ascii_lowercase())
                    .or_insert_with(|| class_name.clone());
            }
        }
        let found = cache.aliases.get(&requested).cloned();
        if let Ok(mut slot) = self.item_aliases.write() {
            *slot = Some(cache);
        }
        found
    }

    /// Return the effective ruleset as a shared parsed table.
    ///
    /// Rules source is public for editor compatibility, so the cache verifies
    /// the source text on every access and reparses only after it changes.
    pub fn rules_table(&self) -> Option<Arc<Table>> {
        if let Ok(cache) = self.parsed_rules.read()
            && let Some(cache) = cache.as_ref()
            && cache.source == self.rules
        {
            return cache.table.clone();
        }

        let table = self.rules.parse::<Table>().ok().map(Arc::new);
        if let Ok(mut cache) = self.parsed_rules.write() {
            *cache = Some(ParsedRulesCache {
                source: self.rules.clone(),
                table: table.clone(),
            });
        }
        table
    }

    /// Keep presentation on the configured server tick without parsing the
    /// entire project TOML for every 2D/3D redraw. Direct config edits and cloned
    /// asset snapshots are checked against the source, rather than a stale flag.
    pub(crate) fn animation_tick_seconds(&self) -> f32 {
        if let Ok(cache) = self.animation_tick.read()
            && let Some((source, seconds)) = &*cache
            && source == &self.config
        {
            return *seconds;
        }
        let seconds = self
            .config
            .parse::<Table>()
            .ok()
            .and_then(|t| {
                t.get("game")
                    .and_then(|g| g.get("game_tick_ms"))
                    .and_then(toml::Value::as_integer)
            })
            .unwrap_or(250)
            .clamp(1, 10000) as f32
            / 1000.;
        if let Ok(mut cache) = self.animation_tick.write() {
            *cache = Some((self.config.clone(), seconds));
        }
        seconds
    }

    pub fn ruleset_attribute_role(&self, role: &str) -> Option<String> {
        self.rules_table()
            .and_then(|rules| eldiron_ruleset::resolve_attribute_roles(&rules).ok())
            .and_then(|roles| roles.get(role).map(str::to_string))
    }

    pub fn ruleset_declares_attribute(&self, attribute: &str) -> bool {
        self.rules_table()
            .map(|rules| {
                eldiron_ruleset::declared_attribute_ids(&rules)
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(attribute))
            })
            .unwrap_or(false)
    }

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
            node_behaviors: None,
            maps: FxHashMap::default(),
            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            item_template_ids: FxHashMap::default(),
            entity_authoring: FxHashMap::default(),
            item_authoring: FxHashMap::default(),
            item_icons: FxHashMap::default(),
            project_item_icon_keys: FxHashSet::default(),
            tiles: IndexMap::default(),
            block_props: IndexMap::default(),
            textures: FxHashMap::default(),
            ruleset_materials: FxHashMap::default(),
            material_animations: FxHashMap::default(),
            item_models: FxHashMap::default(),
            item_wearables: FxHashMap::default(),
            tile_list: vec![],
            tile_indices: FxHashMap::default(),
            materials: FxHashMap::default(),
            screens: FxHashMap::default(),
            screen_settings: FxHashMap::default(),
            character_maps: FxHashMap::default(),
            entity_tiles: FxHashMap::default(),
            item_maps: FxHashMap::default(),
            item_tiles: FxHashMap::default(),
            config: String::new(),
            world_source: String::new(),
            rules: String::new(),
            parsed_rules: Arc::new(RwLock::new(None)),
            animation_tick: Default::default(),
            item_aliases: Arc::new(RwLock::new(None)),
            display_defaults: Default::default(),
            default_avatar: None,
            locales_src: String::new(),
            audio_fx_src: String::new(),
            authoring_src: String::new(),
            region_sources: FxHashMap::default(),
            atlas: Texture::default(),
            fonts: FxHashMap::default(),
            audio: FxHashMap::default(),
            palette: ThePalette::default(),
            palette_tile_overrides: IndexMap::new(),
            palette_materials: vec![[0.5, 0.0, 1.0, 0.0]; 256],
            palette_material_ids: vec![0; 256],
            locales: FxHashMap::default(),
            avatars: Default::default(),
            procedural_avatars: FxHashMap::default(),
        }
    }

    pub fn screen_is_responsive(&self, name: &str) -> bool {
        let Some(settings) = self.screen_settings.get(name) else {
            return false;
        };
        let Ok(table) = settings.parse::<Table>() else {
            return false;
        };
        table
            .get("layout")
            .and_then(toml::Value::as_table)
            .and_then(|layout| layout.get("mode"))
            .and_then(toml::Value::as_str)
            .is_some_and(|mode| mode.trim().eq_ignore_ascii_case("responsive"))
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
        let Some(table) = self.rules_table() else {
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

    pub fn set_block_props(&mut self, block_props: IndexMap<Uuid, BlockPropAsset>) {
        self.block_props = block_props;
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
        let maps = maps.into_iter().collect::<Vec<_>>();
        for map in &maps {
            for object in &map.geometry_objects {
                for face in &object.faces {
                    for source in face.tile.iter().chain(face.tiles.values()) {
                        match source {
                            PixelSource::Color(color) => {
                                let tile_id = PixelSource::color_tile_uuid(color);
                                tiles.entry(tile_id).or_insert_with(|| {
                                    let mut tile = Tile::from_texture(Texture::from_color(
                                        color.to_u8_array(),
                                    ));
                                    tile.id = tile_id;
                                    tile
                                });
                            }
                            PixelSource::Noise(noise) => {
                                tiles
                                    .entry(noise.tile_id())
                                    .or_insert_with(|| noise.to_tile(64));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

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
    fn region_assets_share_startup_data_and_isolate_live_edits() {
        let mut assets = Assets::new();
        assets.rules = "[value]\nnumber = 1\n".into();
        assets.maps.insert("Britannia".into(), Map::default());
        let snapshot = RegionAssets::new(&assets);
        let mut town = snapshot.clone();
        let world = snapshot.clone();
        assert!(Arc::ptr_eq(&town.0, &world.0));

        town.rules = "[value]\nnumber = 2\n".into();
        town.maps.clear();
        assert!(!Arc::ptr_eq(&town.0, &world.0));
        assert_eq!(world.rules, assets.rules);
        assert!(world.maps.contains_key("Britannia"));
        assert_eq!(
            town.rules_table().unwrap()["value"]["number"].as_integer(),
            Some(2)
        );
        assert_eq!(
            world.rules_table().unwrap()["value"]["number"].as_integer(),
            Some(1)
        );
    }

    #[test]
    fn region_snapshot_leaves_world_maps_in_the_client_registry() {
        let mut assets = Assets::new();
        let map = Map::default();
        let map_id = map.id;
        assets.maps.insert("Britannia".into(), map);
        let snapshot = RegionAssets::for_regions(&mut assets);
        assert!(snapshot.maps.is_empty());
        assert_eq!(assets.maps["Britannia"].id, map_id);
    }

    #[test]
    fn equipment_alias_cache_tracks_renames_removals_and_cloned_asset_edits() {
        let mut assets = Assets::default();
        assets.items.insert(
            "Shirt".into(),
            (
                String::new(),
                "[attributes]\nname='Linen Shirt'\nruleset_path='clothing.linen'\n".into(),
            ),
        );
        assert_eq!(
            assets.resolve_item_class_name("linen shirt"),
            Some("Shirt".into())
        );
        assert_eq!(
            assets.resolve_item_class_name("LINEN"),
            Some("Shirt".into())
        );
        let mut other = assets.clone();
        other.items.get_mut("Shirt").unwrap().1 =
            other.items["Shirt"].1.replace("Linen Shirt", "Chain Shirt");
        assert_eq!(other.resolve_item_class_name("linen shirt"), None);
        assert_eq!(
            other.resolve_item_class_name("chain shirt"),
            Some("Shirt".into())
        );
        assert_eq!(
            assets.resolve_item_class_name("linen shirt"),
            Some("Shirt".into())
        );
        assets.items.clear();
        assert_eq!(assets.resolve_item_class_name("linen"), None);
    }

    #[test]
    fn parsed_rules_are_reused_and_invalidated_by_source_changes() {
        let mut assets = Assets::new();
        assets.rules = "[value]\nnumber = 1\n".into();

        let first = assets.rules_table().expect("first parsed rules");
        let second = assets.rules_table().expect("cached parsed rules");
        assert!(Arc::ptr_eq(&first, &second));

        assets.rules = "[value]\nnumber = 2\n".into();
        let changed = assets.rules_table().expect("updated parsed rules");
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(
            changed
                .get("value")
                .and_then(toml::Value::as_table)
                .and_then(|value| value.get("number"))
                .and_then(toml::Value::as_integer),
            Some(2)
        );
    }

    #[test]
    fn animation_tick_cache_tracks_config_edits_and_cloned_snapshots() {
        let mut assets = Assets::default();
        assert_eq!(assets.animation_tick_seconds(), 0.25);
        assets.config = "[game]\ngame_tick_ms=500\n".into();
        assert_eq!(assets.animation_tick_seconds(), 0.5);
        assert_eq!(assets.animation_tick_seconds(), 0.5);
        let mut edited = assets.clone();
        edited.config = "[game]\ngame_tick_ms=50\n".into();
        assert_eq!(edited.animation_tick_seconds(), 0.05);
        assert_eq!(assets.animation_tick_seconds(), 0.5);
        for (source, expected) in [
            ("invalid = [", 0.25),
            ("[game]\ngame_tick_ms='fast'", 0.25),
            ("[game]\ngame_tick_ms=0", 0.001),
            ("[game]\ngame_tick_ms=20000", 10.),
        ] {
            assets.config = source.into();
            assert_eq!(assets.animation_tick_seconds(), expected);
        }
    }

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

    #[test]
    fn noise_material_is_registered_as_a_real_grayscale_atlas_tile() {
        let noise = crate::map::pixelsource::NoiseMaterial::default();
        let mut object = GeometryObject::box_("noise", Vec3::zero(), Vec3::broadcast(1.0));
        object.faces[0].tile = Some(PixelSource::Noise(noise.clone()));
        let mut map = Map::default();
        map.geometry_objects.push(object);
        let mut tiles = IndexMap::new();
        Assets::new().materialize_geometry_material_tiles_for_maps(&mut tiles, [&map]);
        let texture = &tiles[&noise.tile_id()].textures[0];
        assert_eq!((texture.width, texture.height), (64, 64));
        for pixel in texture.data.chunks_exact(4) {
            assert_eq!(pixel[0], pixel[1]);
            assert_eq!(pixel[1], pixel[2]);
            assert_eq!(pixel[3], 255);
            assert!((noise.low..=noise.high).contains(&pixel[0]));
        }
        assert!(
            texture
                .data
                .chunks_exact(4)
                .any(|pixel| pixel[0] != texture.data[0])
        );
    }

    #[test]
    fn materialize_geometry_tiles_registers_direct_color_sources() {
        let color = TheColor::new(0.3, 0.25, 0.2, 1.0);
        let color_id = PixelSource::color_tile_uuid(&color);
        let mut object = GeometryObject::box_("color", Vec3::zero(), Vec3::broadcast(1.0));
        object.faces[0].tile = Some(PixelSource::Color(color.clone()));
        let mut map = Map::default();
        map.geometry_objects.push(object);
        let mut tiles = IndexMap::new();

        Assets::new().materialize_geometry_material_tiles_for_maps(&mut tiles, [&map]);

        let tile = tiles.get(&color_id).expect("synthetic color tile");
        assert_eq!(tile.id, color_id);
        assert_eq!(tile.textures[0].data[0..4], color.to_u8_array());
    }
}
