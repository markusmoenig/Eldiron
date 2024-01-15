//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RegionUpdate {
    pub characters: FxHashMap<Uuid, CharacterUpdate>,
}

impl Default for RegionUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionUpdate {
    pub fn new() -> Self {
        Self {
            characters: FxHashMap::default(),
        }
    }

    /// Sets up the region instance.
    pub fn clear(&mut self) {
        self.characters.clear();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterUpdate {
    pub tile_id: Uuid,
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
            tile_id: Uuid::nil(),
            tile_name: "".to_string(),

            name: "".to_string(),
            position: vec2f(0.0, 0.0),
            moving: None,
            move_delta: 0.0,
        }
    }
}
