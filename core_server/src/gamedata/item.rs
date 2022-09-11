use crate::prelude::*;

// Generate item sink

pub fn update_item_sink(sink: &mut PropertySink) {

    if sink.contains("item_type") == false {
        sink.properties.insert(0,Property::new_color("item_type".to_string(), "\"Tool\"".to_string()));
    }
}

pub fn generate_item_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map : FxHashMap<String, Vec<String>> = FxHashMap::default();

    map.insert("item_type".to_string(), vec!["Type of the item, either \"Weapon\", \"Gear\" or \"Tool\"".to_string()]);

    map
}