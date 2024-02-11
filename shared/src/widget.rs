//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Widget {
    pub id: Uuid,
    pub name: String,

    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

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

            x: 0,
            y: 0,
            width: 20,
            height: 20,

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
