use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };
use crate::gamedata::get_node_value;
use core_shared::asset::TileUsage;
use core_shared::message::{MessageType, MessageData};

/// Enter Area
pub fn enter_area(region_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut enter_everyone = true;

    if let Some(value) = get_node_value((id.0, id.1, "character"), data, behavior_type, region_id) {
        if value.0 == 1.0 {
            enter_everyone = false;
        }
    }

    if enter_everyone {

    }

    BehaviorNodeConnector::Fail
}

/// Leave Area
pub fn leave_area(region_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut leave_everyone = true;

    if let Some(value) = get_node_value((id.0, id.1, "character"), data, behavior_type, region_id) {
        if value.0 == 1.0 {
            leave_everyone = false;
        }
    }

    if leave_everyone {

    }

    BehaviorNodeConnector::Fail
}

/// Inside Area
pub fn inside_area(region_id: usize, id: (usize, usize), data: &mut GameData, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut found_character = false;
    if let Some(region) = data.regions.get_mut(&region_id) {
        if let Some(characters) = data.characters.get(&region_id) {
            for character_data in characters {
                if let Some(position) = data.instances[character_data.index].position {
                    if region.data.areas[id.0].area.contains(&(position.1, position.2)) {
                        //println!("{} is in area {}", data.instances[*instance_index].name, region.data.areas[id.0].name);
                        if data.area_characters.contains_key(&(region_id, id.0)) == false {
                            data.area_characters.insert((region_id, id.0), vec![character_data.index]);
                        } else
                        if let Some(area_list) = data.area_characters.get_mut(&(region_id, id.0)) {
                            area_list.push(character_data.index);
                        }
                        found_character = true;
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

/// Displace Tiles
pub fn displace_tiles(region_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(value) = get_node_value((id.0, id.1, "tile"), data, behavior_type, region_id) {
        let tile_id = (value.0 as usize, value.1 as usize, value.2 as usize, TileUsage::Environment);

        // Filter based ?
        if let Some(value) = get_node_value((id.0, id.1, "filter"), data, behavior_type, region_id) {
            let filter_id = (value.0 as usize, value.1 as usize, value.2 as usize);

            if let Some(region) = data.regions.get_mut(&region_id) {

                for (x,y) in &region.data.areas[id.0].area {
                    let tiles = region.get_value_without_displacements((*x, *y));

                    for tile in tiles {
                        if tile.0 == filter_id.0 && tile.1 == filter_id.1 && tile.2 == filter_id.2 {
                            region.displacements.insert((*x, *y), tile_id.clone());
                        }
                    }
                }
            }
        } else {
            // No filter, displace all
            if let Some(region) = data.regions.get_mut(&region_id) {
                for (x,y) in &region.data.areas[id.0].area {
                    region.displacements.insert((*x, *y), tile_id.clone());
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// Teleport Area
pub fn teleport_area(region_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let value = get_node_value((id.0, id.1, "position"), data, behavior_type, region_id);

    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&(region_id, id.0)) {
        if let Some(value) = value {
            for index in area_list {
                data.instances[*index].position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// Message Area
pub fn message_area(region_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut message_type : MessageType = MessageType::Status;
    let text;

    // Message Type
    if let Some(value) = get_node_value((id.0, id.1, "type"), data, behavior_type, 0) {
        message_type = match value.0 as usize {
            1 => MessageType::Say,
            2 => MessageType::Yell,
            3 => MessageType::Private,
            4 => MessageType::Debug,
            _ => MessageType::Status
        }
    }

    if let Some(value) = get_node_value((id.0, id.1, "text"), data, behavior_type, 0) {
        text = value.4;
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

    // Somebody is in the area ?
    if let Some(area_list) = data.area_characters.get(&(region_id, id.0)) {

        let message_data = MessageData { message_type, message: text.clone(), from: "System".to_string() };

        for index in area_list {
            data.instances[*index].messages.push(message_data.clone());
        }

    }
    BehaviorNodeConnector::Fail
}