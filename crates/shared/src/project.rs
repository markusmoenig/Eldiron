use crate::prelude::*;
use buildergraph::BuilderGraph;
use indexmap::IndexMap;
pub use rusterix::map::*;
use theframework::prelude::*;

/// Canonical procedural Recipe source stored as a first-class project asset.
///
/// The user-facing name and document kind deliberately are not duplicated here:
/// Creator derives both from `source`, keeping the canonical Recipe text as the
/// single source of truth.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProceduralRecipeAsset {
    pub id: Uuid,
    /// Stable project/ruleset lookup alias (normally the source-relative path).
    pub alias: String,
    /// Canonical `.recipe` source.
    pub source: String,
    /// Baked Tile generated from this source, when this is a Tile document.
    #[serde(default)]
    pub tile_id: Option<Uuid>,
}

impl ProceduralRecipeAsset {
    pub fn new(alias: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            alias: alias.into(),
            source: source.into(),
            tile_id: None,
        }
    }
}

impl Default for ProceduralRecipeAsset {
    fn default() -> Self {
        Self::new(
            "untitled-tile",
            "Tile\n    name = \"Untitled Tile\"\n    size = I2(64, 64)\n    coverage = I2(1, 1)\n\n    Height Surface\n        source = 0.5\n\n    Output\n        height = Surface\n",
        )
    }
}

/// The default target fps for the game.
fn default_target_fps() -> u32 {
    30
}

/// The default ms per tick for the game.
fn default_tick_ms() -> u32 {
    250
}

fn default_rules() -> String {
    String::new()
}

fn default_locales() -> String {
    String::new()
}

fn default_audio_fx() -> String {
    String::new()
}

fn default_authoring() -> String {
    String::new()
}

fn default_shortcuts() -> String {
    r#"# Override editor shortcuts by stable action id.
# Values are single keys and are matched case-insensitively.
[shortcuts]
"tool.object" = "O"
"tool.vertex" = "V"
"tool.edge" = "E"
"tool.face" = "F"
"#
    .to_string()
}

fn command_from_legacy_ui(ui: &toml::value::Table) -> Option<String> {
    if let Some(command) = ui
        .get("command")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(command.to_string());
    }

    if let Some(intent) = ui
        .get("intent")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if intent.eq_ignore_ascii_case("spell")
            && let Some(spell) = ui
                .get("spell")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        {
            return Some(format!("intent.spell:{}", spell));
        }
        return Some(format!("intent.{}", intent));
    }

    ui.get("action")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|action| format!("control.{}", action))
}

fn default_tile_board_cols() -> i32 {
    13
}

fn default_tile_board_rows() -> i32 {
    9
}

fn default_tile_collection_name() -> String {
    "New Collection".to_string()
}

fn default_tile_collection_version() -> String {
    "0.1".to_string()
}

fn default_palette_material_slots() -> Vec<PaletteMaterial> {
    vec![PaletteMaterial::default(); 256]
}

fn default_art_palette() -> ThePalette {
    const ENDESGA_64: [&str; 64] = [
        "ff0040", "131313", "1b1b1b", "272727", "3d3d3d", "5d5d5d", "858585", "b4b4b4", "ffffff",
        "c7cfdd", "92a1b9", "657392", "424c6e", "2a2f4e", "1a1932", "0e071b", "1c121c", "391f21",
        "5d2c28", "8a4836", "bf6f4a", "e69c69", "f6ca9f", "f9e6cf", "edab50", "e07438", "c64524",
        "8e251d", "ff5000", "ed7614", "ffa214", "ffc825", "ffeb57", "d3fc7e", "99e65f", "5ac54f",
        "33984b", "1e6f50", "134c4c", "0c2e44", "00396d", "0069aa", "0098dc", "00cdf9", "0cf1ff",
        "94fdff", "fdd2ed", "f389f5", "db3ffd", "7a09fa", "3003d9", "0c0293", "03193f", "3b1443",
        "622461", "93388f", "ca52c9", "c85086", "f68187", "f5555d", "ea323c", "c42430", "891e2b",
        "571c27",
    ];

    let mut palette = ThePalette::empty_256();
    for (index, hex) in ENDESGA_64.iter().enumerate() {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            palette.colors[index] = Some(TheColor::from_u8(r, g, b, 255));
        }
    }
    palette
}

fn merge_toml_tables(base: &mut toml::Table, overlay: toml::Table) {
    for (key, value) in overlay {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(base_table)), toml::Value::Table(overlay_table)) => {
                merge_toml_tables(base_table, overlay_table);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

fn item_ruleset_path(item: &Item) -> Option<String> {
    item.data
        .parse::<toml::Table>()
        .ok()
        .and_then(|data| {
            data.get("attributes")
                .and_then(toml::Value::as_table)
                .cloned()
        })
        .and_then(|attributes| {
            attributes
                .get("ruleset_path")
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
}

fn item_is_ruleset_owned(item: &Item) -> bool {
    item_ruleset_path(item).is_some()
}

fn sort_ruleset_items_after_custom(items: &mut IndexMap<Uuid, Item>) {
    let mut ruleset_items = items
        .iter()
        .filter(|(_, item)| item_is_ruleset_owned(item))
        .map(|(id, item)| (*id, item.clone()))
        .collect::<Vec<_>>();
    ruleset_items.sort_by(|(_, a), (_, b)| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then_with(|| item_ruleset_path(a).cmp(&item_ruleset_path(b)))
    });

    let custom_items = items
        .iter()
        .filter(|(_, item)| !item_is_ruleset_owned(item))
        .map(|(id, item)| (*id, item.clone()))
        .collect::<Vec<_>>();

    items.clear();
    for (id, item) in custom_items.into_iter().chain(ruleset_items) {
        items.insert(id, item);
    }
}

pub fn merge_config_toml(project_config: &str, region_config: &str) -> String {
    if project_config.trim().is_empty() {
        return region_config.to_string();
    }
    if region_config.trim().is_empty() {
        return project_config.to_string();
    }

    let Ok(mut merged) = project_config.parse::<toml::Table>() else {
        return region_config.to_string();
    };
    let Ok(region) = region_config.parse::<toml::Table>() else {
        return project_config.to_string();
    };

    merge_toml_tables(&mut merged, region);
    toml::to_string(&merged).unwrap_or_else(|_| project_config.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaletteMaterial {
    #[serde(
        default = "default_palette_material_preset",
        skip_serializing_if = "is_default_palette_material_preset"
    )]
    pub preset: String,
    #[serde(
        default = "default_palette_material_finish",
        skip_serializing_if = "is_default_palette_material_finish"
    )]
    pub finish: String,
    #[serde(default = "default_palette_roughness", skip_serializing)]
    pub roughness: f32,
    #[serde(default = "default_palette_metallic", skip_serializing)]
    pub metallic: f32,
    #[serde(default = "default_palette_opacity", skip_serializing)]
    pub opacity: f32,
    #[serde(default = "default_palette_emissive", skip_serializing)]
    pub emissive: f32,
}

fn default_palette_material_preset() -> String {
    "default".to_string()
}

fn default_palette_material_finish() -> String {
    "natural".to_string()
}

fn is_default_palette_material_preset(value: &String) -> bool {
    value.trim().is_empty() || value.eq_ignore_ascii_case("default")
}

fn is_default_palette_material_finish(value: &String) -> bool {
    value.trim().is_empty() || value.eq_ignore_ascii_case("natural")
}

fn default_palette_roughness() -> f32 {
    0.5
}

fn default_palette_metallic() -> f32 {
    0.0
}

fn default_palette_opacity() -> f32 {
    1.0
}

fn default_palette_emissive() -> f32 {
    0.0
}

impl Default for PaletteMaterial {
    fn default() -> Self {
        Self {
            preset: default_palette_material_preset(),
            finish: default_palette_material_finish(),
            roughness: default_palette_roughness(),
            metallic: default_palette_metallic(),
            opacity: default_palette_opacity(),
            emissive: default_palette_emissive(),
        }
    }
}

impl PaletteMaterial {
    pub fn material_id(&self) -> u8 {
        rusterix::material_library::MaterialDefinition::from_preset_finish(
            self.preset.trim(),
            self.finish.trim(),
        )
        .id()
    }

