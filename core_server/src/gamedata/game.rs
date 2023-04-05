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
    if sink.contains("screen_size") == false {
        sink.properties.insert(0,Property::new_intx("screen_size".to_string(), vec![1024, 608]));
    }
    if sink.contains("square_tile_size") == false {
        sink.properties.insert(1,Property::new_int("square_tile_size".to_string(), 32));
    }
    if sink.contains("primary_currency") == false {
        sink.push(Property::new_string("primary_currency".to_string(), "gold".to_string()));
    }
    if sink.contains("secondary_currency") == false {
        sink.push(Property::new_string("secondary_currency".to_string(), "silver".to_string()));
    }
    if sink.contains("secondary_to_primary") == false {
        sink.push(Property::new_int("secondary_to_primary".to_string(), 100));
    }
    if sink.contains("hitpoints") == false {
        sink.push(Property::new_string("hitpoints".to_string(), "HP".to_string()));
    }
    if sink.contains("max_hitpoints") == false {
        sink.push(Property::new_string("max_hitpoints".to_string(), "MAX_HP".to_string()));
    }
    if sink.contains("gear_slots") == false {
        sink.push(Property::new_string("gear_slots".to_string(), "helm, chest, legs, boots".to_string()));
    }
    if sink.contains("weapon_slots") == false {
        sink.push(Property::new_string("weapon_slots".to_string(), "main hand, off hand".to_string()));
    }
    if sink.contains("ticks_per_minute") == false {
        sink.push(Property::new_int("ticks_per_minute".to_string(), 4));
    }
}

pub fn generate_game_sink_descriptions() -> FxHashMap<String, Vec<String>> {

    let mut map : FxHashMap<String, Vec<String>> = FxHashMap::default();

    map.insert("screen_size".to_string(), vec!["The size of the game screen.".to_string()]);
    map.insert("square_tile_size".to_string(), vec!["The default drawing size of square tiles.".to_string()]);
    map.insert("primary_currency".to_string(), vec!["The variable name of your primary currency. \"gold\" by default.".to_string()]);
    map.insert("secondary_currency".to_string(), vec!["The variable name of your (optional) secondary currency. \"silver\" by default.".to_string()]);
    map.insert("secondary_to_primary".to_string(), vec!["How much secondary currency makes up one primary. 100 by default.".to_string()]);
    map.insert("hitpoints".to_string(), vec!["The name of the characters hitpoints variable.".to_string()]);
    map.insert("max_hitpoints".to_string(), vec!["The name of the characters maximum hitpoints variable.".to_string()]);
    map.insert("gear_slots".to_string(), vec!["The supported gear slots.".to_string()]);
    map.insert("weapon_slots".to_string(), vec!["The supported weapon slots.".to_string()]);
    map.insert("ticks_per_minute".to_string(), vec!["The number of game ticks per in-game minute. 4 by default.".to_string()]);

    map
}