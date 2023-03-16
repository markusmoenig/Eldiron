// --- Weapons System

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Weapons {
    pub slots               : FxHashMap<String, Item>,
}

impl Weapons {
    pub fn new() -> Self {
        Self {
            slots           : FxHashMap::default(),
        }
    }

    /// Query the stats for a given attribute name
    pub fn stats(&mut self, _name: &str) -> i32 {
        0
    }

    /// Returns the item name for the given slot.
    pub fn item_name_for(&mut self, name: &str) -> String {
        if let Some(item) = self.slots.get(name) {
            return item.name.clone();
        }
        "".to_string()
    }
}

pub fn script_register_weapons_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Weapons>("Weapons")
        .register_fn("stats", Weapons::stats)
        .register_fn("item_name_for", Weapons::item_name_for);
}
