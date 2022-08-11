use core_shared::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::behavior::{ GameBehavior, GameBehaviorData };

#[derive(Serialize, Deserialize)]
pub struct GameData {
    pub behavior_data           : GameBehaviorData,
}

pub struct Game {
    pub path                    : PathBuf,
    pub behavior                : GameBehavior,
}

impl Game {
    pub fn load_from_path(path: &PathBuf) -> Self {

        let json_path = path.join("game").join( format!("{}{}", "game", ".json"));
        let behavior = GameBehavior::load_from_path(&json_path, &path.join("game"));

        Self {
            path                : path.clone(),
            behavior,
        }
    }

    #[cfg(feature = "embed_binaries")]
    pub fn load_from_embedded(file_name: &str) -> Self {

        let behavior = GameBehavior::load_from_embedded(file_name);

        Self {
            path                : PathBuf::new(),
            behavior,
        }
    }

    pub fn new() -> Self {

        let mut behavior = GameBehavior::new();
        let mut settings = PropertySink::new();
        update_game_sink(&mut settings);
        behavior.data.settings = Some(settings);

        Self {
            path                : std::path::Path::new("").to_path_buf(),
            behavior,
        }
    }

    /// Save the game behavior to file
    pub fn save_data(&self) {
        self.behavior.save_data();
    }

    pub fn startup(&mut self) {

    }
}

// Generate region sink

pub fn update_game_sink(sink: &mut PropertySink) {

    //
    if sink.contains("character_attributes") == false {
        sink.properties.insert(0,Property::new_color("character_attributes".to_string(), "\"HP, STR\"".to_string()));
    }
    /*
    if sink.contains("lighting") == false {
        sink.push(Property::new_string("lighting".to_string(), "off".to_string()));
    }*/
}

pub fn generate_game_sink_descriptions() -> HashMap<String, Vec<String>> {

    let mut map : HashMap<String, Vec<String>> = HashMap::new();

    map.insert("character_attributes".to_string(), vec!["The attributes of characters. These will be added as variables to each character instance".to_string()]);
    // map.insert("lighting".to_string(), vec!["The lighting mode. Use \"off\" for no lighting.".to_string()]);

    map
}