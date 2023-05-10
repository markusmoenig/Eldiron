use crate::prelude::*;

use rhai::Engine;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// CharacterSheet
pub struct Sheet {
    pub abilities               : FxHashMap<String, i32>,

    pub gold                    : i32,
    pub silver                  : i32,

    pub hit_points              : i32,
    pub max_hit_points          : i32,

    pub inventory               : Inventory,
}

impl Sheet {
    pub fn new() -> Self {
        Self {
            abilities           : FxHashMap::default(),

            gold                : 0,
            silver              : 0,

            hit_points          : 0,
            max_hit_points      : 0,

            inventory           : Inventory::new(),
        }
    }

    /// Get the inventory
    pub fn get_inventory(&mut self) -> Inventory {
        self.inventory.clone()
    }

    /// Get the hit_points
    pub fn get_hit_points(&mut self) -> i32 {
        self.hit_points
    }

    /// Set the hit points
    pub fn set_hit_points(&mut self, value: i32) {
        self.hit_points = value
    }

    /// Get the maximum amount of hit oints
    pub fn get_max_hit_points(&mut self) -> i32 {
        self.max_hit_points
    }

    /// Set the maximum amount of hit points
    pub fn set_max_hit_points(&mut self, value: i32) {
        self.max_hit_points = value
    }

    /// Get the amount of gold
    pub fn get_gold(&mut self) -> i32 {
        self.gold
    }

    /// Set the amount of gold
    pub fn set_gold(&mut self, value: i32) {
        self.gold = value
    }

    /// Get the amount of silver
    pub fn get_silver(&mut self) -> i32 {
        self.silver
    }

    /// Set the amount of silver (overflows go to gold)
    pub fn set_silver(&mut self, value: i32) {
        self.gold += value / 100;
        self.silver = value % 100;
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

        engine.register_get("inventory", Sheet::get_inventory);

        engine.register_get_set("hit_points", Sheet::get_hit_points, Sheet::set_hit_points);
        engine.register_get_set("max_hit_points", Sheet::get_max_hit_points, Sheet::set_max_hit_points);

        engine.register_get_set("gold", Sheet::get_gold, Sheet::set_gold);
        engine.register_get_set("silver", Sheet::get_silver, Sheet::set_silver);

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