    pub fn rmoe_values(&self) -> [f32; 4] {
        let mut material = self.clone();
        if !is_default_palette_material_preset(&material.preset)
            || !is_default_palette_material_finish(&material.finish)
        {
            return rusterix::material_library::MaterialDefinition::from_preset_finish(
                &material.preset,
                &material.finish,
            )
            .rmoe_values();
        } else {
            material.roughness = default_palette_roughness();
            material.metallic = default_palette_metallic();
            material.opacity = default_palette_opacity();
            material.emissive = default_palette_emissive();
        }
        [
            material.roughness,
            material.metallic,
            material.opacity,
            material.emissive,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BuilderGraphAsset {
    pub id: Uuid,
    pub graph_id: Uuid,
    #[serde(default)]
    pub graph_name: String,
    #[serde(default)]
    pub graph_data: String,
}

impl BuilderGraphAsset {
    fn from_script(name: String, fallback_name: &str, graph_data: String) -> Self {
        let graph_name = if let Ok(document) = buildergraph::BuilderDocument::from_text(&graph_data)
        {
            document.name().to_string()
        } else if name.is_empty() {
            fallback_name.to_string()
        } else {
            name
        };
        Self {
            id: Uuid::new_v4(),
            graph_id: Uuid::new_v4(),
            graph_name,
            graph_data,
        }
    }

    pub fn new_table(name: String) -> Self {
        let graph_data = BuilderGraph::preset_table_script_named(name.clone());
        Self::from_script(name, "Table", graph_data)
    }

    pub fn new_empty(name: String) -> Self {
        let graph_data = BuilderGraph::empty_script_named(name.clone());
        Self::from_script(name, "Empty", graph_data)
    }

    pub fn new_wall_torch(name: String) -> Self {
        let graph_data = BuilderGraph::preset_wall_torch_script_named(name.clone());
        Self::from_script(name, "Wall Torch", graph_data)
    }

    pub fn new_wall_lantern(name: String) -> Self {
        let graph_data = BuilderGraph::preset_wall_lantern_script_named(name.clone());
        Self::from_script(name, "Wall Lantern", graph_data)
    }

    pub fn new_campfire(name: String) -> Self {
        let graph_data = BuilderGraph::preset_campfire_script_named(name.clone());
        Self::from_script(name, "Campfire", graph_data)
    }

    pub fn new_surface_masonry(name: String) -> Self {
        let graph_data = BuilderGraph::preset_surface_masonry_script_named(name.clone());
        Self::from_script(name, "Surface Masonry", graph_data)
    }

    pub fn new_wall_masonry(name: String) -> Self {
        let graph_data = BuilderGraph::preset_wall_masonry_script_named(name.clone());
        Self::from_script(name, "Wall Masonry", graph_data)
    }

    pub fn new_wall_columns_masonry(name: String) -> Self {
        let graph_data = BuilderGraph::preset_wall_columns_masonry_script_named(name.clone());
        Self::from_script(name, "Wall Columns Masonry", graph_data)
    }

    pub fn new_grass(name: String) -> Self {
        let graph_data = BuilderGraph::preset_grass_script_named(name.clone());
        Self::from_script(name, "Grass", graph_data)
    }

    pub fn new_grass_patch(name: String) -> Self {
        Self::new_grass(name)
    }

    pub fn new_bush(name: String) -> Self {
        let graph_data = BuilderGraph::preset_bush_script_named(name.clone());
        Self::from_script(name, "Bush", graph_data)
    }

    pub fn new_bush_cluster(name: String) -> Self {
        Self::new_bush(name)
    }

    pub fn new_tree(name: String) -> Self {
        let graph_data = BuilderGraph::preset_tree_script_named(name.clone());
        Self::from_script(name, "Tree", graph_data)
    }

    pub fn new_tree_grove(name: String) -> Self {
        Self::new_tree(name)
    }

    pub fn new_grass_vertex(name: String) -> Self {
        let graph_data = BuilderGraph::preset_grass_vertex_script_named(name.clone());
        Self::from_script(name, "Grass", graph_data)
    }

    pub fn new_bush_vertex(name: String) -> Self {
        let graph_data = BuilderGraph::preset_bush_vertex_script_named(name.clone());
        Self::from_script(name, "Bush", graph_data)
    }

    pub fn new_tree_vertex(name: String) -> Self {
        let graph_data = BuilderGraph::preset_tree_vertex_script_named(name.clone());
        Self::from_script(name, "Tree", graph_data)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TileCollectionEntry {
    SingleTile(Uuid),
    TileGroup(Uuid),
}

impl TileCollectionEntry {
    pub fn matches_source(&self, source: rusterix::TileSource) -> bool {
        match (self, source) {
            (Self::SingleTile(a), rusterix::TileSource::SingleTile(b)) => *a == b,
            (Self::TileGroup(a), rusterix::TileSource::TileGroup(b)) => *a == b,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TileCollectionAsset {
    pub id: Uuid,
    #[serde(default = "default_tile_collection_name")]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_tile_collection_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entries: Vec<TileCollectionEntry>,
    #[serde(default)]
    pub tile_board_tiles: IndexMap<Uuid, Vec2<i32>>,
    #[serde(default)]
    pub tile_board_groups: IndexMap<Uuid, Vec2<i32>>,
    #[serde(default)]
    pub tile_board_empty_slots: Vec<Vec2<i32>>,
}

impl TileCollectionAsset {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            author: String::new(),
            version: default_tile_collection_version(),
            description: String::new(),
            entries: Vec::new(),
            tile_board_tiles: IndexMap::default(),
            tile_board_groups: IndexMap::default(),
            tile_board_empty_slots: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub regions: Vec<Region>,
    pub tilemaps: Vec<Tilemap>,

    /// Tiles in the project
    #[serde(default)]
    pub tiles: IndexMap<Uuid, rusterix::Tile>,

    /// Spatial tile groups in the project.
    #[serde(default)]
    pub tile_groups: IndexMap<Uuid, rusterix::TileGroup>,

    /// Standalone builder graphs for props and assemblies.
    #[serde(default)]
    pub builder_graphs: IndexMap<Uuid, BuilderGraphAsset>,

    /// Reusable linked construction blocks, furniture, and interactive props.
    #[serde(default)]
    pub block_props: IndexMap<Uuid, rusterix::BlockPropAsset>,

    /// Asset-local 3D Paint keyed by Prefab UUID. Keeping paint beside the
    /// project catalog avoids making the renderer's geometry crate depend on
    /// Creator's shared paint model while still serializing it as Prefab data.
    #[serde(default)]
    pub block_prop_paint: IndexMap<Uuid, IsoPaintLayer>,

    /// Runtime-only isolated map used while editing a Prefab asset.
    #[serde(skip)]
    pub prefab_editor_map: Option<Map>,

    /// Runtime-only ownership of isolated editor objects by stable Prefab part.
    #[serde(skip)]
    pub prefab_editor_part_by_object: IndexMap<Uuid, Uuid>,

    /// Custom top-level tile collections shown as tabs in the tile picker.
    #[serde(default)]
    pub tile_collections: IndexMap<Uuid, TileCollectionAsset>,

    /// Persisted board positions for top-level single tiles in the tile picker.
    #[serde(default)]
    pub tile_board_tiles: IndexMap<Uuid, Vec2<i32>>,

    /// Persisted board positions for top-level tile groups in the tile picker.
    #[serde(default)]
    pub tile_board_groups: IndexMap<Uuid, Vec2<i32>>,

    /// Persisted empty board cells left behind by deletions in the tile picker.
    #[serde(default)]
    pub tile_board_empty_slots: Vec<Vec2<i32>>,

    /// Total board width in cells, including the trailing empty strip.
    #[serde(default = "default_tile_board_cols")]
    pub tile_board_cols: i32,

    /// Total board height in cells, including the trailing empty strip.
    #[serde(default = "default_tile_board_rows")]
    pub tile_board_rows: i32,

    #[serde(default)]
    pub time: TheTime,

    #[serde(default)]
    pub characters: IndexMap<Uuid, Character>,
    #[serde(default)]
    pub items: IndexMap<Uuid, Item>,

    #[serde(default)]
    pub screens: IndexMap<Uuid, Screen>,

    #[serde(default)]
    pub assets: IndexMap<Uuid, Asset>,

    /// Editable, shareable procedural Recipe documents.
    #[serde(default)]
    pub procedural_recipes: IndexMap<Uuid, ProceduralRecipeAsset>,

    /// Reusable procedural material sources keyed by their project alias.
    #[serde(default)]
    pub procedural_materials: IndexMap<String, String>,

    /// Reusable procedural SDF sources keyed by their project alias.
    #[serde(default)]
    pub procedural_sdfs: IndexMap<String, String>,

    #[serde(default)]
    pub palette: ThePalette,

    #[serde(default = "default_art_palette")]
    pub art_palette: ThePalette,

    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    #[serde(default = "default_tick_ms")]
    pub tick_ms: u32,

    #[serde(default)]
    pub config: String,

    #[serde(default)]
    pub world_module: serde_json::Value,

    #[serde(default)]
    pub world_source: String,

    #[serde(default)]
    pub world_source_debug: String,

    #[serde(default = "default_rules")]
    pub rules: String,

    #[serde(default = "default_locales")]
    pub locales: String,

    #[serde(default = "default_audio_fx")]
    pub audio_fx: String,

    #[serde(default = "default_authoring")]
    pub authoring: String,

    #[serde(default = "default_shortcuts")]
    pub shortcuts: String,

    #[serde(default)]
    pub avatars: IndexMap<Uuid, Avatar>,

    #[serde(default = "default_palette_material_slots")]
    pub palette_materials: Vec<PaletteMaterial>,

    #[serde(default = "default_palette_material_slots")]
    pub art_palette_materials: Vec<PaletteMaterial>,
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        let region = Region::default();

        Self {
            name: String::new(),

            regions: vec![region],
            tilemaps: vec![],

            tiles: IndexMap::default(),
            tile_groups: IndexMap::default(),
            builder_graphs: IndexMap::default(),
            block_props: IndexMap::default(),
            block_prop_paint: IndexMap::default(),
            prefab_editor_map: None,
            prefab_editor_part_by_object: IndexMap::default(),
            tile_collections: IndexMap::default(),
            tile_board_tiles: IndexMap::default(),
            tile_board_groups: IndexMap::default(),
            tile_board_empty_slots: Vec::new(),
            tile_board_cols: default_tile_board_cols(),
            tile_board_rows: default_tile_board_rows(),

            time: TheTime::default(),

            characters: IndexMap::default(),
            items: IndexMap::default(),

            screens: IndexMap::default(),
            assets: IndexMap::default(),
            procedural_recipes: IndexMap::default(),
            procedural_materials: IndexMap::default(),
            procedural_sdfs: IndexMap::default(),

            palette: ThePalette::default(),
            art_palette: default_art_palette(),

            target_fps: default_target_fps(),
            tick_ms: default_tick_ms(),

            avatars: IndexMap::default(),
            palette_materials: default_palette_material_slots(),
            art_palette_materials: default_palette_material_slots(),

            config: String::new(),
            world_module: serde_json::Value::Null,
            world_source: String::new(),
            world_source_debug: String::new(),
            rules: default_rules(),
            locales: default_locales(),
            audio_fx: default_audio_fx(),
            authoring: default_authoring(),
            shortcuts: default_shortcuts(),
        }
    }

    pub fn migrate_default_ruleset(&mut self) -> bool {
        if crate::rulesets::has_top_level_ruleset(&self.config) {
            return false;
        }

        crate::rulesets::prefix_default_ruleset_config(&mut self.config);
        self.rules = crate::rulesets::DEFAULT_RULES_OVERRIDE.to_string();
        true
    }

    pub fn migrate_button_commands(&mut self) -> bool {
        let mut changed = false;
        for screen in self.screens.values_mut() {
            for sector in &mut screen.map.sectors {
                let Some(Value::Str(data)) = sector.properties.get("data").cloned() else {
                    continue;
                };
                let Ok(mut table) = data.parse::<toml::Table>() else {
                    continue;
                };
                let Some(ui) = table.get_mut("ui").and_then(toml::Value::as_table_mut) else {
                    continue;
                };
                let role_is_button = ui
                    .get("role")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|role| role.trim().eq_ignore_ascii_case("button"));
                if !role_is_button || ui.get("command").is_some() {
                    continue;
                }
                let Some(command) = command_from_legacy_ui(ui) else {
                    continue;
                };
                ui.insert("command".to_string(), toml::Value::String(command));
                if let Ok(serialized) = toml::to_string(&table) {
                    sector.properties.set("data", Value::Str(serialized));
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn sync_ruleset_palette(&mut self) -> Result<bool, String> {
        let rules = crate::rulesets::resolve_project_rules(&self.config, &self.rules)?;
        let mut palette = crate::rulesets::ruleset_palette_from_source(&rules)?;
        if palette.is_empty() {
            return Ok(false);
        }

        let visible_count = palette
            .colors
            .iter()
            .rposition(Option::is_some)
            .map(|index| index + 1)
            .unwrap_or(1);
        palette.current_index = self
            .palette
            .current_index
            .min(visible_count.saturating_sub(1) as u16);
        let prev_palette = self.palette.clone();
        let prev_materials = self.palette_materials.clone();
        self.palette = palette;
        self.reset_all_palette_materials();
        Ok(prev_palette != self.palette || prev_materials != self.palette_materials)
    }

    pub fn ruleset_palette_is_active(&self) -> bool {
        let (id, version, source) = crate::rulesets::selected_ruleset(&self.config);
        if source != "project" {
            return crate::rulesets::official_ruleset(&id, &version)
                .and_then(|rules| crate::rulesets::ruleset_palette_from_source(rules).ok())
                .is_some_and(|palette| !palette.is_empty());
        }

        crate::rulesets::ruleset_palette_from_source(&self.rules)
            .is_ok_and(|palette| !palette.is_empty())
    }

    pub fn palette_visible_color_count(&self) -> usize {
        self.palette
            .colors
            .iter()
            .rposition(Option::is_some)
            .map(|index| index + 1)
            .unwrap_or(1)
    }

    pub fn art_palette_visible_color_count(&self) -> usize {
        self.art_palette
            .colors
            .iter()
            .rposition(Option::is_some)
            .map(|index| index + 1)
            .unwrap_or(1)
    }

    pub fn sync_ruleset_items(&mut self) -> Result<usize, String> {
        self.sync_ruleset_palette()?;
        let bundled_tiles = crate::rulesets::bundled_tiles_for_project(&self.config)?;
        let rules = crate::rulesets::resolve_project_rules(&self.config, &self.rules)?;
        let templates = crate::rulesets::ruleset_item_templates_from_source(&rules)?;
        let mut changed = 0;

        for (id, tile) in bundled_tiles {
            if self.tiles.get(&id) != Some(&tile) {
                self.tiles.insert(id, tile);
                changed += 1;
            }
        }

        for template in templates {
            if let Some(item) = self
                .items
                .values_mut()
                .find(|item| item_ruleset_path(item).as_deref() == Some(&template.ruleset_path))
            {
                let mut item_changed = false;
                if item.name != template.name {
                    item.name = template.name.clone();
                    item_changed = true;
                }
                if item.data != template.data {
                    item.data = template.data.clone();
                    item_changed = true;
                }
                if item.source != template.source {
                    item.source = template.source.clone();
                    item_changed = true;
                }
                if item.authoring != template.authoring {
                    item.authoring = template.authoring.clone();
                    item_changed = true;
                }
                if item_changed {
                    changed += 1;
                }
            } else {
                let mut item = Item::new();
                item.name = template.name;
                item.source = template.source;
                item.data = template.data;
                item.authoring = template.authoring;
                self.add_item(item);
                changed += 1;
            }
        }
        sort_ruleset_items_after_custom(&mut self.items);

        Ok(changed)
    }

    /// Add Character
    pub fn add_character(&mut self, character: Character) {
        self.characters.insert(character.id, character);
    }

    pub fn ensure_palette_materials_len(&mut self) {
        if self.palette_materials.len() < self.palette.colors.len() {
            self.palette_materials
                .resize(self.palette.colors.len(), PaletteMaterial::default());
        } else if self.palette_materials.len() > self.palette.colors.len() {
            self.palette_materials.truncate(self.palette.colors.len());
        }
    }

    pub fn reset_palette_material(&mut self, index: usize) {
        self.ensure_palette_materials_len();
        if let Some(material) = self.palette_materials.get_mut(index) {
            *material = PaletteMaterial::default();
        }
    }

    pub fn reset_all_palette_materials(&mut self) {
        self.palette_materials = default_palette_material_slots();
        self.ensure_palette_materials_len();
    }

    pub fn ensure_art_palette_materials_len(&mut self) {
        if self.art_palette_materials.len() < self.art_palette.colors.len() {
            self.art_palette_materials
                .resize(self.art_palette.colors.len(), PaletteMaterial::default());
        } else if self.art_palette_materials.len() > self.art_palette.colors.len() {
            self.art_palette_materials
                .truncate(self.art_palette.colors.len());
        }
    }

    pub fn reset_art_palette_material(&mut self, index: usize) {
        self.ensure_art_palette_materials_len();
        if let Some(material) = self.art_palette_materials.get_mut(index) {
            *material = PaletteMaterial::default();
        }
    }

    pub fn reset_all_art_palette_materials(&mut self) {
        self.art_palette_materials = default_palette_material_slots();
        self.ensure_art_palette_materials_len();
    }

    pub fn load_art_palette_from_text(&mut self, text: String) {
        let mut loaded = ThePalette::empty_256();
        let mut index = 0usize;

        for line in text.lines() {
            let value = line
                .trim()
                .trim_start_matches('#')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if value.is_empty() || value.starts_with(';') {
                continue;
            }

            let rgb = if value.len() == 6 {
                Some((0, 2, 4))
            } else if value.len() >= 8 {
                Some((2, 4, 6))
            } else {
                None
            };
            let Some((r_start, g_start, b_start)) = rgb else {
                continue;
            };

            let parsed = (
                u8::from_str_radix(&value[r_start..r_start + 2], 16),
                u8::from_str_radix(&value[g_start..g_start + 2], 16),
                u8::from_str_radix(&value[b_start..b_start + 2], 16),
            );
            if let (Ok(r), Ok(g), Ok(b)) = parsed {
                if index < loaded.colors.len() {
                    loaded.colors[index] = Some(TheColor::from_u8(r, g, b, 255));
                    index += 1;
                } else {
                    break;
                }
            }
        }

        if index == 0 {
            self.art_palette.load_from_txt(text);
        } else {
            loaded.current_index = self
                .art_palette
                .current_index
                .min(index.saturating_sub(1) as u16);
            self.art_palette = loaded;
        }
        self.reset_all_art_palette_materials();
    }

    /// Removes the given character from the project.
    pub fn remove_character(&mut self, id: &Uuid) {
        self.characters.shift_remove(id);
    }

    /// Returns a list of all characters sorted by name.
    pub fn sorted_character_list(&self) -> Vec<(Uuid, String)> {
        let mut entries: Vec<(Uuid, String)> = self
            .characters
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries
    }

    /// Returns a list of all items sorted by name.
    pub fn sorted_item_list(&self) -> Vec<(Uuid, String)> {
        let mut entries: Vec<(Uuid, String)> = self
            .items
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries
    }

    /// Add Avatar
    pub fn add_avatar(&mut self, avatar: Avatar) {
        self.avatars.insert(avatar.id, avatar);
    }

    pub fn add_tile_group(&mut self, tile_group: rusterix::TileGroup) {
        self.tile_groups.insert(tile_group.id, tile_group);
    }

    pub fn add_builder_graph(&mut self, builder_graph: BuilderGraphAsset) {
        self.builder_graphs.insert(builder_graph.id, builder_graph);
    }

    pub fn add_tile_collection(&mut self, collection: TileCollectionAsset) {
        self.tile_collections.insert(collection.id, collection);
    }

    pub fn collection_contains_source(
        &self,
        collection_id: &Uuid,
        source: rusterix::TileSource,
    ) -> bool {
        self.tile_collections
            .get(collection_id)
            .map(|collection| {
                collection
                    .entries
                    .iter()
                    .any(|entry| entry.matches_source(source))
            })
            .unwrap_or(false)
    }

    pub fn add_source_to_collection(&mut self, collection_id: &Uuid, source: rusterix::TileSource) {
        let Some(collection) = self.tile_collections.get_mut(collection_id) else {
            return;
        };
        let entry = match source {
            rusterix::TileSource::SingleTile(id) => TileCollectionEntry::SingleTile(id),
            rusterix::TileSource::TileGroup(id) => TileCollectionEntry::TileGroup(id),
            _ => return,
        };
        if !collection.entries.contains(&entry) {
            collection.entries.push(entry);
        }
    }

    pub fn remove_source_from_collections(&mut self, source: rusterix::TileSource) {
        for collection in self.tile_collections.values_mut() {
            collection
                .entries
                .retain(|entry| !entry.matches_source(source));
            match source {
                rusterix::TileSource::SingleTile(id) => {
                    collection.tile_board_tiles.shift_remove(&id);
                }
                rusterix::TileSource::TileGroup(id) => {
                    collection.tile_board_groups.shift_remove(&id);
                }
                _ => {}
            }
        }
    }

    pub fn remove_tile_group(&mut self, id: &Uuid) {
        self.tile_groups.shift_remove(id);
        self.tile_board_groups.shift_remove(id);
        self.remove_source_from_collections(rusterix::TileSource::TileGroup(*id));
    }

    pub fn tile_board_position(&self, source: rusterix::TileSource) -> Option<Vec2<i32>> {
        match source {
            rusterix::TileSource::SingleTile(id) => self.tile_board_tiles.get(&id).copied(),
            rusterix::TileSource::TileGroup(id) => self.tile_board_groups.get(&id).copied(),
            _ => None,
        }
    }

    pub fn collection_tile_board_position(
        &self,
        collection_id: &Uuid,
        source: rusterix::TileSource,
    ) -> Option<Vec2<i32>> {
        let collection = self.tile_collections.get(collection_id)?;
        match source {
            rusterix::TileSource::SingleTile(id) => collection.tile_board_tiles.get(&id).copied(),
            rusterix::TileSource::TileGroup(id) => collection.tile_board_groups.get(&id).copied(),
            _ => None,
        }
    }

    pub fn tile_board_empty_slots(&self) -> &[Vec2<i32>] {
        &self.tile_board_empty_slots
    }

    pub fn collection_tile_board_empty_slots(&self, collection_id: &Uuid) -> Option<&[Vec2<i32>]> {
        self.tile_collections
            .get(collection_id)
            .map(|collection| collection.tile_board_empty_slots.as_slice())
    }

    pub fn set_tile_board_position(&mut self, source: rusterix::TileSource, pos: Vec2<i32>) {
        self.clear_tile_board_empty_slot(pos);
        match source {
            rusterix::TileSource::SingleTile(id) => {
                self.tile_board_tiles.insert(id, pos);
            }
            rusterix::TileSource::TileGroup(id) => {
                self.tile_board_groups.insert(id, pos);
            }
            _ => {}
        }
    }

    pub fn set_collection_tile_board_position(
        &mut self,
        collection_id: &Uuid,
        source: rusterix::TileSource,
        pos: Vec2<i32>,
    ) {
        let Some(collection) = self.tile_collections.get_mut(collection_id) else {
            return;
        };
        if let Some(index) = collection
            .tile_board_empty_slots
            .iter()
            .position(|p| *p == pos)
        {
            collection.tile_board_empty_slots.swap_remove(index);
        }
        match source {
            rusterix::TileSource::SingleTile(id) => {
                collection.tile_board_tiles.insert(id, pos);
            }
            rusterix::TileSource::TileGroup(id) => {
                collection.tile_board_groups.insert(id, pos);
            }
            _ => {}
        }
    }

    pub fn reserve_tile_board_empty_slot(&mut self, pos: Vec2<i32>) {
        if !self.tile_board_empty_slots.contains(&pos) {
            self.tile_board_empty_slots.push(pos);
        }
    }

    pub fn reserve_collection_tile_board_empty_slot(
        &mut self,
        collection_id: &Uuid,
        pos: Vec2<i32>,
    ) {
        let Some(collection) = self.tile_collections.get_mut(collection_id) else {
            return;
        };
        if !collection.tile_board_empty_slots.contains(&pos) {
            collection.tile_board_empty_slots.push(pos);
        }
    }

    pub fn clear_tile_board_empty_slot(&mut self, pos: Vec2<i32>) {
        if let Some(index) = self.tile_board_empty_slots.iter().position(|p| *p == pos) {
            self.tile_board_empty_slots.swap_remove(index);
        }
    }

    pub fn ensure_tile_board_space(&mut self, pos: Vec2<i32>) {
        if pos.x >= self.tile_board_cols - 1 {
            self.tile_board_cols = pos.x + 2;
        }
        if pos.y >= self.tile_board_rows - 1 {
            self.tile_board_rows = pos.y + 2;
        }
    }

    /// Removes the given avatar from the project.
    pub fn remove_avatar(&mut self, id: &Uuid) {
        self.avatars.shift_remove(id);
    }

    /// Finds the avatar that contains the given animation id.
    pub fn find_avatar_for_animation(&self, animation_id: &Uuid) -> Option<&Avatar> {
        self.avatars
            .values()
            .find(|a| a.animations.iter().any(|anim| anim.id == *animation_id))
    }

    /// Returns an immutable reference to the texture identified by the editing context.
    pub fn get_editing_texture(
        &self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&rusterix::Texture> {
        match editing_ctx {
            PixelEditingContext::None => None,
            PixelEditingContext::Tile(tile_id, frame_index) => {
                let tile = self.tiles.get(tile_id)?;
                tile.textures.get(*frame_index)
            }
            PixelEditingContext::ItemIcon(item_id, frame_index) => {
                let item = self.items.get(item_id)?;
                item.icon_frames.get(*frame_index)
            }
            PixelEditingContext::AvatarFrame(
                avatar_id,
                anim_id,
                perspective_index,
                frame_index,
            ) => {
                let avatar = self.avatars.get(avatar_id)?;
                let anim = avatar.animations.iter().find(|a| a.id == *anim_id)?;
                let perspective = anim.perspectives.get(*perspective_index)?;
                perspective.frames.get(*frame_index).map(|f| &f.texture)
            }
        }
    }

    /// Returns a mutable reference to the texture identified by the editing context.
    pub fn get_editing_texture_mut(
        &mut self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&mut rusterix::Texture> {
        match editing_ctx {
            PixelEditingContext::None => None,
            PixelEditingContext::Tile(tile_id, frame_index) => {
                let tile = self.tiles.get_mut(tile_id)?;
                tile.textures.get_mut(*frame_index)
            }
            PixelEditingContext::ItemIcon(item_id, frame_index) => {
                let item = self.items.get_mut(item_id)?;
                item.icon_frames.get_mut(*frame_index)
            }
            PixelEditingContext::AvatarFrame(
                avatar_id,
                anim_id,
                perspective_index,
                frame_index,
            ) => {
                let avatar = self.avatars.get_mut(avatar_id)?;
                let anim = avatar.animations.iter_mut().find(|a| a.id == *anim_id)?;
                let perspective = anim.perspectives.get_mut(*perspective_index)?;
                perspective
                    .frames
                    .get_mut(*frame_index)
                    .map(|f| &mut f.texture)
            }
        }
    }

    /// Returns an immutable avatar frame for avatar frame editing contexts.
    pub fn get_editing_avatar_frame(
        &self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&rusterix::AvatarAnimationFrame> {
        match editing_ctx {
            PixelEditingContext::AvatarFrame(
                avatar_id,
                anim_id,
                perspective_index,
                frame_index,
            ) => {
                let avatar = self.avatars.get(avatar_id)?;
                let anim = avatar.animations.iter().find(|a| a.id == *anim_id)?;
                let perspective = anim.perspectives.get(*perspective_index)?;
                perspective.frames.get(*frame_index)
            }
            _ => None,
        }
    }

    /// Returns a mutable avatar frame for avatar frame editing contexts.
    pub fn get_editing_avatar_frame_mut(
        &mut self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&mut rusterix::AvatarAnimationFrame> {
        match editing_ctx {
            PixelEditingContext::AvatarFrame(
                avatar_id,
                anim_id,
                perspective_index,
                frame_index,
            ) => {
                let avatar = self.avatars.get_mut(avatar_id)?;
                let anim = avatar.animations.iter_mut().find(|a| a.id == *anim_id)?;
                let perspective = anim.perspectives.get_mut(*perspective_index)?;
                perspective.frames.get_mut(*frame_index)
            }
            _ => None,
        }
    }

    /// Returns an immutable avatar perspective for avatar frame editing contexts.
    pub fn get_editing_avatar_perspective(
        &self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&rusterix::AvatarPerspective> {
        match editing_ctx {
            PixelEditingContext::AvatarFrame(avatar_id, anim_id, perspective_index, _) => {
                let avatar = self.avatars.get(avatar_id)?;
                let anim = avatar.animations.iter().find(|a| a.id == *anim_id)?;
                anim.perspectives.get(*perspective_index)
            }
            _ => None,
        }
    }

    /// Returns a mutable avatar perspective for avatar frame editing contexts.
    pub fn get_editing_avatar_perspective_mut(
        &mut self,
        editing_ctx: &PixelEditingContext,
    ) -> Option<&mut rusterix::AvatarPerspective> {
        match editing_ctx {
            PixelEditingContext::AvatarFrame(avatar_id, anim_id, perspective_index, _) => {
                let avatar = self.avatars.get_mut(avatar_id)?;
                let anim = avatar.animations.iter_mut().find(|a| a.id == *anim_id)?;
                anim.perspectives.get_mut(*perspective_index)
            }
            _ => None,
        }
    }

    /// Add Item
    pub fn add_item(&mut self, item: Item) {
        self.items.insert(item.id, item);
    }

    /// Removes the given item from the project.
    pub fn remove_item(&mut self, id: &Uuid) {
        self.items.shift_remove(id);
    }

    /// Add a tilemap
    pub fn add_tilemap(&mut self, tilemap: Tilemap) {
        self.tilemaps.push(tilemap)
    }

    /// Get the tilemap of the given uuid.
    pub fn get_tilemap(&self, uuid: Uuid) -> Option<&Tilemap> {
        self.tilemaps.iter().find(|t| t.id == uuid)
    }

    /// Get the tilemap of the given uuid.
    pub fn get_tilemap_mut(&mut self, uuid: Uuid) -> Option<&mut Tilemap> {
        self.tilemaps.iter_mut().find(|t| t.id == uuid)
    }

    /// Removes the given tilemap from the project.
    pub fn remove_tilemap(&mut self, id: TheId) {
        self.tilemaps.retain(|item| item.id != id.uuid);
    }

    /// Contains the region of the given uuid.
    pub fn contains_region(&self, uuid: &Uuid) -> bool {
        self.regions.iter().find(|t| t.id == *uuid).is_some()
    }

    /// Get the region of the given uuid.
    pub fn get_region(&self, uuid: &Uuid) -> Option<&Region> {
        self.regions.iter().find(|t| t.id == *uuid)
    }

    /// Get the region of the given uuid as mutable.
    pub fn get_region_mut(&mut self, uuid: &Uuid) -> Option<&mut Region> {
        self.regions.iter_mut().find(|t| t.id == *uuid)
    }

    /// Get the region of the given uuid.
    pub fn get_region_ctx(&self, ctx: &ServerContext) -> Option<&Region> {
        self.regions.iter().find(|t| t.id == ctx.curr_region)
    }

    /// Get the region of the given uuid as mutable.
    pub fn get_region_ctx_mut(&mut self, ctx: &ServerContext) -> Option<&mut Region> {
        self.regions.iter_mut().find(|t| t.id == ctx.curr_region)
    }

    /// Get the screen of the given uuid.
    pub fn get_screen_ctx(&self, ctx: &ServerContext) -> Option<&Screen> {
        self.screens.get(&ctx.curr_screen)
    }

    /// Get the mut screen of the given uuid.
    pub fn get_screen_ctx_mut(&mut self, ctx: &ServerContext) -> Option<&mut Screen> {
        self.screens.get_mut(&ctx.curr_screen)
    }

    /// Remove a region
    pub fn remove_region(&mut self, id: &Uuid) {
        self.regions.retain(|item| item.id != *id);
    }

    /// Get the map of the current context.
    pub fn get_map(&self, ctx: &ServerContext) -> Option<&Map> {
        if ctx.pc.is_prefab() {
            return self.prefab_editor_map.as_ref();
        }
        if ctx.editor_view_mode != EditorViewMode::D2 {
            if let Some(region) = self.get_region(&ctx.curr_region) {
                if ctx.geometry_edit_mode == GeometryEditMode::Detail {
                    if let Some(surface) = ctx.active_detail_surface.as_ref() {
                        if let Some(surface) = region.map.surfaces.get(&surface.id) {
                            if let Some(profile_id) = surface.profile {
                                return region.map.profiles.get(&profile_id);
                            }
                        }
                    }
                }
                return Some(&region.map);
            }
        } else if ctx.get_map_context() == MapContext::Region {
            let id = ctx.curr_region;
            // if let Some(id) = ctx.pc.id() {
            if let Some(surface) = &ctx.editing_surface {
                if let Some(region) = self.regions.iter().find(|t| t.id == id) {
                    if let Some(surface) = region.map.surfaces.get(&surface.id) {
                        if let Some(profile_id) = surface.profile {
                            return region.map.profiles.get(&profile_id);
                        }
                    }
                }
                return None;
            } else if let Some(region) = self.regions.iter().find(|t| t.id == id) {
                return Some(&region.map);
            }
            // }
        } else if ctx.get_map_context() == MapContext::Screen {
            if let Some(id) = ctx.pc.id() {
                if let Some(screen) = self.screens.get(&id) {
                    return Some(&screen.map);
                }
            }
        } else if ctx.get_map_context() == MapContext::Character {
            if let ContentContext::CharacterTemplate(id) = ctx.curr_character {
                if let Some(character) = self.characters.get(&id) {
                    return Some(&character.map);
                }
            }
        } else if ctx.get_map_context() == MapContext::Item {
            if let ContentContext::ItemTemplate(id) = ctx.curr_item {
                if let Some(item) = self.items.get(&id) {
                    return Some(&item.map);
                }
            }
        }
        None
    }

    /// Get the mutable map of the current context.
    pub fn get_map_mut(&mut self, ctx: &ServerContext) -> Option<&mut Map> {
        if ctx.pc.is_prefab() {
            return self.prefab_editor_map.as_mut();
        }
        if ctx.get_map_context() == MapContext::Region {
            let id = ctx.curr_region;
            // if let Some(id) = ctx.pc.id() {
            if ctx.editor_view_mode != EditorViewMode::D2 {
                if let Some(region) = self.get_region_mut(&ctx.curr_region) {
                    if ctx.geometry_edit_mode == GeometryEditMode::Detail {
                        if let Some(surface) = ctx.active_detail_surface.as_ref() {
                            if let Some(surface) = region.map.surfaces.get_mut(&surface.id) {
                                if let Some(profile_id) = surface.profile {
                                    return region.map.profiles.get_mut(&profile_id);
                                }
                            }
                        }
                    }
                    return Some(&mut region.map);
                }
            } else if let Some(surface) = &ctx.editing_surface {
                if let Some(region) = self.regions.iter_mut().find(|t| t.id == id) {
                    if let Some(surface) = region.map.surfaces.get_mut(&surface.id) {
                        if let Some(profile_id) = surface.profile {
                            return region.map.profiles.get_mut(&profile_id);
                        }
                    }
                }
                return None;
            } else if let Some(region) = self.regions.iter_mut().find(|t| t.id == id) {
                return Some(&mut region.map);
            }
            // }
        } else if ctx.get_map_context() == MapContext::Screen {
            if let Some(id) = ctx.pc.id() {
                if let Some(screen) = self.screens.get_mut(&id) {
                    return Some(&mut screen.map);
                }
            }
        } else if ctx.get_map_context() == MapContext::Character {
            if let ContentContext::CharacterTemplate(id) = ctx.curr_character {
                if let Some(character) = self.characters.get_mut(&id) {
                    return Some(&mut character.map);
                }
            }
        } else if ctx.get_map_context() == MapContext::Item {
            if let ContentContext::ItemTemplate(id) = ctx.curr_item {
                if let Some(item) = self.items.get_mut(&id) {
                    return Some(&mut item.map);
                }
            }
        }
        None
    }

    /// Add Screen
    pub fn add_screen(&mut self, screen: Screen) {
        self.screens.insert(screen.id, screen);
    }

    /// Removes the given code from the project.
    pub fn remove_screen(&mut self, id: &Uuid) {
        self.screens.shift_remove(id);
    }

    /// Returns a list of all screens sorted by name.
    pub fn sorted_screens_list(&self) -> Vec<(Uuid, String)> {
        let mut entries: Vec<(Uuid, String)> = self
            .screens
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries
    }

    /// Add an asset
    pub fn add_asset(&mut self, asset: Asset) {
        self.assets.insert(asset.id, asset);
    }

    /// Removes the given code from the project.
    pub fn remove_asset(&mut self, id: &Uuid) {
        self.assets.shift_remove(id);
    }

    /// Returns a list of all assets sorted by name.
    pub fn sorted_assets_list(&self) -> Vec<(Uuid, String)> {
        let mut entries: Vec<(Uuid, String)> = self
            .assets
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries
    }

    /// Removes the given tile from the project.
    pub fn remove_tile(&mut self, id: &Uuid) {
        self.tiles.shift_remove(id);
    }

    /// Gets the given tile from the project.
    pub fn get_tile(&self, id: &Uuid) -> Option<&rusterix::Tile> {
        self.tiles.get(id)
    }

    /// Gets the given mutable tile from the project.
    pub fn get_tile_mut(&mut self, id: &Uuid) -> Option<&mut rusterix::Tile> {
        self.tiles.get_mut(id)
    }

    pub fn find_tile_id_by_alias(&self, alias: &str) -> Option<Uuid> {
        let needle = alias.trim();
        if needle.is_empty() {
            return None;
        }

        let matches_alias = |value: &str| {
            value
                .split([',', ';', '\n'])
                .map(str::trim)
                .any(|part| !part.is_empty() && part.eq_ignore_ascii_case(needle))
        };

        for (id, tile) in &self.tiles {
            if matches_alias(&tile.alias) {
                return Some(*id);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::{
        BlockPropAsset, BlockPropFaceRef, BlockPropOccupancyPolicy, BlockPropOccupant,
        BlockPropSemanticShape, BlockPropSupportSurface, BlockPropSurfacePlacement, Entity,
        GeometryObject, PixelSource, RegionCtx, Sector, Value, identity_block_prop_transform,
    };

    fn official_ruleset_item_count() -> usize {
        let rules = crate::rulesets::resolve_project_rules(
            crate::rulesets::DEFAULT_RULESET_CONFIG,
            crate::rulesets::DEFAULT_RULES_OVERRIDE,
        )
        .unwrap();
        crate::rulesets::ruleset_item_templates_from_source(&rules)
            .unwrap()
            .len()
    }

    fn official_ruleset_tile_count() -> usize {
        crate::rulesets::bundled_tiles_for_project(crate::rulesets::DEFAULT_RULESET_CONFIG)
            .unwrap()
            .len()
    }

    #[test]
    fn project_starts_with_endesga_art_palette() {
        let project = Project::new();

        assert_eq!(project.art_palette.colors.len(), 256);
        assert_eq!(project.art_palette_visible_color_count(), 64);
        assert_eq!(
            project.art_palette.colors[0]
                .as_ref()
                .map(TheColor::to_hex)
                .as_deref(),
            Some("#FF0040")
        );
        assert_eq!(
            project.art_palette.colors[63]
                .as_ref()
                .map(TheColor::to_hex)
                .as_deref(),
            Some("#571C27")
        );
        assert!(project.art_palette.colors[64].is_none());
    }

    #[test]
    fn block_prop_catalog_instances_and_surface_placements_round_trip() {
        let mut project = Project::new();
        let object = GeometryObject::box_from_bounds(
            "Table Top",
            Vec3::new(-0.75, 0.7, -0.4),
            Vec3::new(0.75, 0.8, 0.4),
        );
        let object_id = object.id;
        let top_face_id = object.faces[4].id;
        let mut asset = BlockPropAsset::new_authored("Table", vec![object]);
        asset.alias = "furniture/table".to_string();
        asset.category = "Furniture".to_string();
        let asset_id = asset.id;
        let part_id = asset.parts[0].id;
        let surface_id = Uuid::new_v4();
        asset.support_surfaces.push(BlockPropSupportSurface {
            id: surface_id,
            name: "Tabletop".to_string(),
            part_id,
            shape: BlockPropSemanticShape::Faces(vec![BlockPropFaceRef {
                object_id,
                face_id: top_face_id,
            }]),
            snap_spacing: 0.1,
            allowed_item_tags: vec!["placeable".to_string()],
            capacity: Some(6),
            occupancy_policy: BlockPropOccupancyPolicy::RejectOverlap,
        });
        project.block_props.insert(asset_id, asset);
        let mut prefab_paint = IsoPaintLayer::default();
        prefab_paint
            .chunks
            .insert("tabletop".to_string(), IsoPaintChunk::new([0, 0]));
        project.block_prop_paint.insert(asset_id, prefab_paint);
        project
            .prefab_editor_part_by_object
            .insert(object_id, part_id);

        let instance = rusterix::BlockPropInstance::new(asset_id);
        let instance_id = instance.id;
        project.regions[0].map.block_prop_instances.push(instance);
        let mut local_transform = identity_block_prop_transform();
        local_transform[3][1] = 0.05;
        project.regions[0]
            .map
            .block_prop_surface_placements
            .push(BlockPropSurfacePlacement {
                id: Uuid::new_v4(),
                prop_instance_id: instance_id,
                surface_id,
                occupant: BlockPropOccupant::Item(7),
                local_transform,
            });

        let serialized = serde_json::to_string(&project).expect("serialize block/prop project");
        let restored: Project =
            serde_json::from_str(&serialized).expect("deserialize block/prop project");

        let restored_asset = restored
            .block_props
            .get(&asset_id)
            .expect("restored block/prop asset");
        assert_eq!(restored_asset.name, "Table");
        assert!(restored_asset.find_support_surface(surface_id).is_some());
        assert!(
            restored.block_prop_paint[&asset_id]
                .chunks
                .contains_key("tabletop")
        );
        assert!(restored.prefab_editor_part_by_object.is_empty());
        assert_eq!(restored.regions[0].map.block_prop_instances.len(), 1);
        assert_eq!(
            restored.regions[0].map.block_prop_instances[0].asset_id,
            asset_id
        );
        assert_eq!(
            restored.regions[0].map.block_prop_surface_placements[0].occupant,
            BlockPropOccupant::Item(7)
        );
        assert_eq!(
            restored.regions[0].map.block_prop_surface_placements[0].local_transform[3][1],
            0.05
        );
    }

    #[test]
    fn project_imports_hex_text_into_art_palette() {
        let mut project = Project::new();
        project.art_palette.current_index = 8;

        project.load_art_palette_from_text("112233\n#445566\n778899\n".to_string());

        assert_eq!(project.art_palette_visible_color_count(), 3);
        assert_eq!(project.art_palette.current_index, 2);
        assert_eq!(
            project.art_palette.colors[0]
                .as_ref()
                .map(TheColor::to_hex)
                .as_deref(),
            Some("#112233")
        );
        assert_eq!(
            project.art_palette.colors[1]
                .as_ref()
                .map(TheColor::to_hex)
                .as_deref(),
            Some("#445566")
        );
        assert!(project.art_palette.colors[3].is_none());
        assert_eq!(project.art_palette_materials.len(), 256);
    }

    #[test]
    fn palette_material_serializes_presets_not_raw_rmoe() {
        let mut material = PaletteMaterial {
            preset: "default".to_string(),
            finish: "natural".to_string(),
            roughness: 0.0,
            metallic: 1.0,
            opacity: 0.2,
            emissive: 1.0,
        };

        assert_eq!(material.rmoe_values(), [0.5, 0.0, 1.0, 0.0]);

        material.preset = "metal".to_string();
        material.finish = "polished".to_string();
        let serialized = toml::to_string(&material).unwrap();

        assert!(serialized.contains("preset = \"metal\""));
        assert!(serialized.contains("finish = \"polished\""));
        assert!(!serialized.contains("roughness"));
        assert!(!serialized.contains("metallic"));
        assert!(!serialized.contains("opacity"));
        assert!(!serialized.contains("emissive"));
    }

    #[test]
    fn project_can_load_3d_starter_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/3DStarter.eldiron");
        if !path.exists() {
            return;
        }

        let contents = std::fs::read_to_string(path).expect("read 3D starter fixture");
        let project: Project =
            serde_json::from_str(&contents).expect("3D starter fixture deserializes");

        assert!(
            !project.regions.is_empty(),
            "3D starter fixture should contain at least one region"
        );
    }

    #[test]
    fn project_can_load_hideout2d_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Hideout2D.eldiron");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read Hideout2D fixture '{}': {err}", path.display()));
        let project: Project = serde_json::from_str(&contents)
            .unwrap_or_else(|err| panic!("Hideout2D fixture deserializes: {err}"));

        assert!(
            project
                .screens
                .values()
                .any(|screen| screen.map.name == "Start"),
            "Hideout2D fixture should contain the Start screen"
        );
    }

    #[test]
    fn hideout2d_tagged_tile_events_work_under_a_named_sector() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Hideout2D.eldiron");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read Hideout2D fixture '{}': {err}", path.display()));
        let mut project: Project = serde_json::from_str(&contents)
            .unwrap_or_else(|err| panic!("Hideout2D fixture deserializes: {err}"));

        let (region_index, sector_index, tile_id) = project
            .regions
            .iter()
            .enumerate()
            .find_map(|(region_index, region)| {
                region
                    .map
                    .sectors
                    .iter()
                    .enumerate()
                    .find_map(|(sector_index, sector)| {
                        if sector.layer.is_none() {
                            return None;
                        }
                        match sector.properties.get_default_source() {
                            Some(PixelSource::TileId(tile_id))
                                if project.tiles.contains_key(tile_id) =>
                            {
                                Some((region_index, sector_index, *tile_id))
                            }
                            _ => None,
                        }
                    })
            })
            .expect("Hideout2D contains a painted 2D tile sector");

        project.tiles.get_mut(&tile_id).unwrap().gameplay_tags = vec!["chair".into()];

        // Exercise the same JSON round-trip used by real .eldiron projects.
        let serialized = serde_json::to_string(&project).expect("serialize tagged 2D project");
        let roundtrip: Project =
            serde_json::from_str(&serialized).expect("reload tagged 2D project");
        assert_eq!(
            roundtrip.tiles[&tile_id].gameplay_tags,
            vec!["chair".to_string()]
        );

        let mut ctx = RegionCtx::default();
        ctx.map = roundtrip.regions[region_index].map.clone();
        ctx.assets.set_tiles(roundtrip.tiles.clone());

        let painted_sector = ctx.map.sectors[sector_index].clone();
        let chair_position = painted_sector
            .center(&ctx.map)
            .expect("painted tile sector has a center");

        // Add the logical sector users previously had to keep synchronized with
        // the painted tile. Tile events must still see the tile underneath it.
        let mut named_sector = painted_sector;
        named_sector.id = ctx.map.find_free_sector_id().unwrap();
        named_sector.creator_id = Uuid::new_v4();
        named_sector.name = "Chair Area".into();
        named_sector.layer = None;
        named_sector
            .properties
            .set("source", Value::Source(PixelSource::Off));
        ctx.map.sectors.push(named_sector);

        let mut entity = Entity::new();
        entity.id = 42;
        entity.set_pos_xz(chair_position);
        ctx.map.entities.push(entity);

        ctx.check_player_for_section_change_id(42);
        assert!(ctx.to_execute_entity.iter().any(|(id, event, value)| {
            *id == 42 && event == "entered" && value.as_string() == Some("Chair Area")
        }));
        assert!(ctx.to_execute_entity.iter().any(|(id, event, value)| {
            *id == 42 && event == "entered_tile" && value.as_string() == Some("chair")
        }));

        ctx.to_execute_entity.clear();
        ctx.check_player_for_section_change_id(42);
        assert!(
            ctx.to_execute_entity
                .iter()
                .all(|(_, event, _)| event != "entered_tile" && event != "left_tile")
        );

        ctx.map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 42)
            .unwrap()
            .set_pos_xz(chair_position + Vec2::new(10_000.0, 10_000.0));
        ctx.check_player_for_section_change_id(42);
        assert!(ctx.to_execute_entity.iter().any(|(id, event, value)| {
            *id == 42 && event == "left_tile" && value.as_string() == Some("chair")
        }));
    }

    #[test]
    fn hideout2d_resolves_through_current_ruleset_model() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Hideout2D.eldiron");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read Hideout2D fixture '{}': {err}", path.display()));
        let mut project: Project = serde_json::from_str(&contents)
            .unwrap_or_else(|err| panic!("Hideout2D fixture deserializes: {err}"));
        let selection = crate::rulesets::selected_ruleset_config(&project.config);
        assert_eq!(selection.id, crate::rulesets::OFFICIAL_RULESET_ID);
        assert_eq!(selection.version, crate::rulesets::OFFICIAL_RULESET_VERSION);
        assert_eq!(
            selection.schema_version,
            crate::rulesets::OFFICIAL_RULESET_SCHEMA_VERSION
        );
        assert_eq!(selection.source, "official");

        let resolved = crate::rulesets::resolve_project_ruleset(&project.config, &project.rules)
            .unwrap_or_else(|err| panic!("Hideout2D resolves its ruleset: {err}"));
        assert_eq!(resolved.metadata().id, crate::rulesets::OFFICIAL_RULESET_ID);
        assert_eq!(
            resolved.metadata().schema_version,
            crate::rulesets::OFFICIAL_RULESET_SCHEMA_VERSION
        );
        assert!(
            resolved.validation().is_ok(),
            "Hideout2D should resolve without ruleset validation errors: {:?}",
            resolved.validation().issues
        );
        assert!(resolved.table().get("actions").is_some());
        assert!(resolved.table().get("classes").is_some());
        assert!(resolved.table().get("items").is_some());
        let basic_attack = resolved
            .action("basic_attack")
            .expect("Hideout2D resolves typed Basic Attack")
            .expect("Hideout2D includes Basic Attack");
        assert_eq!(
            basic_attack.kind,
            crate::rulesets::ResolvedActionKind::Attack
        );
        assert_eq!(
            basic_attack.damage_source(),
            Some(&crate::rulesets::ResolvedActionValueSource::Weapon)
        );
        let minor_heal = resolved
            .action("minor_heal")
            .expect("Hideout2D resolves typed Minor Heal")
            .expect("Hideout2D includes Minor Heal");
        assert_eq!(minor_heal.required_spell(), Some("minor_heal"));
        assert!(matches!(
            minor_heal.healing_source(),
            Some(crate::rulesets::ResolvedActionValueSource::RulesetPath(path))
                if path == "spells.minor_heal.healing"
        ));
        let holy_light = resolved
            .action("holy_light")
            .expect("Hideout2D resolves typed Holy Light")
            .expect("Hideout2D includes Holy Light");
        assert_eq!(holy_light.required_spell(), Some("holy_light"));
        assert!(matches!(
            holy_light.damage_source(),
            Some(crate::rulesets::ResolvedActionValueSource::RulesetPath(path))
                if path == "spells.holy_light.damage"
        ));

        assert!(
            project.characters.values().any(|character| {
                character.name == "Skeleton"
                    && character.data.contains("avatar = \"skeleton\"")
                    && character.data.contains("race = \"Skeleton\"")
                    && character.data.contains("class = \"Warrior\"")
            }),
            "Hideout2D should contain its ruleset-backed Skeleton"
        );
        for character_name in [
            "Bone Warden",
            "Bone Archer",
            "Warden Mara",
            "Brother Corvin",
            "Quartermaster Nessa",
        ] {
            assert!(
                project
                    .characters
                    .values()
                    .any(|character| character.name == character_name),
                "Hideout2D should contain {character_name}"
            );
        }
        for item_name in [
            "Grave Sigil",
            "Old Grave",
            "Sunstone Outcrop",
            "Moonleaf Patch",
        ] {
            assert!(
                project.items.values().any(|item| item.name == item_name),
                "Hideout2D should contain {item_name}"
            );
        }
        assert!(
            project
                .characters
                .values()
                .find(|character| character.name == "Brother Corvin")
                .is_some_and(|character| character.data.contains("SAR IR")),
            "Hideout2D should connect Words of Power to the dungeon quest"
        );
        let player_data = project
            .characters
            .values()
            .find(|character| character.name == "Player")
            .map(|character| character.data.as_str())
            .expect("Hideout2D contains its Player template");
        for shortcut in [
            "u = \"intent.use\"",
            "l = \"intent.look\"",
            "t = \"rules.basic_attack\"",
            "k = \"rules.take\"",
            "tab = \"ui.actions\"",
        ] {
            assert!(
                player_data.contains(shortcut),
                "Hideout2D Player should contain shortcut {shortcut}"
            );
        }
        assert!(
            project.screens.values().any(|screen| {
                screen.name == "Game"
                    && screen.map.sectors.iter().any(|sector| {
                        sector.name == "Actions"
                            && sector
                                .properties
                                .get_str("data")
                                .is_some_and(|data| data.contains("command = \"ui.actions\""))
                    })
            }),
            "Hideout2D should expose its ruleset action catalogue from the Game screen"
        );
        assert!(
            !crate::rulesets::bundled_avatars_for_project(&project.config)
                .expect("official avatars resolve")
                .is_empty()
        );
        assert!(
            !crate::rulesets::bundled_textures_for_project(&project.config)
                .expect("official icons resolve")
                .is_empty()
        );
        project
            .sync_ruleset_items()
            .unwrap_or_else(|err| panic!("Hideout2D syncs ruleset items: {err}"));
        assert_eq!(
            std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("re-read '{}': {err}", path.display())),
            contents,
            "Hideout2D model checks must not rewrite the fixture"
        );
    }

    #[test]
    fn project_can_load_gate_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/Gate.eldiron");
        if !path.exists() {
            return;
        }

        let contents = std::fs::read_to_string(path).expect("read Gate fixture");
        let project: Project = serde_json::from_str(&contents).expect("Gate fixture deserializes");

        assert!(
            project
                .regions
                .iter()
                .any(|region| region.name == "StartScene"),
            "Gate fixture should contain the StartScene region"
        );
    }

    #[test]
    fn old_project_gets_default_ruleset_and_empty_rules_override() {
        let mut project = Project::new();
        project.config = "[game]\nname = \"Old Project\"\n".to_string();
        project.rules = "[combat]\nincoming_damage = \"old\"\n".to_string();

        assert!(project.migrate_default_ruleset());

        assert!(crate::rulesets::has_top_level_ruleset(&project.config));
        assert_eq!(project.rules, crate::rulesets::DEFAULT_RULES_OVERRIDE);
    }

    #[test]
    fn project_with_ruleset_keeps_rules_override() {
        let mut project = Project::new();
        project.config = crate::rulesets::DEFAULT_RULESET_CONFIG.to_string();
        project.rules = "[actions.minor_heal]\ncost = { MP = 3 }\n".to_string();

        assert!(!project.migrate_default_ruleset());

        assert_eq!(project.rules, "[actions.minor_heal]\ncost = { MP = 3 }\n");
    }

    #[test]
    fn project_migrates_legacy_button_fields_to_commands() {
        let mut project = Project::new();
        let mut screen = crate::screen::Screen::new();

        let mut attack = Sector::new(1, vec![]);
        attack.properties.set(
            "data",
            Value::Str("[ui]\nrole = \"button\"\nintent = \"attack\"\n".into()),
        );
        let mut forward = Sector::new(2, vec![]);
        forward.properties.set(
            "data",
            Value::Str("[ui]\nrole = \"button\"\naction = \"forward\"\n".into()),
        );
        let mut spell = Sector::new(3, vec![]);
        spell.properties.set(
            "data",
            Value::Str(
                "[ui]\nrole = \"button\"\nintent = \"spell\"\nspell = \"minor_heal\"\n".into(),
            ),
        );

        screen.map.sectors.push(attack);
        screen.map.sectors.push(forward);
        screen.map.sectors.push(spell);
        project.screens.insert(screen.id, screen);

        assert!(project.migrate_button_commands());
        assert!(!project.migrate_button_commands());

        let data = project
            .screens
            .values()
            .next()
            .unwrap()
            .map
            .sectors
            .iter()
            .filter_map(|sector| sector.properties.get_str("data"))
            .collect::<Vec<_>>();
        assert!(
            data.iter()
                .any(|data| data.contains("command = \"intent.attack\""))
        );
        assert!(
            data.iter()
                .any(|data| data.contains("command = \"control.forward\""))
        );
        assert!(
            data.iter()
                .any(|data| data.contains("command = \"intent.spell:minor_heal\""))
        );
    }

    #[test]
    fn project_creates_missing_ruleset_items_once() {
        let mut project = Project::new();
        project.config = crate::rulesets::DEFAULT_RULESET_CONFIG.to_string();
        project.rules = crate::rulesets::DEFAULT_RULES_OVERRIDE.to_string();

        assert_eq!(
            project.sync_ruleset_items().unwrap(),
            official_ruleset_item_count() + official_ruleset_tile_count()
        );
        assert_eq!(project.sync_ruleset_items().unwrap(), 0);
        assert_eq!(
            project.palette.colors[2]
                .as_ref()
                .map(TheColor::to_hex)
                .as_deref(),
            Some("#BCAD9F")
        );
        assert!(project.ruleset_palette_is_active());
        assert!(project.items.values().any(|item| {
            item.name == "Training Sword"
                && item
                    .data
                    .contains("ruleset_path = \"items.weapons.training_sword\"")
        }));
        assert!(
            project
                .tiles
                .contains_key(&Uuid::parse_str("05ab6adc-1631-4ed2-9857-f85820a7f1ad").unwrap())
        );
        assert!(
            project
                .tiles
                .contains_key(&Uuid::parse_str("f76473d1-70f6-4649-8b0d-cbac627f93d8").unwrap())
        );

        let linen_shirt = project
            .items
            .values_mut()
            .find(|item| item_ruleset_path(item).as_deref() == Some("items.clothing.linen_shirt"))
            .unwrap();
        linen_shirt.data =
            "[attributes]\nruleset_path = \"items.clothing.linen_shirt\"\n".to_string();

        assert_eq!(project.sync_ruleset_items().unwrap(), 1);
        assert!(project.items.values().any(|item| {
            item_ruleset_path(item).as_deref() == Some("items.clothing.linen_shirt")
                && item.data.contains("torso_index = 2")
                && item.data.contains("arms_index = 2")
        }));
    }

    #[test]
    fn project_sorts_ruleset_items_below_custom_items_by_display_name() {
        let mut project = Project::new();
        let mut custom = Item::new();
        custom.name = "Custom Idol".to_string();
        let custom_id = custom.id;
        project.add_item(custom);
        project.config = crate::rulesets::DEFAULT_RULESET_CONFIG.to_string();
        project.rules = crate::rulesets::DEFAULT_RULES_OVERRIDE.to_string();

        assert_eq!(
            project.sync_ruleset_items().unwrap(),
            official_ruleset_item_count() + official_ruleset_tile_count()
        );
        assert_eq!(project.items.first().map(|(id, _)| *id), Some(custom_id));
        let names = project
            .items
            .values()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>();
        let green_wood = names.iter().position(|name| *name == "Green Wood").unwrap();
        let green_wood_node = names
            .iter()
            .position(|name| *name == "Green Wood Node")
            .unwrap();
        let wild_herb = names.iter().position(|name| *name == "Wild Herb").unwrap();
        let wild_herb_node = names
            .iter()
            .position(|name| *name == "Wild Herb Node")
            .unwrap();

        assert_eq!(green_wood_node, green_wood + 1);
        assert_eq!(wild_herb_node, wild_herb + 1);
    }
}
