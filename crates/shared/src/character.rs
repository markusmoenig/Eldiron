use codegridfx::Module;
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
    #[serde(default)]
    pub module: Module,

    /// The instance initialization or template code.
    pub source: String,

    /// The instance initialization or template debug code.
    #[serde(default)]
    pub source_debug: String,

    /// The attributes toml data.
    #[serde(default)]
    pub data: String,

    /// Editor-only rigging preview TOML data.
    #[serde(default)]
    pub preview_rigging: String,

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

            module: Module::as_type(codegridfx::ModuleType::CharacterTemplate),

            map: Map::default(),
            source: String::new(),
            source_debug: String::new(),
            data: String::new(),
            preview_rigging: String::new(),
            position: zero(),

            character_id: Uuid::new_v4(),
        }
    }
}
