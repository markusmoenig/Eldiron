use crate::prelude::*;
use indexmap::IndexMap;
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
    pub characters: FxHashMap<Uuid, TheCodeBundle>,
    #[serde(default)]
    pub items: FxHashMap<Uuid, TheCodeBundle>,
    #[serde(default)]
    pub codes: FxHashMap<Uuid, TheCodeBundle>,

    #[serde(default)]
    pub screens: FxHashMap<Uuid, Screen>,

    #[serde(default)]
    pub assets: FxHashMap<Uuid, Asset>,

    #[serde(default)]
    pub models: Vec<ModelFX>,

    #[serde(default)]
    pub palette: ThePalette,

    #[serde(default)]
    pub materials: IndexMap<Uuid, MaterialFXObject>,

    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    #[serde(default = "default_tick_ms")]
    pub tick_ms: u32,
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        Self {
            name: String::new(),

            regions: vec![],
            tilemaps: vec![],

            time: TheTime::default(),
            map_mode: MapMode::default(),

            characters: FxHashMap::default(),
            items: FxHashMap::default(),
            codes: FxHashMap::default(),

            screens: FxHashMap::default(),
            assets: FxHashMap::default(),
            models: vec![],

            palette: ThePalette::default(),
            materials: IndexMap::default(),

            target_fps: default_target_fps(),
            tick_ms: default_tick_ms(),
        }
    }

    /// Add Character
    pub fn add_character(&mut self, character: TheCodeBundle) {
        self.characters.insert(character.id, character);
    }

    /// Removes the given character from the project.
    pub fn remove_character(&mut self, id: &Uuid) {
        self.characters.remove(id);
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
    pub fn add_item(&mut self, item: TheCodeBundle) {
        self.items.insert(item.id, item);
    }

    /// Removes the given item from the project.
    pub fn remove_item(&mut self, id: &Uuid) {
        self.items.remove(id);
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

    /// Remove a region
    pub fn remove_region(&mut self, id: &Uuid) {
        self.regions.retain(|item| item.id != *id);
    }

    /// Add Code
    pub fn add_code(&mut self, code: TheCodeBundle) {
        self.codes.insert(code.id, code);
    }

    /// Removes the given code from the project.
    pub fn remove_code(&mut self, id: &Uuid) {
        self.codes.remove(id);
    }

    /// Returns a list of all codes sorted by name.
    pub fn sorted_code_list(&self) -> Vec<(Uuid, String)> {
        let mut entries: Vec<(Uuid, String)> = self
            .codes
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries
    }

    /// Add Screen
    pub fn add_screen(&mut self, screen: Screen) {
        self.screens.insert(screen.id, screen);
    }

    /// Removes the given code from the project.
    pub fn remove_screen(&mut self, id: &Uuid) {
        self.screens.remove(id);
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
        self.assets.remove(id);
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
                rgba_tile.blocking = tile.blocking;
                rgba_tile.billboard = tile.billboard;
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
                rgba_tile.blocking = tile.blocking;
                rgba_tile.billboard = tile.billboard;
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
                    rgba_tile.blocking = tile.blocking;
                    rgba_tile.billboard = tile.billboard;
                    return Some(rgba_tile);
                }
            }
        }
        None
    }

    /// Get the tile in the region at the given position.
    pub fn extract_region_tile(&self, region_id: Uuid, pos: (i32, i32)) -> Option<TheRGBATile> {
        if let Some(region) = self.get_region(&region_id) {
            if let Some(tile) = region.tiles.get(&pos) {
                if let Some(id) = tile.layers[Layer2DRole::Wall as usize] {
                    if let Some(t) = self.get_tile(&id) {
                        return self.extract_tile(&t.id);
                    }
                } else if let Some(id) = tile.layers[Layer2DRole::Ground as usize] {
                    if let Some(t) = self.get_tile(&id) {
                        return self.extract_tile(&t.id);
                    }
                }
            }
        }
        None
    }
}
