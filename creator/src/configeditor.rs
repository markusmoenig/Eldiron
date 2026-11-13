use theframework::prelude::*;
pub struct ConfigEditor {
    table: toml::Table,
    pub target_fps: i32,
    pub game_tick_ms: i32,
    pub viewport: Vec2<i32>,
    pub grid_size: i32,
}

#[allow(clippy::new_without_default)]
impl ConfigEditor {
    pub fn new() -> Self {
        Self {
            table: toml::Table::new(),
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
        let tab = &self.table;
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
        let tab = &self.table;
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
        let tab = &self.table;
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
        let tab = &self.table;
        if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_str() {
                    return v.to_string();
                }
            }
        }
        default.to_string()
    }

    /// Keep the last valid settings while the authored text is being edited.
    /// The parsed table and derived defaults belong to this application only.
    pub fn read_config(&mut self, source: &str) {
        if let Ok(table) = source.parse::<toml::Table>() {
            self.table = table;
        }
        self.read_defaults();
    }

    pub fn read_defaults(&mut self) {
        self.target_fps = self.get_i32_default("game", "target_fps", 30).clamp(1, 60);
        self.game_tick_ms = self.get_i32_default("game", "game_tick_ms", 250);
        self.viewport.x = self.get_i32_default("viewport", "width", 800);
        self.viewport.y = self.get_i32_default("viewport", "height", 600);
        self.grid_size = self.get_i32_default("viewport", "grid_size", 32);
    }
}

#[cfg(test)]
mod tests {
    use crate::creator_context::CreatorContext;

    #[test]
    fn application_configuration_and_last_valid_edits_do_not_cross_contexts() {
        let mut first = CreatorContext::new();
        let mut second = CreatorContext::new();
        first.sidebar_mode = crate::sidebar::SidebarMode::Palette;
        assert_eq!(second.sidebar_mode, crate::sidebar::SidebarMode::Region);
        first.config.read_config(
            "[game]\ntarget_fps=45\ngame_tick_ms=123\n[viewport]\nwidth=321\nheight=234\ngrid_size=11\n[render]\nsample_mode='linear'",
        );
        second
            .config
            .read_config("[game]\ntarget_fps=20\ngame_tick_ms=333");
        first.config.read_config("[game]\ntarget_fps=");
        assert_eq!(
            (first.config.target_fps, first.config.game_tick_ms),
            (45, 123)
        );
        assert_eq!(first.config.viewport.into_array(), [321, 234]);
        assert_eq!(first.config.grid_size, 11);
        assert_eq!(
            first
                .config
                .get_string_default("render", "sample_mode", "nearest"),
            "linear"
        );
        assert_eq!(
            (second.config.target_fps, second.config.game_tick_ms),
            (20, 333)
        );
        assert_eq!(
            second
                .config
                .get_string_default("render", "sample_mode", "nearest"),
            "nearest"
        );
        // A valid replacement removes settings absent from the new document.
        first.config.read_config("");
        assert_eq!(
            (first.config.target_fps, first.config.game_tick_ms),
            (30, 250)
        );
        assert_eq!(
            (second.config.target_fps, second.config.game_tick_ms),
            (20, 333)
        );
        assert!(!first.reference.initialized() && !second.reference.initialized());
        assert!(!first.jobs.scene_initialized() && !second.jobs.scene_initialized());
    }
}
