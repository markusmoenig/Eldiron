use rusterix::Map;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Screen {
    pub id: Uuid,
    pub name: String,

    /// Screen-level presentation settings. Empty settings retain the legacy
    /// fixed-layout behavior for backwards compatibility.
    #[serde(default)]
    pub settings: String,

    pub map: Map,
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

            settings:
                "[layout]\n# Available modes: \"fixed\" or \"responsive\"\nmode = \"fixed\"\n"
                    .to_string(),

            map: Map::default(),
        }
    }

    /// Whether this screen follows the runtime surface and anchors its widgets
    /// relative to that surface.
    pub fn is_responsive(&self) -> bool {
        let Ok(table) = self.settings.parse::<toml::Table>() else {
            return false;
        };
        table
            .get("layout")
            .and_then(toml::Value::as_table)
            .and_then(|layout| layout.get("mode"))
            .and_then(toml::Value::as_str)
            .is_some_and(|mode| mode.trim().eq_ignore_ascii_case("responsive"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_invalid_screen_settings_are_fixed() {
        let mut screen = Screen::new();
        screen.settings.clear();
        assert!(!screen.is_responsive());
        screen.settings = "not valid toml =".into();
        assert!(!screen.is_responsive());
    }

    #[test]
    fn responsive_mode_is_case_insensitive() {
        let mut screen = Screen::new();
        screen.settings = "[layout]\nmode = \"Responsive\"\n".into();
        assert!(screen.is_responsive());
    }

    #[test]
    fn legacy_serialized_screen_without_settings_is_fixed() {
        let screen = Screen::new();
        let mut value = serde_json::to_value(screen).unwrap();
        value.as_object_mut().unwrap().remove("settings");
        let decoded: Screen = serde_json::from_value(value).unwrap();
        assert!(decoded.settings.is_empty());
        assert!(!decoded.is_responsive());
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
