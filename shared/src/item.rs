use codegridfx::Module;
use num_traits::zero;
use rusterix::Map;
use theframework::prelude::*;

/// An item instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item {
    pub id: Uuid,
    pub name: String,

    /// The item map model.
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

    /// The initial position.
    pub position: Vec3<f32>,

    /// The id of the character template.
    pub item_id: Uuid,
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl Item {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "NewItem".to_string(),

            module: Module::as_type(codegridfx::ModuleType::ItemTemplate),
            map: Map::default(),
            source: String::new(),
            source_debug: String::new(),
            data: String::new(),
            position: zero(),

            item_id: Uuid::new_v4(),
        }
    }
}
