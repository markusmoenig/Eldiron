use crate::prelude::*;

use rhai::Engine;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// CharacterSheet
pub struct Sheet {
    pub abilities               : FxHashMap<String, i32>,
}

impl Sheet {
    pub fn new() -> Self {
        Self {
            abilities           : FxHashMap::default(),
        }
    }

    /// Get the ability of the given name
    pub fn get_ability(&mut self, name: &str) -> i32 {
        if let Some(v) = self.abilities.get(&name.to_string()) {
            return *v;
        }
        -1
    }

    /// Set the ability of the given name
    pub fn set_ability(&mut self, name: &str, value: i32) {
        self.abilities.insert(name.to_string(), value);
    }

    /// Register sheet related fns and getter / setter
    pub fn register(engine: &mut Engine) {
        engine.register_type_with_name::<Sheet>("Sheet");

        engine.register_fn("get_ability", Sheet::get_ability);
        engine.register_fn("set_ability", Sheet::set_ability);
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// CharacterSheet
pub struct Ability {
    pub value                   : i32,
    pub roll                    : String,
}

impl Ability {
    pub fn new() -> Self {
        Self {
            value               : -1,
            roll                : "".to_string(),
        }
    }
}
