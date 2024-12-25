use crate::prelude::*;
use theframework::prelude::*;

fn default_min_brightness() -> f32 {
    0.3
}

fn default_max_brightness() -> f32 {
    1.0
}

fn default_pathtracer_samples() -> i32 {
    30
}

fn default_tile_size() -> i32 {
    36
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum RegionType {
    Region2D,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum CameraMode {
    Pinhole,
    Orthogonal,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Region {
    pub id: Uuid,
    pub region_type: RegionType,

    pub name: String,

    #[serde(default)]
    pub map: Map,

    #[serde(default)]
    pub regionfx: RegionFXObject,

    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<(i32, i32), RegionTile>,

    #[serde(default)]
    #[serde(with = "vectorize")]
    pub effects: FxHashMap<Vec3<i32>, TileFXObject>,

    #[serde(default)]
    pub areas: FxHashMap<Uuid, Area>,

    #[serde(default)]
    pub characters: FxHashMap<Uuid, Character>,

    #[serde(default)]
    pub items: FxHashMap<Uuid, Item>,

    // #[serde(default)]
    // pub prerendered: PreRendered,
    pub width: i32,
    pub height: i32,
    pub grid_size: i32,
    #[serde(default = "default_tile_size")]
    pub tile_size: i32,
    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,

    #[serde(default = "default_pathtracer_samples")]
    pub pathtracer_samples: i32,

    #[serde(default)]
    pub editing_position_3d: Vec3<f32>,

    #[serde(default = "default_min_brightness")]
    pub min_brightness: f32,

    #[serde(default = "default_max_brightness")]
    pub max_brightness: f32,

    #[serde(default)]
    pub property_1: String,
    #[serde(default)]
    pub property_2: String,
    #[serde(default)]
    pub property_3: String,
    #[serde(default)]
    pub property_4: String,
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Region {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            region_type: RegionType::Region2D,

            name: "New Region".to_string(),

            map: Map::default(),

            regionfx: RegionFXObject::default(),

            tiles: FxHashMap::default(),

            effects: FxHashMap::default(),

            areas: FxHashMap::default(),
            characters: FxHashMap::default(),
            items: FxHashMap::default(),

            // prerendered: PreRendered::default(),
            width: 80,
            height: 80,
            grid_size: 24,
            tile_size: 36,
            scroll_offset: Vec2::zero(),
            zoom: 1.0,

            pathtracer_samples: default_pathtracer_samples(),

            editing_position_3d: Vec3::zero(),

            min_brightness: 0.3,
            max_brightness: 1.0,

            property_1: String::default(),
            property_2: String::default(),
            property_3: String::default(),
            property_4: String::default(),
        }
    }

    /// Calculate the min / max positions of the tiles.
    pub fn min_max(&self) -> Option<(Vec2<i32>, Vec2<i32>)> {
        if self.tiles.is_empty() {
            return None;
        }

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for &(x, y) in self.tiles.keys() {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        Some((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)))
    }

    /// Set the tile of the given position and role.
    pub fn set_tile(&mut self, pos: (i32, i32), role: Layer2DRole, tile: Option<Uuid>) {
        if role == Layer2DRole::FX {
            return;
        }
        if let Some(t) = self.tiles.get_mut(&pos) {
            t.layers[role as usize] = tile;
        } else {
            let mut region_tile = RegionTile::default();
            region_tile.layers[role as usize] = tile;
            self.tiles.insert(pos, region_tile);
        }
    }

    /// Returns true if the character can move to the given position.
    pub fn can_move_to(
        &self,
        pos: Vec2<i32>,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        update: &RegionUpdate,
    ) -> bool {
        let mut can_move = true;

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

                            if let Some(wallfx) = update.wallfx.get(&(pos.x, pos.y)) {
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

    // pub fn distance(&self, x: Vec2i, y: Vec2i) -> f32 {
    //     distance(Vec2f::from(x), Vec2f::from(y))
    // }

    /// Fills a code level with the blocking tiles of the region.
    pub fn fill_code_level(
        &self,
        level: &mut Level,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        update: &RegionUpdate,
        region: &Region,
    ) {
        level.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut can_move = true;
                let pos = Vec2::new(x, y);

                if let Some(tile) = self.tiles.get(&(pos.x, pos.y)) {
                    for index in 0..tile.layers.len() {
                        if let Some(layer) = tile.layers[index] {
                            if let Some(t) = tiles.get(&layer) {
                                if t.blocking && index == Layer2DRole::Wall as usize {
                                    can_move = false;

                                    if let Some(wallfx) = update.wallfx.get(&(pos.x, pos.y)) {
                                        if wallfx.fx != WallFX::Normal {
                                            can_move = true;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // // If the tile contains a light, add it.
                    // if let Some(timeline) = &tile.tilefx {
                    //     if timeline.contains_collection("Light Emitter") {
                    //         let light = TileFX::new_fx("Light Emitter", None);
                    //         let mut l = light.collection_cloned();
                    //         timeline.fill(&level.time, &mut l);
                    //         level.add_light(pos, l);
                    //     }
                    // }
                }

                if !can_move {
                    level.set_blocking((x, y));
                }

                if let Some(fx) = region.effects.get(&Vec3::new(x, 0, y)) {
                    if let Some(props) = fx.get_light_collection() {
                        level.add_light(pos, props);
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
    FX,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionTile {
    // Tile layers
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
