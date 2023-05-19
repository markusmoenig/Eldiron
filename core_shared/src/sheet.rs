use crate::prelude::*;

use rhai::Engine;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// CharacterSheet
pub struct Sheet {

    pub name                    : String,
    pub class_name              : String,
    pub race_name               : String,

    pub abilities               : FxHashMap<String, i32>,

    pub hit_points              : i32,
    pub max_hit_points          : i32,

    pub inventory               : Inventory,
    pub weapons                 : Weapons,
    pub gear                    : Gear,

    pub spells                  : Spells,

    pub skills                  : Skills,
    pub experience              : Experience,

    pub wealth                  : Currency,
}

impl Sheet {
    pub fn new() -> Self {
        Self {
            name                : String::new(),
            class_name          : String::new(),
            race_name           : String::new(),

            abilities           : FxHashMap::default(),

            hit_points          : 0,
            max_hit_points      : 0,

            inventory           : Inventory::new(),
            weapons             : Weapons::new(),
            gear                : Gear::new(),

            spells              : Spells::new(),

            skills              : Skills::new(),
            experience          : Experience::new(),

            wealth              : Currency::empty(),
        }
    }

    /// Get the name
    pub fn get_name(&mut self) -> String {
        self.name.clone()
    }

    /// Get the class name
    pub fn get_class_name(&mut self) -> String {
        self.class_name.clone()
    }

    /// Get the race name
    pub fn get_race_name(&mut self) -> String {
        self.race_name.clone()
    }

    /// Get the inventory
    pub fn get_inventory(&mut self) -> Inventory {
        self.inventory.clone()
    }

    /// Get the weapons
    pub fn get_weapons(&mut self) -> Weapons {
        self.weapons.clone()
    }

    /// Get the gear
    pub fn get_gear(&mut self) -> Gear {
        self.gear.clone()
    }

    /// Get the spells
    pub fn get_spells(&mut self) -> Spells {
        self.spells.clone()
    }

    /// Get the wealth
    pub fn get_wealth(&mut self) -> Currency {
        self.wealth.clone()
    }

    /// Set the wealth
    pub fn set_wealth(&mut self, wealth: Currency) {
        self.wealth = wealth
    }

    /// Get the hit_points
    pub fn get_hit_points(&mut self) -> i32 {
        self.hit_points
    }

    /// Set the hit points
    pub fn set_hit_points(&mut self, value: i32) {
        self.hit_points = value.clamp(0, i32::MAX);
    }

    /// Get the maximum amount of hit oints
    pub fn get_max_hit_points(&mut self) -> i32 {
        self.max_hit_points
    }

    /// Set the maximum amount of hit points
    pub fn set_max_hit_points(&mut self, value: i32) {
        self.max_hit_points = value
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

    /// Get the skills
    pub fn get_skills(&mut self) -> Skills {
        self.skills.clone()
    }

    /// Get the experience
    pub fn get_experience(&mut self) -> Experience {
        self.experience.clone()
    }

    /// Can the character afford this ?
    pub fn can_afford(&mut self, value: Currency) -> bool {
        self.wealth >= value
    }

    /// Register sheet related fns and getter / setter
    pub fn register(engine: &mut Engine) {
        engine.register_type_with_name::<Sheet>("Sheet");

        engine.register_get("name", Sheet::get_name);
        engine.register_get("class", Sheet::get_class_name);
        engine.register_get("name", Sheet::get_race_name);

        engine.register_get("inventory", Sheet::get_inventory);
        engine.register_get("weapons", Sheet::get_weapons);
        engine.register_get("gear", Sheet::get_gear);
        engine.register_get("spells", Sheet::get_spells);
        engine.register_get("skills", Sheet::get_skills);
        engine.register_get("experience", Sheet::get_experience);

        engine.register_get_set("hit_points", Sheet::get_hit_points, Sheet::set_hit_points);
        engine.register_get_set("max_hit_points", Sheet::get_max_hit_points, Sheet::set_max_hit_points);

        engine.register_get_set("wealth", Sheet::get_wealth, Sheet::set_wealth);

        engine.register_fn("get_ability", Sheet::get_ability);
        engine.register_fn("set_ability", Sheet::set_ability);

        engine.register_fn("can_afford", Sheet::can_afford);
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
