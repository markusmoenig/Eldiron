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
            layers: vec![None, None, None],
        }
    }
}
