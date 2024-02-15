use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RegionUpdate {
    pub id: Uuid,

    #[serde(with = "vectorize")]
    pub wallfx: FxHashMap<(i32, i32), WallFxUpdate>,
    pub characters: FxHashMap<Uuid, CharacterUpdate>,
    pub items: FxHashMap<Uuid, ItemUpdate>,

    pub server_tick: i64,
    pub brightness: f32,
}

impl Default for RegionUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionUpdate {
    pub fn new() -> Self {
        Self {
            id: Uuid::nil(),
            wallfx: FxHashMap::default(),
            characters: FxHashMap::default(),
            items: FxHashMap::default(),
            server_tick: 0,
            brightness: 1.0,
        }
    }

    /// Clear the update.
    pub fn clear(&mut self) {
        self.characters.clear();
    }

    /// Create an update from json.
    pub fn from_json(json: &str) -> Option<Self> {
        if let Ok(update) = serde_json::from_str(json) {
            Some(update)
        } else {
            None
        }
    }

    /// Convert the update to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

/// A character as described by the server for consumption by the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterUpdate {
    pub id: Uuid,
    pub tile_name: String,

    pub name: String,
    pub position: Vec2f,
    pub moving: Option<(Vec2f, Vec2f)>,
    pub move_delta: f32,
}

impl Default for CharacterUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterUpdate {
    pub fn new() -> Self {
        Self {
            id: Uuid::nil(),
            tile_name: "".to_string(),

            name: "".to_string(),
            position: vec2f(0.0, 0.0),
            moving: None,
            move_delta: 0.0,
        }
    }
}

/// An item as described by the server for consumption by the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemUpdate {
    pub tile_id: Uuid,
    pub tile_name: String,

    pub name: String,
    pub position: Vec2f,
}

impl Default for ItemUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemUpdate {
    pub fn new() -> Self {
        Self {
            tile_id: Uuid::nil(),
            tile_name: "".to_string(),

            name: "".to_string(),
            position: vec2f(0.0, 0.0),
        }
    }
}

/// Update structure for the current wall effects in the region.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WallFxUpdate {
    /// When this effect got inserted.
    pub at_tick: i64,

    pub fx: WallFX,
    pub prev_fx: WallFX,
}

impl Default for WallFxUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl WallFxUpdate {
    pub fn new() -> Self {
        Self {
            at_tick: 0,
            fx: WallFX::Normal,
            prev_fx: WallFX::Normal,
        }
    }
}
