//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Widget {
    pub id: Uuid,
    pub name: String,

    #[serde(with = "vectorize")]
    #[serde(default)]
    pub ui_tiles: FxHashMap<(i32, i32), Vec<Uuid>>,

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

            ui_tiles: FxHashMap::default(),
            bundle: TheCodeBundle::default(),
        }
    }

    /// Checks if the given position is inside the widget.
    pub fn is_inside(&self, pos: &Vec2i) -> bool {
        pos.x >= self.x as i32
            && pos.x <= (self.x + self.width) as i32
            && pos.y >= self.y as i32
            && pos.y <= (self.y + self.height) as i32
    }

    /// Add a tile to the widget.
    pub fn add_tile(&mut self, screen_pos: (i32, i32), tile: Uuid) {
        // Convert screen coordinates to widget coordinates.
        let pos = (screen_pos.0 - self.x as i32, screen_pos.1 - self.y as i32);
        if let Some(tiles) = self.ui_tiles.get_mut(&pos) {
            tiles.push(tile);
        } else {
            self.ui_tiles.insert(pos, vec![tile]);
        }
    }

    /// Erase a tile from the widget.
    pub fn erase_tile(&mut self, pos: (i32, i32)) {
        self.ui_tiles.remove(&pos);
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
