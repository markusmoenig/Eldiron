use num_traits::zero;
use rusterix::Map;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RegionContentRole {
    Character,
    Item,
}

/// Region content item. Represents a character or item instance in the region.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionContent {
    pub id: Uuid,
    pub role: RegionContentRole,

    /// The source code for individual set up of the content.
    pub source: String,

    /// The id of the character or item.
    pub template_id: Uuid,
}

impl Default for RegionContent {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionContent {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            role: RegionContentRole::Character,

            source: String::new(),

            template_id: Uuid::new_v4(),
        }
    }
}

/// The data for a character instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Character {
    pub id: Uuid,

    pub name: String,

    /// The character map model.
    pub map: Map,

    /// The instance initialization code.
    pub source: String,

    /// The initial position.
    pub position: Vec3<f32>,

    /// The id of the character bundle.
    pub character_id: Uuid,
    /// The custom bundle to override the default behavior.
    pub instance: TheCodeBundle,
}

impl Default for Character {
    fn default() -> Self {
        Self::new()
    }
}

impl Character {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "NewCharacter".to_string(),

            map: Map::default(),
            source: String::new(),
            position: zero(),

            character_id: Uuid::new_v4(),
            instance: TheCodeBundle::new(),
        }
    }
}
