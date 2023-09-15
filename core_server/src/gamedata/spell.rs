//use crate::prelude::*;
use core_shared::prelude::*;

// Generate item sink

pub fn update_spell_sink(sink: &mut PropertySink) {
    if sink.contains("classes") == false {
        sink.properties.push(Property::new_string(
            "classes".to_string(),
            "All".to_string(),
        ));
    }

    if sink.contains("races") == false {
        sink.properties
            .push(Property::new_string("races".to_string(), "All".to_string()));
    }

    if sink.contains("level") == false {
        sink.properties
            .push(Property::new_int("level".to_string(), 1));
    }

    if sink.contains("spell_distance") == false {
        sink.properties
            .push(Property::new_int("spell_distance".to_string(), 3));
    }
}

pub fn generate_spell_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map: FxHashMap<String, Vec<String>> = FxHashMap::default();

    map.insert(
        "classes".to_string(),
        vec!["Classes which can use the spell. \"All\" for all classes.".to_string()],
    );
    map.insert(
        "races".to_string(),
        vec!["Races which can use the spell. \"All\" for all races.".to_string()],
    );
    map.insert(
        "level".to_string(),
        vec!["The minimum level for this spell.".to_string()],
    );
    map.insert(
        "spell_distance".to_string(),
        vec!["The maximum distance for the spell. Default is 3.".to_string()],
    );

    map
}
