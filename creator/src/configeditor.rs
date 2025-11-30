use theframework::prelude::*;
pub struct ConfigEditor {
    pub target_fps: i32,
    pub game_tick_ms: i32,
    pub viewport: Vec2<i32>,
    pub grid_size: i32,
}

use crate::editor::CONFIG;

#[allow(clippy::new_without_default)]
impl ConfigEditor {
    pub fn new() -> Self {
        Self {
            target_fps: 30,
            game_tick_ms: 250,
            viewport: Vec2::new(800, 600),
            grid_size: 32,
        }
    }

    pub fn build(&self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("ConfigEdit"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
            }
        }
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        center.set_widget(textedit);

        center
    }

    pub fn get_i32_default(&self, table: &str, key: &str, default: i32) -> i32 {
        let tab = CONFIG.read().unwrap();
        if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_integer() {
                    return v as i32;
                }
            }
        }
        default
    }

    pub fn get_f32_default(&self, table: &str, key: &str, default: f32) -> f32 {
        let tab = CONFIG.read().unwrap();
        if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_float() {
                    return v as f32;
                }
            }
        }
        default
    }

    pub fn get_bool_default(&self, table: &str, key: &str, default: bool) -> bool {
        let tab = CONFIG.read().unwrap();
        if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_bool() {
                    return v;
                }
            }
        }
        default
    }

    pub fn get_string_default(&self, table: &str, key: &str, default: &str) -> String {
        let tab = CONFIG.read().unwrap();
        if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_str() {
                    return v.to_string();
                }
            }
        }
        default.to_string()
    }

    pub fn read_defaults(&mut self) {
        self.target_fps = self.get_i32_default("game", "target_fps", 30).clamp(1, 60);
        self.game_tick_ms = self.get_i32_default("game", "game_tick_ms", 250);
        self.viewport.x = self.get_i32_default("viewport", "width", 800);
        self.viewport.y = self.get_i32_default("viewport", "height", 600);
        self.grid_size = self.get_i32_default("viewport", "grid_size", 32);
    }
}
