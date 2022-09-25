// --- Items / Inventory System

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InventoryItem {
    pub id                  : Uuid,
    pub name                : String,
    pub item_type           : String,
    pub tile                : Option<TileId>,
    pub amount              : u16,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Inventory {
    pub items               : Vec<InventoryItem>,
    pub items_to_add        : Vec<String>
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items           : vec![],
            items_to_add    : vec![],
        }
    }

    /// Queues an item name to be added to the inventory.
    pub fn add(&mut self, name: &str) {
        self.items_to_add.push(name.to_string());
    }

    /// Add an item to the inventory.
    pub fn add_item(&mut self, item: InventoryItem) {
        self.items.push(item);
    }
}

pub fn script_register_item_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Inventory>("Inventory")
        .register_fn("add", Inventory::add);
        //.register_fn("debug", InventoryItem::debug)
        //.register_fn("error", InventoryItem::error);
}

