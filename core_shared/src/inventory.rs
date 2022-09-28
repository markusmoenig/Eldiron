// --- Items / Inventory System

use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct InventoryItem {
    pub id                  : Uuid,
    pub name                : String,
    pub item_type           : String,
    pub tile                : Option<TileData>,
    pub state               : Option<ScopeBuffer>,
    pub amount              : u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Inventory {
    pub items               : Vec<InventoryItem>,
    pub items_to_add        : Vec<(String, u32)>
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items           : vec![],
            items_to_add    : vec![],
        }
    }

    /// Queues an item name to be added to the inventory.
    pub fn add(&mut self, name: &str, amount: i32) {
        self.items_to_add.push((name.to_string(), amount as u32));
    }

    /// Add an item to the inventory.
    pub fn add_item(&mut self, item: InventoryItem) {
        self.items.push(item);
    }

    /// Length of the item array
    pub fn len(&mut self) -> i32 {
        self.items.len() as i32
    }

    /// Returns the item name at the given index.
    pub fn item_name_at(&mut self, index: i32) -> String {
        if index >= 0 && index < self.items.len() as i32 {
            return self.items[index as usize].name.clone();
        }
        "".to_string()
    }

    /// Returns the item amount at the given index.
    pub fn item_amount_at(&mut self, index: i32) -> i32 {
        if index >= 0 && index < self.items.len() as i32 {
            return self.items[index as usize].amount as i32;
        }
        0
    }
}

pub fn script_register_inventory_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Inventory>("Inventory")
        .register_fn("len", Inventory::len)
        .register_fn("item_name_at", Inventory::item_name_at)
        .register_fn("item_amount_at", Inventory::item_amount_at)
        .register_fn("add", Inventory::add);

}

