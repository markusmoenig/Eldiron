//use crate::prelude::*;
use core_shared::prelude::*;

// Generate item sink

pub fn update_item_sink(sink: &mut PropertySink) {

    if sink.contains("item_type") == false {
        sink.properties.insert(0,Property::new_string("item_type".to_string(), "Tool".to_string()));
    }

    if sink.contains("state") == false {
        sink.properties.insert(1,Property::new_bool("state".to_string(), false));
    }

    if sink.contains("stackable") == false {
        sink.properties.insert(2,Property::new_int("stackable".to_string(), 1));
    }

    if sink.contains("static") == false {
        sink.properties.insert(3,Property::new_bool("static".to_string(), false));
    }

    if sink.contains("price") == false {
        sink.properties.insert(4,Property::new_float("price".to_string(), 0.0));
    }

    if sink.contains("weight") == false {
        sink.properties.push(Property::new_float("weight".to_string(), 0.0));
    }

    if sink.contains("slot") == false {
        sink.properties.push(Property::new_string("slot".to_string(), "".to_string()));
    }

    if sink.contains("weapon_distance") == false {
        sink.properties.push(Property::new_int("weapon_distance".to_string(), 1));
    }
}

pub fn generate_item_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map : FxHashMap<String, Vec<String>> = FxHashMap::default();

    map.insert("item_type".to_string(), vec!["Type of the item, either \"Weapon\", \"Gear\" or \"Tool\"".to_string()]);
    map.insert("state".to_string(), vec!["true if the item should have it's own state (variables).".to_string()]);
    map.insert("stackable".to_string(), vec!["Value greater than 1 if item should be stackable. Only for items without state.".to_string()]);
    map.insert("static".to_string(), vec!["True if the item is static, i.e. cannot be picked up (campfire etc.).".to_string()]);
    map.insert("price".to_string(), vec!["The sales price of the item. 0.0 if the item cannot be sold.".to_string()]);
    map.insert("weight".to_string(), vec!["The weight of the item.".to_string()]);
    map.insert("slot".to_string(), vec!["If item_type is \"Weapon\" or \"Gear\" the slot it fits in.".to_string()]);
    map.insert("weapon_distance".to_string(), vec!["The maximum distance for a weapon. Default is 1 (Sword etc.).".to_string()]);

    map
}