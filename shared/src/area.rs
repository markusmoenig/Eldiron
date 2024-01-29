use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Area {
    pub id: Uuid,

    pub name: String,

    /// The area of the region.
    pub area: FxHashSet<(i32, i32)>,

    /// The tiles of the region which, when activated, replace the region tiles.
    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<(i32, i32), RegionTile>,

    /// The bundle which defines the behavior of the region. "main" is called for every tick.
    pub bundle: TheCodeBundle,
}

impl Default for Area {
    fn default() -> Self {
        Self::new()
    }
}

impl Area {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: "New Area".to_string(),

            area: FxHashSet::default(),
            tiles: FxHashMap::default(),

            bundle: TheCodeBundle::new(),
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

    /// Returns the center position of all the tiles.
    pub fn center(&self) -> Option<(i32, i32)> {
        if self.area.is_empty() {
            return None;
        }

        let mut sum_x = 0i32;
        let mut sum_y = 0i32;

        for (x, y) in &self.area {
            sum_x += x;
            sum_y += y;
        }

        let count = self.area.len() as i32;
        Some((sum_x / count, sum_y / count))
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(Area::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
