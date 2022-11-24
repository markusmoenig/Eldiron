// --- Gear System

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Gear {
    pub slots               : FxHashMap<String, InventoryItem>,
}

impl Gear {
    pub fn new() -> Self {
        Self {
            slots           : FxHashMap::default(),
        }
    }

    /// Query the stats for a given attribute name
    pub fn stats(&mut self, _name: &str) -> i32 {
        0
    }

}

pub fn script_register_gear_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Gear>("Gear")
        .register_fn("stats", Gear::stats);
}
