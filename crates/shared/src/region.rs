use crate::prelude::*;
pub use rusterix::map::*;
use theframework::prelude::*;

fn default_editing_look_at_3d() -> Vec3<f32> {
    Vec3::new(2.0, 0.0, 0.0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Region {
    pub id: Uuid,

    pub name: String,
    pub map: Map,

    #[serde(default)]
    pub config: String,

    pub characters: IndexMap<Uuid, Character>,
    pub items: IndexMap<Uuid, Item>,

    pub editing_position_3d: Vec3<f32>,
    #[serde(default = "default_editing_look_at_3d")]
    pub editing_look_at_3d: Vec3<f32>,
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
            name: "New Region".to_string(),

            map: Map::default(),
            config: String::new(),

            characters: IndexMap::default(),
            items: IndexMap::default(),

            editing_position_3d: Vec3::zero(),
            editing_look_at_3d: Vec3::zero(),
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
