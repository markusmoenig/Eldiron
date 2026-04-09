use crate::prelude::*;
use codegridfx::Module;
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

    #[serde(default = "default_region_module")]
    pub module: Module,

    #[serde(default)]
    pub source: String,

    #[serde(default)]
    pub source_debug: String,

    pub characters: IndexMap<Uuid, Character>,
    pub items: IndexMap<Uuid, Item>,

    pub editing_position_3d: Vec3<f32>,
    #[serde(default = "default_editing_look_at_3d")]
    pub editing_look_at_3d: Vec3<f32>,

    /// Persisted per-view 3D edit camera anchors.
    #[serde(default)]
    pub editing_position_iso_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_look_at_iso_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_position_orbit_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_look_at_orbit_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_position_firstp_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_look_at_firstp_3d: Option<Vec3<f32>>,
    #[serde(default)]
    pub editing_iso_scale: Option<f32>,
    #[serde(default)]
    pub editing_orbit_distance: Option<f32>,
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
            module: default_region_module(),
            source: String::new(),
            source_debug: String::new(),

            characters: IndexMap::default(),
            items: IndexMap::default(),

            editing_position_3d: Vec3::zero(),
            editing_look_at_3d: Vec3::zero(),
            editing_position_iso_3d: None,
            editing_look_at_iso_3d: None,
            editing_position_orbit_3d: None,
            editing_look_at_orbit_3d: None,
            editing_position_firstp_3d: None,
            editing_look_at_firstp_3d: None,
            editing_iso_scale: None,
            editing_orbit_distance: None,
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

fn default_region_module() -> Module {
    Module::as_type(codegridfx::ModuleType::Region)
}
