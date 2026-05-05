use crate::prelude::*;
use buildergraph::BuilderGraph;
use codegridfx::Module;
use indexmap::IndexMap;
pub use rusterix::map::*;
use theframework::prelude::*;
use tilegraph::TileGraphPaletteSource;

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

fn default_world_module() -> Module {
    Module::as_type(codegridfx::ModuleType::World)
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaletteMaterial {
    #[serde(default = "default_palette_roughness")]
    pub roughness: f32,
    #[serde(default = "default_palette_metallic")]
    pub metallic: f32,
    #[serde(default = "default_palette_opacity")]
    pub opacity: f32,
    #[serde(default = "default_palette_emissive")]
    pub emissive: f32,
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
            roughness: default_palette_roughness(),
            metallic: default_palette_metallic(),
            opacity: default_palette_opacity(),
            emissive: default_palette_emissive(),
        }
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeGroupAsset {
    pub group_id: Uuid,
    pub graph_id: Uuid,
    #[serde(default)]
    pub graph_name: String,
    pub output_grid_width: u16,
    pub output_grid_height: u16,
    pub tile_pixel_width: u16,
    pub tile_pixel_height: u16,
    #[serde(default)]
    pub palette_source: TileGraphPaletteSource,
    #[serde(default)]
    pub palette_colors: Vec<TheColor>,
    #[serde(default)]
    pub graph_data: String,
}

impl NodeGroupAsset {
    pub fn new(
        group_id: Uuid,
        output_grid_width: u16,
        output_grid_height: u16,
        palette_colors: Vec<TheColor>,
    ) -> Self {
        Self {
            group_id,
            graph_id: Uuid::new_v4(),
            graph_name: "New Node Graph".to_string(),
            output_grid_width,
            output_grid_height,
            tile_pixel_width: 32,
            tile_pixel_height: 32,
            palette_source: TileGraphPaletteSource::Local,
            palette_colors,
            graph_data: String::new(),
        }
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

    /// Node-backed tile groups keyed by tile-group id.
    #[serde(default)]
    pub tile_node_groups: IndexMap<Uuid, NodeGroupAsset>,

    /// Standalone builder graphs for props and assemblies.
    #[serde(default)]
    pub builder_graphs: IndexMap<Uuid, BuilderGraphAsset>,

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

    #[serde(default)]
    pub palette: ThePalette,

    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    #[serde(default = "default_tick_ms")]
    pub tick_ms: u32,

    #[serde(default)]
    pub config: String,

    #[serde(default = "default_world_module")]
    pub world_module: Module,

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

    #[serde(default)]
    pub avatars: IndexMap<Uuid, Avatar>,

    #[serde(default = "default_palette_material_slots")]
    pub palette_materials: Vec<PaletteMaterial>,
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
            tile_node_groups: IndexMap::default(),
            builder_graphs: IndexMap::default(),
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

            palette: ThePalette::default(),

            target_fps: default_target_fps(),
            tick_ms: default_tick_ms(),

            avatars: IndexMap::default(),
            palette_materials: default_palette_material_slots(),

            config: String::new(),
            world_module: default_world_module(),
            world_source: String::new(),
            world_source_debug: String::new(),
            rules: default_rules(),
            locales: default_locales(),
            audio_fx: default_audio_fx(),
            authoring: default_authoring(),
        }
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

    pub fn add_tile_node_group(&mut self, node_group: NodeGroupAsset) {
        self.tile_node_groups
            .insert(node_group.group_id, node_group);
    }

    pub fn add_builder_graph(&mut self, builder_graph: BuilderGraphAsset) {
        self.builder_graphs.insert(builder_graph.id, builder_graph);
    }

    pub fn add_tile_collection(&mut self, collection: TileCollectionAsset) {
        self.tile_collections.insert(collection.id, collection);
    }

    pub fn is_tile_node_group(&self, id: &Uuid) -> bool {
        self.tile_node_groups.contains_key(id)
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
        self.tile_node_groups.shift_remove(id);
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
