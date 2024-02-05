use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum RegionType {
    Region2D,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Region {
    pub id: Uuid,
    pub region_type: RegionType,

    pub name: String,

    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<(i32, i32), RegionTile>,

    #[serde(default)]
    pub areas: FxHashMap<Uuid, Area>,

    #[serde(default)]
    pub characters: FxHashMap<Uuid, Character>,

    #[serde(default)]
    pub items: FxHashMap<Uuid, Item>,

    pub width: i32,
    pub height: i32,
    pub grid_size: i32,
    pub scroll_offset: Vec2i,
    pub zoom: f32,
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl Region {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            region_type: RegionType::Region2D,

            name: "New Region".to_string(),
            tiles: FxHashMap::default(),

            areas: FxHashMap::default(),
            characters: FxHashMap::default(),
            items: FxHashMap::default(),

            width: 80,
            height: 80,
            grid_size: 24,
            scroll_offset: Vec2i::zero(),
            zoom: 1.0,
        }
    }

    /// Set the tile of the given position and role.
    pub fn set_tile(&mut self, pos: (i32, i32), role: Layer2DRole, tile: Option<Uuid>) {
        if let Some(t) = self.tiles.get_mut(&pos) {
            t.layers[role as usize] = tile;
        } else {
            let mut region_tile = RegionTile::default();
            region_tile.layers[role as usize] = tile;
            self.tiles.insert(pos, region_tile);
        }
    }

    /// Returns true if the character can move to the given position.
    pub fn can_move_to(&self, pos: Vec3f, tiles: &FxHashMap<Uuid, TheRGBATile>, update: &RegionUpdate) -> bool {
        let mut can_move = true;
        let pos = vec2i(pos.x as i32, pos.z as i32);

        if pos.x < 0 || pos.y < 0 {
            return false;
        }

        if pos.x >= self.width || pos.y >= self.height {
            return false;
        }

        if let Some(tile) = self.tiles.get(&(pos.x, pos.y)) {
            for index in 0..tile.layers.len() {
                if let Some(layer) = tile.layers[index] {
                    if let Some(t) = tiles.get(&layer) {
                        if t.blocking && index == Layer2DRole::Wall as usize {

                            can_move = false;

                            if let Some(wallfx) = update.wallfx.get(&(pos.x, pos.y)){
                                if wallfx.fx != WallFX::Normal {
                                    can_move = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        can_move
    }

    /// Fills a code level with the blocking tiles of the region.
    pub fn fill_code_level(&self, level: &mut TheCodeLevel, tiles: &FxHashMap<Uuid, TheRGBATile>) {
        level.clear_blocking();
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = (x, y);
                if let Some(tile) = self.tiles.get(&pos) {
                    for layer in tile.layers.iter().flatten() {
                        if let Some(t) = tiles.get(layer) {
                            if t.blocking {
                                level.set_blocking((x as u16, y as u16));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(Region::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Layer2DRole {
    Ground,
    Wall,
    Ceiling,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionTile {
    // WallFX (name, position, alpha, delta)
    pub wallfx: Option<(String, Vec2f, f32, f32)>,

    pub layers: Vec<Option<Uuid>>,
}

impl Default for RegionTile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionTile {
    pub fn new() -> Self {
        Self {
            wallfx: None,

            layers: vec![None, None, None],
        }
    }
}
