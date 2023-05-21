// --- Items / Inventory System

use crate::prelude::*;

/// An inventory item
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Item {
    pub id                  : Uuid,
    pub name                : String,
    pub item_type           : String,
    pub tile                : Option<TileData>,
    pub state               : Option<State>,
    pub light               : Option<LightData>,
    pub slot                : Option<String>,
    pub use_skill           : Option<String>,
    pub amount              : i32,
    pub stackable           : i32,
    pub static_item         : bool,
    pub value               : Currency,
    pub weight              : f32,
    pub weapon_distance     : i32,
}

impl Item {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            name,
            item_type       : "tool".into(),
            tile            : None,
            state           : None,
            light           : None,
            slot            : None,
            use_skill       : None,
            amount          : 0,
            stackable       : i32::MAX,
            static_item     : false,
            value           : Currency::empty(),
            weight          : 0.0,
            weapon_distance : 1,
        }
    }

    /// Reads the Item properties from a PropertySink.
    pub fn read_from_sink(&mut self, sink: &PropertySink) {

        if let Some(static_item) = sink.get("static") {
            if let Some(st) = static_item.as_bool() {
                self.static_item = st;
            }
        }
        if let Some(stackable_item) = sink.get("stackable") {
            if let Some(st) = stackable_item.as_int() {
                if st >= 0 {
                    self.stackable = st;
                }
            }
        }
        if let Some(v) = sink.get("value") {
            if let Some(value) = v.to_currency() {
                self.value = value;
            }
        }
        if let Some(weight_item) = sink.get("weight") {
            let weight = weight_item.to_float();
            if weight >= 0.0 {
                self.weight = weight;
            }
        }
        if let Some(item_type) = sink.get("item_type") {
            if let Some(i_type) = item_type.as_string() {
                self.item_type = i_type;
            }
        }
        if let Some(item_slot) = sink.get("slot") {
            if let Some(slot) = item_slot.as_string() {
                self.slot = Some(slot);
            }
        }
        if let Some(item_use_skill) = sink.get("use_skill") {
            if let Some(use_skill) = item_use_skill.as_string() {
                self.use_skill = Some(use_skill);
            }
        }
        if let Some(weapon_distance) = sink.get("weapon_distance") {
            if let Some(wd) = weapon_distance.as_int() {
                if wd >= 0 {
                    self.weapon_distance = wd;
                }
            }
        }

    }

    // Getter

    pub fn get_name(&mut self) -> String {
        self.name.clone()
    }

    pub fn get_amount(&mut self) -> i32 {
        self.amount
    }

}

/// Inventory
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Inventory {
    pub items               : Vec<Item>,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items           : vec![],
        }
    }

    /// Add an item to the inventory.
    pub fn add_item(&mut self, mut item: Item) {
        if item.stackable > 1 && item.state.is_none() {
            for it in &mut self.items {
                if it.id == item.id {
                    if it.amount < item.stackable {
                        it.amount += item.amount;
                        if it.amount > item.stackable {
                            item.amount = it.amount - item.stackable;
                            it.amount = item.stackable;
                        } else {
                            return;
                        }
                    }
                }
            }
        }
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

    // Removes the item of the given name
    pub fn remove_item_by_name(&mut self, name: String) -> Option<Item> {
        let mut id : Option<Uuid> = None;
        for index in 0..self.items.len() {
            if self.items[index].name == name {
                id = Some(self.items[index].id);
            }
        }

        if let Some(id) = id {
            self.remove_item(id, 1)
        } else {
            None
        }
    }

    // Removes the given amount of items from the inventory and returns it
    pub fn remove_item(&mut self, id: Uuid, _amount: i32) -> Option<Item> {

        let mut to_remove : Option<usize> = None;
        for index in 0..self.items.len() {
            if self.items[index].id == id {
                to_remove = Some(index);
            }
        }

        if let Some(item_index) = to_remove {
            let item = self.items.remove(item_index);
            return Some(item);
        }

        None
    }


    pub fn get_item_at(&mut self, index: i32) -> Item {
        if index >= 0 && index < self.items.len() as i32 {
            return self.items[index as usize].clone()
        }
        Item::new(Uuid::new_v4(), "".to_string())
    }

}

// Implement 'IntoIterator' trait
impl IntoIterator for Inventory {
    type Item = Item;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

pub fn script_register_inventory_api(engine: &mut rhai::Engine) {

    engine.register_type_with_name::<Item>("Item")
        .register_get("name", Item::get_name)
        .register_get("amount", Item::get_amount);

    engine.register_type_with_name::<Inventory>("Inventory")
        .register_fn("len", Inventory::len)
        .register_fn("item_name_at", Inventory::item_name_at)
        .register_fn("item_amount_at", Inventory::item_amount_at)
        .register_iterator::<Inventory>();

}
