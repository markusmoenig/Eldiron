// --- Skills system

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Skill {
    pub value: i32,
    pub level: i32,
    pub property: String,
}

impl Skill {
    pub fn get_value(&mut self) -> i32 {
        self.value
    }
    pub fn get_level(&mut self) -> i32 {
        self.level
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Skills {
    pub skills: FxHashMap<String, Skill>,
}

impl Skills {
    pub fn new() -> Self {
        Self {
            skills: FxHashMap::default(),
        }
    }

    /// Returns the skill for the given name.
    pub fn add_skill(&mut self, name: String) {
        self.skills.insert(
            name,
            Skill {
                value: 0,
                level: 0,
                property: "".to_string(),
            },
        );
    }

    /// Returns the skill for the given name.
    pub fn item(&mut self, name: &str) -> Skill {
        if let Some(item) = self.skills.get(name) {
            return item.clone();
        }
        Skill {
            value: 0,
            level: 0,
            property: "".to_string(),
        }
    }
}

pub fn script_register_skills_api(engine: &mut rhai::Engine) {
    engine
        .register_type_with_name::<Skill>("Skill")
        .register_get("value", Skill::get_value)
        .register_get("level", Skill::get_level);

    engine
        .register_type_with_name::<Skills>("Skills")
        .register_fn("item", Skills::item);
}
