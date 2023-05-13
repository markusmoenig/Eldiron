extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;
use crate::prelude::*;

/// Returns an integer value for the given node.
fn get_node_integer(id: Uuid, value_name: &str, nodes: &mut FxHashMap<Uuid, BehaviorNode>) -> Option<i32> {
    if let Some(node) = nodes.get_mut(&id) {
        for (name, value) in &node.values {
            if *name == value_name {
                if let Some(int) = value.to_integer() {
                    return Some(int);
                }
                break;
            }
        }
    }
    None
}

/// Returns a string value for the given node.
fn get_node_string(id: Uuid, value_name: &str, nodes: &mut FxHashMap<Uuid, BehaviorNode>) -> Option<String> {
    if let Some(node) = nodes.get_mut(&id) {
        for (name, value) in &node.values {
            if *name == value_name {
                if let Some(v) = value.to_string() {
                    return Some(v);
                }
                break;
            }
        }
    }
    None
}

/// Returns a value for the given node.
fn get_node_value(id: Uuid, value_name: &str, nodes: &mut FxHashMap<Uuid, BehaviorNode>) -> Option<Value> {
    if let Some(node) = nodes.get_mut(&id) {
        for (name, value) in &node.values {
            if *name == value_name {
                return Some(value.clone());
            }
        }
    }
    None
}

/// Always
pub fn node_always_area(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Right
}

/// Action
pub fn node_action_area(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Right
}

/// Enter Area
pub fn node_enter_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut enter_everyone = true;

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let nodes = &mut data.region_area_behavior[data.curr_area_index].nodes;

    if let Some(index) = get_node_integer(id.1, "character", nodes) {
        if index == 1 {
            enter_everyone = false;
        }
    }

    let mut found_character = false;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = data.get_instance_position(character_data.index) {
                if data.region_data.areas[data.curr_area_index].area.contains(&(position.x, position.y)) {

                    if data.area_characters.contains_key(&data.curr_area_index) == false {
                        data.area_characters.insert(data.curr_area_index, vec![character_data.index]);
                    } else
                    if let Some(area_list) = data.area_characters.get_mut(&data.curr_area_index) {
                        if area_list.contains(&character_data.index) == false {
                            area_list.push(character_data.index);
                        }
                    }

                    // Check if the character existed already in the area in the previous tick
                    let mut was_inside_already = false;
                    if let Some(area_list) = data.prev_area_characters.get(&data.curr_area_index) {
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
                        if data.prev_area_characters.contains_key(&data.curr_area_index) == false {
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
pub fn node_leave_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut leave_everyone = true;

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let nodes = &mut data.region_area_behavior[data.curr_area_index].nodes;

    if let Some(index) = get_node_integer(id.1, "character", nodes) {
        if index == 1 {
            leave_everyone = false;
        }
    }

    let mut found_character = false;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = data.get_instance_position(character_data.index) {
                if data.region_data.areas[data.curr_area_index].area.contains(&(position.x, position.y)) == false {

                    let mut was_inside_already = false;

                    if data.prev_area_characters.contains_key(&data.curr_area_index) == true {
                        was_inside_already = true;
                    }

                    // Check if the character existed already in the area in the previous tick
                    if let Some(area_list) = data.prev_area_characters.get(&data.curr_area_index) {
                        for index in area_list {
                            if *index == character_data.index {
                                was_inside_already = true;
                            }
                        }
                    }

                    if was_inside_already == false {
                        if leave_everyone {
                            // Trigger always if somebody leaves
                            found_character = true;
                        } else
                        if data.prev_area_characters.contains_key(&data.curr_area_index) == false {
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
pub fn node_inside_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut found_character = false;
    if let Some(characters) = data.characters.get(&id.0) {
        for character_data in characters {
            if let Some(position) = data.get_instance_position(character_data.index) {
                if data.region_data.areas[data.curr_area_index].area.contains(&(position.x, position.y)) {
                    //println!("{} is in area {}", data.instances[*instance_index].name, region.data.areas[id.0].name);
                    if data.area_characters.contains_key(&data.curr_area_index) == false {
                        data.area_characters.insert(data.curr_area_index, vec![character_data.index]);
                    } else
                    if let Some(area_list) = data.area_characters.get_mut(&data.curr_area_index) {
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
pub fn node_teleport_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let nodes = &mut data.region_area_behavior[data.curr_area_index].nodes;

    let value = get_node_value(id.1, "position", nodes);

    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&data.curr_area_index) {
        if let Some(value) = value {
            for index in area_list {
                //data.instances[*index].position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                match &value {
                    Value::Position(position) => {
                        data.character_instances[*index].position = Some(position.clone());
                    }
                    _ => {},
                }
                data.character_instances[*index].old_position = None;
                data.character_instances[*index].max_transition_time = 0;
                data.character_instances[*index].curr_transition_time = 0;
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// Message Area
pub fn node_message_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut message_type : MessageType = MessageType::Status;
    let text;

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let nodes = &mut data.region_area_behavior[data.curr_area_index].nodes;

    // Message Type
    if let Some(index) = get_node_integer(id.1, "status", nodes) {
        message_type = match index {
            1 => MessageType::Say,
            2 => MessageType::Yell,
            3 => MessageType::Tell,
            4 => MessageType::Debug,
            _ => MessageType::Status
        }
    }

    if let Some(value) = get_node_string(id.1, "text", nodes) {
        text = value;
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
        data.character_instances[action_index].messages.push(message_data.clone());
    } else
    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&data.curr_area_index) {
        let message_data = MessageData { message_type, message: text.clone(), from: "System".to_string(), buffer: None, right: None, center: None };
        for index in area_list {
            data.character_instances[*index].messages.push(message_data.clone());
        }
    }

    BehaviorNodeConnector::Fail
}


/// Audio Area
pub fn node_audio_area(id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let nodes = &mut data.region_area_behavior[data.curr_area_index].nodes;

    if let Some(value) = get_node_value(id.1, "audio", nodes) {

        if let Some(audio_file) = value.to_string() {
            // Somebody is in the area ?
            if let Some(area_list) = data.area_characters.get(&data.curr_area_index) {
                for index in area_list {
                    data.character_instances[*index].audio.push(audio_file.clone());
                }
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// Light Area
pub fn node_light_area(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let region = &mut data.region_data;
    for pos in &region.areas[data.curr_area_index].area {
        let light = LightData::new(core_shared::lightdata::LightType::PointLight, (pos.0, pos.1), 1);
        data.lights.push(light);
    }
    BehaviorNodeConnector::Fail
}