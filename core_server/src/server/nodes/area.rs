use crate::prelude::*;

/// Always
pub fn always(_region_id: usize, _id: (Uuid, Uuid), _data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Right
}

/// Action
pub fn action(_region_id: usize, _id: (Uuid, Uuid), _data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Right
}

/// Enter Area
pub fn enter_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut enter_everyone = true;

    if let Some(value) = get_node_value((id.0, id.1, "character"), data, behavior_type) {
        if let Some(index) = value.to_integer() {
            if index == 1 {
                enter_everyone = false;
            }
        }
    }

    let mut found_character = false;
    let region = &mut data.region_data;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = get_instance_position(character_data.index, &data.instances) {
                if region.areas[area_index].area.contains(&(position.x, position.y)) {

                    if data.area_characters.contains_key(&area_index) == false {
                        data.area_characters.insert(area_index, vec![character_data.index]);
                    } else
                    if let Some(area_list) = data.area_characters.get_mut(&area_index) {
                        if area_list.contains(&character_data.index) == false {
                            area_list.push(character_data.index);
                        }
                    }

                    // Check if the character existed already in the area in the previous tick
                    let mut was_inside_already = false;
                    if let Some(area_list) = data.prev_area_characters.get(&area_index) {
                        for index in area_list {
                            if *index == character_data.index {
                                was_inside_already = true;
                            }
                        }
                    }

                    if was_inside_already == false {
                        if enter_everyone {
                            // Trigger always if somebody enters
                            found_character = true;
                        } else
                        if data.prev_area_characters.contains_key(&area_index) == false {
                            // This area was empty in the previous tick
                            found_character = true;
                        }
                    }
                }
            }
        }
    }

    if found_character {
        return BehaviorNodeConnector::Right;
    }

    BehaviorNodeConnector::Fail
}

/// Leave Area
pub fn leave_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    // let mut leave_everyone = true;

    // if let Some(value) = get_node_value((id.0, id.1, "character"), data, behavior_type, region_id) {
    //     if value.0 == 1.0 {
    //         leave_everyone = false;
    //     }
    // }

    let mut enter_everyone = true;

    if let Some(value) = get_node_value((id.0, id.1, "character"), data, behavior_type) {
        if let Some(index) = value.to_integer() {
            if index == 1 {
                enter_everyone = false;
            }
        }
    }

    let mut found_character = false;
    let region = &mut data.region_data;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = get_instance_position(character_data.index, &data.instances) {
                if region.areas[area_index].area.contains(&(position.x, position.y)) == false {

                    let mut was_inside_already = false;

                    if data.prev_area_characters.contains_key(&area_index) == true {
                        was_inside_already = true;
                    }

                    // Check if the character existed already in the area in the previous tick
                    if let Some(area_list) = data.prev_area_characters.get(&area_index) {
                        for index in area_list {
                            if *index == character_data.index {
                                was_inside_already = true;
                            }
                        }
                    }

                    if was_inside_already == false {
                        if enter_everyone {
                            // Trigger always if somebody enters
                            found_character = true;
                        } else
                        if data.prev_area_characters.contains_key(&area_index) == false {
                            // This area was empty in the previous tick
                            found_character = true;
                        }
                    }
                }
            }
        }
    }

    if found_character {
        return BehaviorNodeConnector::Right;
    }

    BehaviorNodeConnector::Fail
}

/// Inside Area
pub fn inside_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut found_character = false;
    let region = &mut data.region_data;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = get_instance_position(character_data.index, &data.instances) {
                if region.areas[area_index].area.contains(&(position.x, position.y)) {
                    //println!("{} is in area {}", data.instances[*instance_index].name, region.data.areas[id.0].name);
                    if data.area_characters.contains_key(&area_index) == false {
                        data.area_characters.insert(area_index, vec![character_data.index]);
                    } else
                    if let Some(area_list) = data.area_characters.get_mut(&area_index) {
                        if area_list.contains(&character_data.index) == false {
                            area_list.push(character_data.index);
                        }
                    }
                    found_character = true;
                }
            }
        }
    }

    if found_character {
        return BehaviorNodeConnector::Right;
    }
    BehaviorNodeConnector::Fail
}

/// Overlay Tiles
pub fn overlay_tiles(area_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {
    let region = &mut data.region_data;
    for pos in &region.areas[area_index].area {
        if let Some(t) = region.layer4.get(&pos) {
            data.displacements.insert(*pos, t.clone());
        }
    }
    BehaviorNodeConnector::Fail
}

/// Teleport Area
pub fn teleport_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let value = get_node_value((id.0, id.1, "position"), data, behavior_type);

    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&area_index) {
        if let Some(value) = value {
            for index in area_list {
                //data.instances[*index].position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                match &value {
                    Value::Position(position) => {
                        data.instances[*index].position = Some(position.clone());
                    }
                    _ => {},
                }
                data.instances[*index].old_position = None;
                data.instances[*index].max_transition_time = 0;
                data.instances[*index].curr_transition_time = 0;
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// Message Area
pub fn message_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut message_type : MessageType = MessageType::Status;
    let text;

    // Message Type
    if let Some(value) = get_node_value((id.0, id.1, "type"), data, behavior_type) {
        if let Some(index) = value.to_integer() {
            message_type = match index {
                1 => MessageType::Say,
                2 => MessageType::Yell,
                3 => MessageType::Private,
                4 => MessageType::Debug,
                _ => MessageType::Status
            }
        }
    }

    if let Some(value) = get_node_value((id.0, id.1, "text"), data, behavior_type) {
        text = value.to_string_value();
    } else {
        text = "Hello".to_string();
    }

    /*
    // Do I need to evaluate the script for variables ?
    if text.contains("${") {
        data.scopes[instance_index].push("Self", data.instances[instance_index].name.clone());
        if let Some(target_index) = data.instances[instance_index].target_instance_index {
            data.scopes[instance_index].push("Target", data.instances[target_index].name.clone());
        }
        let r = data.engine.eval_with_scope::<String>(&mut data.scopes[instance_index], format!("`{}`", text).as_str());
        if let Some(rc) = r.ok() {
            text = rc;
        }
    }*/

    if let Some(action_index) = data.curr_action_inst_index {
        let message_data = MessageData { message_type, message: text.clone(), from: "System".to_string(), buffer: None, right: None, center: None };
        data.instances[action_index].messages.push(message_data.clone());
    } else
    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&area_index) {
        let message_data = MessageData { message_type, message: text.clone(), from: "System".to_string(), buffer: None, right: None, center: None };
        for index in area_list {
            data.instances[*index].messages.push(message_data.clone());
        }
    }
    BehaviorNodeConnector::Fail
}


/// Audio Area
pub fn audio_area(area_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(value) = get_node_value((id.0, id.1, "audio"), data, behavior_type) {

        if let Some(audio_file) = value.to_string() {
            // Somebody is in the area ?
            if let Some(area_list) = data.area_characters.get(&area_index) {
                for index in area_list {
                    data.instances[*index].audio.push(audio_file.clone());
                }
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// Light Area
pub fn light_area(area_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {
    let region = &mut data.region_data;
    for pos in &region.areas[area_index].area {
        let light = LightData::new(core_shared::lightdata::LightType::PointLight, (pos.0, pos.1), 1);
        data.lights.push(light);
    }
    BehaviorNodeConnector::Fail
}