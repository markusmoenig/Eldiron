// --- Items / Inventory System

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Item {
    pub id                  : Uuid,
    pub name                : String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Items {
    pub items               : Vec<Item>,
    pub available           : Vec<Item>
}

impl Items {
    pub fn new() -> Self {
        Self {
            items        : vec![],
            available    : vec![],
        }
    }
}

pub fn script_register_item_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<ScriptMessageCmd>("Items")
        .register_fn("status", ScriptMessageCmd::status)
        .register_fn("debug", ScriptMessageCmd::debug)
        .register_fn("error", ScriptMessageCmd::error);

}

