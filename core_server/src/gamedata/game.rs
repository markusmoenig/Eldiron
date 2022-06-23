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

    pub fn load_from_embedded(file_name: &str) -> Self {

        let behavior = GameBehavior::load_from_embedded(file_name);

        Self {
            path                : PathBuf::new(),
            behavior,
        }
    }

    pub fn new() -> Self {

        Self {
            path                : std::path::Path::new("").to_path_buf(),
            behavior            : GameBehavior::new()
        }
    }

    /// Save the game behavior to file
    pub fn save_data(&self) {
        self.behavior.save_data();
    }

    pub fn startup(&mut self) {

    }
}