use crate::prelude::*;
use theframework::prelude::*;

fn default_min_brightness() -> f32 {
    0.3
}

fn default_max_brightness() -> f32 {
    1.0
}

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

    #[serde(default = "default_min_brightness")]
    pub min_brightness: f32,

    #[serde(default = "default_max_brightness")]
    pub max_brightness: f32,
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

            min_brightness: 0.3,
            max_brightness: 1.0,
        }
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

    /// Set the timeline.
    pub fn set_tilefx(&mut self, pos: (i32, i32), timeline: TheTimeline) {
        if let Some(tile) = self.tiles.get_mut(&pos) {
            tile.tilefx = Some(timeline);
        }
    }

    /// Returns true if the character can move to the given position.
    pub fn can_move_to(
        &self,
        pos: Vec2i,
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

    pub fn distance(&self, x: Vec2i, y: Vec2i) -> f32 {
        distance(Vec2f::from(x), Vec2f::from(y))
    }

    /// Fills a code level with the blocking tiles of the region.
    pub fn fill_code_level(
        &self,
        level: &mut Level,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        update: &RegionUpdate,
    ) {
        level.clear_blocking();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut can_move = true;
                let pos = vec2i(x, y);
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

                    // If the tile contains a light, add it.
                    if let Some(timeline) = &tile.tilefx {
                        if timeline.contains_collection("Light Emitter") {
                            let light = TileFX::new_fx("Light Emitter", None);
                            let mut l = light.collection_cloned();
                            timeline.fill(&level.time, &mut l);
                            level.add_light(pos, l);
                        }
                    }
                }

                if !can_move {
                    level.set_blocking((x, y));
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
    // WallFX (name, position, alpha, delta)
    pub wallfx: Option<(String, Vec2f, f32, f32)>,

    // TileFX Timeline
    pub tilefx: Option<TheTimeline>,

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
            wallfx: None,
            tilefx: None,
            layers: vec![None, None, None],
        }
    }
}
