// --- Experience system

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Experience {
    pub experience              : i32,
    pub level                   : i32,

    pub system_name             : Option<String>,
    pub tree_name               : Option<String>,

    pub experience_msg          : String,

    pub level_behavior_id       : Uuid,

    // Level experience needed, message and tree id
    pub levels              : Vec<(i32, String, Uuid)>
}

impl Experience {
    pub fn new() -> Self {
        Self {
            experience          : 0,
            level               : 1,

            system_name         : None,
            tree_name           : None,

            level_behavior_id   : Uuid::new_v4(),

            experience_msg      : "You gain {} experience.".to_string(),

            levels              : vec![]
        }
    }

    pub fn get_experience(&mut self) -> i32 {
        self.experience
    }
    pub fn get_level(&mut self) -> i32 {
        self.level
    }
}

pub fn script_register_experience_api(engine: &mut rhai::Engine) {

    engine.register_type_with_name::<Experience>("Experience")
        .register_get("experience", Experience::get_experience)
        .register_get("level", Experience::get_level);
}