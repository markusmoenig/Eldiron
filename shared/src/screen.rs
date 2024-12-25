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
    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,

    /// The tiles which get drawn in the background, i.e. before widgets are drawn.
    #[serde(with = "vectorize")]
    #[serde(default)]
    pub tiles: FxHashMap<(i32, i32), Vec<Uuid>>,

    /// The tiles which get drawn in the foreground, i.e. after widgets are drawn.
    #[serde(with = "vectorize")]
    #[serde(default)]
    pub foreground_tiles: FxHashMap<(i32, i32), Vec<Uuid>>,

    #[serde(default)]
    pub widget_list: Vec<Widget>,

    #[serde(default)]
    pub bundle: TheCodeBundle,
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
            scroll_offset: Vec2::zero(),
            zoom: 1.0,

            tiles: FxHashMap::default(),
            foreground_tiles: FxHashMap::default(),

            widget_list: vec![],

            bundle: TheCodeBundle::default(),
        }
    }

    /// Get the given widget.
    pub fn get_widget(&self, id: &Uuid) -> Option<&Widget> {
        self.widget_list.iter().find(|w| w.id == *id)
    }

    /// Get the given widget mutable.
    pub fn get_widget_mut(&mut self, id: &Uuid) -> Option<&mut Widget> {
        self.widget_list.iter_mut().find(|w| w.id == *id)
    }

    /// Remove the given widget.
    pub fn remove_widget(&mut self, id: &Uuid) {
        self.widget_list.retain(|w| w.id != *id);
    }

    /// Returns the widgets sorted by size (width * height), smallest first.
    pub fn sorted_widgets_by_size(&self) -> Vec<Widget> {
        let mut widgets = self.widget_list.clone();
        widgets.sort_by(|a, b| {
            let size_a = a.width * a.height;
            let size_b = b.width * b.height;
            size_b
                .partial_cmp(&size_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        widgets
    }

    /// Add a background tile to the screen.
    pub fn add_background_tile(&mut self, pos: (i32, i32), tile: Uuid) {
        if let Some(tiles) = self.tiles.get_mut(&pos) {
            tiles.push(tile);
        } else {
            self.tiles.insert(pos, vec![tile]);
        }
    }

    /// Add a foreground tile to the screen.
    pub fn add_foreground_tile(&mut self, pos: (i32, i32), tile: Uuid) {
        if let Some(tiles) = self.foreground_tiles.get_mut(&pos) {
            tiles.push(tile);
        } else {
            self.foreground_tiles.insert(pos, vec![tile]);
        }
    }

    /// Erase a background tile from the widget.
    pub fn erase_background_tile(&mut self, pos: (i32, i32)) {
        self.tiles.remove(&pos);
    }

    /// Erase a foreground tile from the widget.
    pub fn erase_foreground_tile(&mut self, pos: (i32, i32)) {
        self.foreground_tiles.remove(&pos);
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
