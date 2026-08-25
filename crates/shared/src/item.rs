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
    pub module: serde_json::Value,

    /// The instance initialization or template code.
    pub source: String,

    /// The instance initialization or template debug code.
    #[serde(default)]
    pub source_debug: String,

    /// The attributes toml data.
    #[serde(default)]
    pub data: String,

    /// Authoring metadata used for look/description style presentation.
    #[serde(default)]
    pub authoring: String,

    /// Project-owned editable icon animation frames for the default/on state.
    /// An empty list inherits the icon resolved from a mapped tile, the active
    /// ruleset, or the item's ordinary visual fallback.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_frames: Vec<rusterix::Texture>,

    /// Archive paths for project-owned On frames. The archive loader hydrates
    /// these PNGs back into `icon_frames`; legacy JSON keeps using inline data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_frame_paths: Vec<String>,

    /// Project-owned editable icon animation frames for the optional off state.
    /// An empty list inherits an available off-state tile or ruleset artwork;
    /// otherwise the item has no off-state icon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_off_frames: Vec<rusterix::Texture>,

    /// Archive paths for project-owned Off frames. The archive loader hydrates
    /// these PNGs back into `icon_off_frames`; legacy JSON keeps using inline data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_off_frame_paths: Vec<String>,

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

            module: serde_json::Value::Null,
            map: Map::default(),
            source: String::new(),
            source_debug: String::new(),
            data: String::new(),
            authoring: String::new(),
            icon_frames: Vec::new(),
            icon_frame_paths: Vec::new(),
            icon_off_frames: Vec::new(),
            icon_off_frame_paths: Vec::new(),
            position: zero(),

            item_id: Uuid::new_v4(),
        }
    }

    pub fn icon_frames_for_state(&self, on: bool) -> &Vec<rusterix::Texture> {
        if on {
            &self.icon_frames
        } else {
            &self.icon_off_frames
        }
    }

    pub fn icon_frames_for_state_mut(&mut self, on: bool) -> &mut Vec<rusterix::Texture> {
        if on {
            &mut self.icon_frames
        } else {
            &mut self.icon_off_frames
        }
    }
}
