use crate::prelude::*;
use indexmap::IndexMap;
pub use rusterix::map::*;
use theframework::prelude::*;

/// The default target fps for the game.
fn default_target_fps() -> u32 {
    30
}

/// The default ms per tick for the game.
fn default_tick_ms() -> u32 {
    250
}

#[derive(Serialize, Deserialize, Default, Copy, Clone, Debug)]
pub enum MapMode {
    #[default]
    TwoD,
    Mixed,
    ThreeD,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub regions: Vec<Region>,
    pub tilemaps: Vec<Tilemap>,

    #[serde(default)]
    pub time: TheTime,

    #[serde(default)]
    pub map_mode: MapMode,

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

    #[serde(default)]
    pub materials: IndexMap<Uuid, Map>,

    #[serde(default)]
    pub models: IndexMap<Uuid, Map>,

    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    #[serde(default = "default_tick_ms")]
    pub tick_ms: u32,

    #[serde(default)]
    pub config: String,
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        let mut materials = IndexMap::default();
        let map = Map {
            name: "Unnamed Material".to_string(),
            ..Default::default()
        };
        materials.insert(map.id, map);

        Self {
            name: String::new(),

            regions: vec![],
            tilemaps: vec![],

            time: TheTime::default(),
            map_mode: MapMode::default(),

            characters: IndexMap::default(),
            items: IndexMap::default(),

            screens: IndexMap::default(),
            assets: IndexMap::default(),

            palette: ThePalette::default(),
            materials,
            models: IndexMap::default(),

            target_fps: default_target_fps(),
            tick_ms: default_tick_ms(),

            config: String::new(),
        }
    }

    /// Add Character
    pub fn add_character(&mut self, character: Character) {
        self.characters.insert(character.id, character);
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
    pub fn get_tilemap(&mut self, uuid: Uuid) -> Option<&mut Tilemap> {
        self.tilemaps.iter_mut().find(|t| t.id == uuid)
    }

    /// Removes the given tilemap from the project.
    pub fn remove_tilemap(&mut self, id: TheId) {
        self.tilemaps.retain(|item| item.id != id.uuid);
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

    /// Remove a region
    pub fn remove_region(&mut self, id: &Uuid) {
        self.regions.retain(|item| item.id != *id);
    }

    /// Remove a model
    pub fn remove_model(&mut self, id: &Uuid) {
        #[allow(deprecated)]
        self.models.remove(id);
    }

    /// Get the map of the current context.
    pub fn get_map(&self, ctx: &ServerContext) -> Option<&Map> {
        if ctx.curr_map_context == MapContext::Region {
            if let Some(region) = self.regions.iter().find(|t| t.id == ctx.curr_region) {
                return Some(&region.map);
            }
        } else if ctx.curr_map_context == MapContext::Material {
            if let Some(material_id) = ctx.curr_material {
                if let Some(material) = self.materials.get(&material_id) {
                    return Some(material);
                }
            }
        }
        None
    }

    /// Get the mutable map of the current context.
    pub fn get_map_mut(&mut self, ctx: &ServerContext) -> Option<&mut Map> {
        if ctx.curr_map_context == MapContext::Region {
            if let Some(region) = self.regions.iter_mut().find(|t| t.id == ctx.curr_region) {
                return Some(&mut region.map);
            }
        } else if ctx.curr_map_context == MapContext::Material {
            if let Some(material_id) = ctx.curr_material {
                if let Some(material) = self.materials.get_mut(&material_id) {
                    return Some(material);
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
        for tilemap in &mut self.tilemaps {
            tilemap.tiles.retain(|t| t.id != *id);
        }
    }

    /// Gets the given tile from the project.
    pub fn get_tile(&self, id: &Uuid) -> Option<&Tile> {
        for tilemap in &self.tilemaps {
            for tile in &tilemap.tiles {
                if tile.id == *id {
                    return Some(tile);
                }
            }
        }
        None
    }

    /// Gets the given mutable tile from the project.
    pub fn get_tile_mut(&mut self, id: &Uuid) -> Option<&mut Tile> {
        for tilemap in &mut self.tilemaps {
            for tile in &mut tilemap.tiles {
                if tile.id == *id {
                    return Some(tile);
                }
            }
        }
        None
    }

    /// Extract all tiles from all tilemaps and store them in a hash.
    pub fn extract_tiles(&self) -> FxHashMap<Uuid, TheRGBATile> {
        let mut tiles = FxHashMap::default();
        for tilemap in &self.tilemaps {
            for tile in &tilemap.tiles {
                let mut rgba_tile = TheRGBATile::new();
                rgba_tile.id = tile.id;
                rgba_tile.name.clone_from(&tile.name);
                rgba_tile.buffer = tilemap.buffer.extract_sequence(&tile.sequence);
                rgba_tile.role = tile.role as u8;
                rgba_tile.scale = tile.scale;
                rgba_tile.render_mode = tile.render_mode;
                rgba_tile.blocking = tile.blocking;
                tiles.insert(tile.id, rgba_tile);
            }
        }
        tiles
    }

    /// Extract all tiles from all tilemaps and store them in a vec.
    pub fn extract_tiles_vec(&self) -> Vec<TheRGBATile> {
        let mut tiles = vec![];
        for tilemap in &self.tilemaps {
            for tile in &tilemap.tiles {
                let mut rgba_tile = TheRGBATile::new();
                rgba_tile.id = tile.id;
                rgba_tile.name.clone_from(&tile.name);
                rgba_tile.buffer = tilemap.buffer.extract_sequence(&tile.sequence);
                rgba_tile.role = tile.role as u8;
                tiles.push(rgba_tile);
            }
        }
        tiles
    }

    /// Extract the given tile from the tilemaps.
    pub fn extract_tile(&self, id: &Uuid) -> Option<TheRGBATile> {
        for tilemap in &self.tilemaps {
            for tile in &tilemap.tiles {
                if tile.id == *id {
                    let mut rgba_tile = TheRGBATile::new();
                    rgba_tile.id = tile.id;
                    rgba_tile.name.clone_from(&tile.name);
                    rgba_tile.buffer = tilemap.buffer.extract_sequence(&tile.sequence);
                    rgba_tile.role = tile.role as u8;
                    return Some(rgba_tile);
                }
            }
        }
        None
    }
}
