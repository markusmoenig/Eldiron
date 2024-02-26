use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Screen {
    pub id: Uuid,
    pub name: String,

    pub aspect_ratio: ScreenAspectRatio,

    pub width: i32,
    pub height: i32,
    pub grid_size: i32,
    pub scroll_offset: Vec2i,
    pub zoom: f32,

    pub widgets: FxHashMap<Uuid, Widget>,
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Screen".to_string(),

            aspect_ratio: ScreenAspectRatio::_16_9,

            width: 1280,
            height: 720,
            grid_size: 16,
            scroll_offset: Vec2i::zero(),
            zoom: 1.0,

            widgets: FxHashMap::default(),
        }
    }

    /// Returns the widgets sorted by size (width * height), smallest first.
    pub fn sorted_widgets_by_size(&self) -> Vec<&Widget> {
        let mut widgets: Vec<&Widget> = self.widgets.values().collect();
        widgets.sort_by(|a, b| {
            let size_a = a.width * a.height;
            let size_b = b.width * b.height;
            size_b
                .partial_cmp(&size_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        widgets
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(Screen::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

/// The aspect ratio of the screen.
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum ScreenAspectRatio {
    _16_9,
    _4_3,
}

impl ScreenAspectRatio {
    pub fn to_string(self) -> &'static str {
        match self {
            Self::_16_9 => "16:9",
            Self::_4_3 => "4:3",
        }
    }
    pub fn ratio(self) -> f32 {
        match self {
            Self::_16_9 => 16.0 / 9.0,
            Self::_4_3 => 4.0 / 3.0,
        }
    }
    pub fn iterator() -> impl Iterator<Item = ScreenAspectRatio> {
        [Self::_16_9, Self::_4_3].iter().copied()
    }
    pub fn width(self, height: i32) -> i32 {
        (height as f32 * self.ratio()) as i32
    }
    pub fn height(self, width: i32) -> i32 {
        (width as f32 / self.ratio()) as i32
    }
    pub fn from_index(index: u8) -> Option<ScreenAspectRatio> {
        match index {
            0 => Some(Self::_16_9),
            1 => Some(Self::_4_3),
            _ => None,
        }
    }
}
