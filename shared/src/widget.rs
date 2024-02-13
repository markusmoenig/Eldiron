//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Widget {
    pub id: Uuid,
    pub name: String,

    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,

    pub bundle: TheCodeBundle,
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Widget".to_string(),

            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,

            bundle: TheCodeBundle::default(),
        }
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(Widget::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
