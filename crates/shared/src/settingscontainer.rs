use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Debug)]
pub enum SettingsType {
    Project,
    Render,
    Region(Uuid),
    Game,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettingsContainer {
    pub settings: IndexMap<SettingsType, TheNodeUI>,
}

impl Default for SettingsContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsContainer {
    pub fn new() -> Self {
        let mut settings: IndexMap<SettingsType, TheNodeUI> = IndexMap::default();

        //--- Project

        let mut project = TheNodeUI::default();
        let item = TheNodeUIItem::IntEditSlider(
            "projectTickMs".into(),
            "Ms per Tick".into(),
            "Set tick / animation frequency in milliseconds.".into(),
            250,
            1..=1000,
            false,
        );
        project.add_item(item);
        settings.insert(SettingsType::Project, project);

        //--- Render

        let mut render = TheNodeUI::default();
        let item = TheNodeUIItem::IntEditSlider(
            "renderFPS".into(),
            "Target FPS".into(),
            "Set the target FPS.".into(),
            30,
            1..=60,
            false,
        );
        render.add_item(item);
        let item = TheNodeUIItem::Selector(
            "renderSampleMode".into(),
            "Sample Mode".into(),
            "Set the sampling mode for 3D.".into(),
            vec!["Nearest".to_string(), "Linear".to_string()],
            0,
        );
        render.add_item(item);

        settings.insert(SettingsType::Render, render);

        //--- Game

        let mut game = TheNodeUI::default();
        let item = TheNodeUIItem::IntEditSlider(
            "gameScreenWidth".into(),
            "Screen Width".into(),
            "Set the global game screeen width.".into(),
            1280,
            400..=4000,
            false,
        );
        game.add_item(item);
        let item = TheNodeUIItem::IntEditSlider(
            "gameScreenHeight".into(),
            "Screen Height".into(),
            "Set the global game screen height.".into(),
            720,
            400..=4000,
            false,
        );
        game.add_item(item);

        settings.insert(SettingsType::Game, game);

        Self { settings }
    }

    pub fn apply_to_text_layout(
        &self,
        settings: SettingsType,
        layout: &mut dyn TheTextLayoutTrait,
    ) {
        for (set, ui) in self.settings.iter() {
            if *set == settings {
                layout.clear();
                ui.apply_to_text_layout(layout);
                break;
            }
        }
    }

    pub fn handle_event(&mut self, event: TheEvent) -> bool {
        for ui in self.settings.values_mut() {
            if ui.handle_event(&event) {
                return true;
            }
        }
        false
    }

    /// Get an i32 value with a default fallback.
    pub fn get_i32_value(&self, id: &str, default: i32) -> i32 {
        for ui in self.settings.values() {
            if let Some(value) = ui.get_i32_value(id) {
                return value;
            }
        }
        default
    }
}
