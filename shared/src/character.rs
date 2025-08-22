use codegridfxlib::Module;
use num_traits::zero;
use rusterix::Map;
use theframework::prelude::*;

/// The data for a character instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Character {
    pub id: Uuid,
    pub name: String,

    /// The character map model.
    pub map: Map,

    /// The module source
    pub module: Module,

    /// The instance initialization code.
    pub source: String,

    /// The attributes toml data.
    #[serde(default)]
    pub data: String,

    /// The initial position.
    pub position: Vec3<f32>,

    /// The id of the character template.
    pub character_id: Uuid,
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

            module: Module::default(),

            map: Map::default(),
            source: String::new(),
            data: String::new(),
            position: zero(),

            character_id: Uuid::new_v4(),
        }
    }
}
