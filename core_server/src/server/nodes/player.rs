use crate::prelude::*;

/// Player moves
pub fn player_move(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    let mut speed : f32 = 8.0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "speed".to_string()), data) {
        speed = rc;
    }

    // Apply the speed delay
    let delay = speed.clamp(0.0, f32::MAX);
    data.instances[instance_index].sleep_cycles = delay as usize;

    let mut dp:Option<Position> = None;
    if let Some(p) = &data.instances[instance_index].position {
        if let Some(action) = &data.instances[instance_index].action {
            if action.direction == PlayerDirection::North {
                dp = Some(Position::new(p.region, p.x, p.y - 1));
                data.action_direction_text = "North".to_string();
            } else
            if action.direction == PlayerDirection::South {
                dp = Some(Position::new(p.region, p.x, p.y + 1));
                data.action_direction_text = "South".to_string();
            } else
            if action.direction == PlayerDirection::East {
                dp = Some(Position::new(p.region, p.x + 1, p.y));
                data.action_direction_text = "East".to_string();
            } else
            if action.direction == PlayerDirection::West {
                dp = Some(Position::new(p.region, p.x - 1, p.y));
                data.action_direction_text = "West".to_string();
            }
        }
    }

    data.instances[instance_index].action = None;

    let mut rc = walk_towards(instance_index, data.instances[instance_index].position.clone(), dp, false, data);
    if rc == BehaviorNodeConnector::Right {
        data.instances[instance_index].max_transition_time = delay as usize;
        data.instances[instance_index].curr_transition_time = 1;
        rc = BehaviorNodeConnector::Success;
    }
    // println!("rc {:?}", rc);
    rc
}

/// Player invokes an action
pub fn player_action(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut dp:Option<Position> = None;
    if let Some(p) = &data.instances[instance_index].position {
        if let Some(action) = &data.instances[instance_index].action {
            if action.direction == PlayerDirection::North {
                dp = Some(Position::new(p.region, p.x, p.y - 1));
                data.action_direction_text = "North".to_string();
            } else
            if action.direction == PlayerDirection::South {
                dp = Some(Position::new(p.region, p.x, p.y + 1));
                data.action_direction_text = "South".to_string();
            } else
            if action.direction == PlayerDirection::East {
                dp = Some(Position::new(p.region, p.x + 1, p.y));
                data.action_direction_text = "East".to_string();
            } else
            if action.direction == PlayerDirection::West {
                dp = Some(Position::new(p.region, p.x - 1, p.y));
                data.action_direction_text = "West".to_string();
            }
        }
    }

    let mut action_name = "".to_string();

    if let Some(value) = get_node_value((id.0, id.1, "action"), data, behavior_type) {
        if let Some(name) = value.to_string() {
            action_name = name;
        }
    }

    data.instances[instance_index].action = None;

    execute_region_action(instance_index, action_name, dp, data)
}

/// Player wants to take something
pub fn player_take(instance_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut dp:Option<Position> = None;
    if let Some(p) = &data.instances[instance_index].position {
        if let Some(action) = &data.instances[instance_index].action {
            if action.direction == PlayerDirection::North {
                dp = Some(Position::new(p.region, p.x, p.y - 1));
                data.action_direction_text = "North".to_string();
            } else
            if action.direction == PlayerDirection::South {
                dp = Some(Position::new(p.region, p.x, p.y + 1));
                data.action_direction_text = "South".to_string();
            } else
            if action.direction == PlayerDirection::East {
                dp = Some(Position::new(p.region, p.x + 1, p.y));
                data.action_direction_text = "East".to_string();
            } else
            if action.direction == PlayerDirection::West {
                dp = Some(Position::new(p.region, p.x - 1, p.y));
                data.action_direction_text = "West".to_string();
            }
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(dp) = dp {
        if let Some(loot) = data.loot.get_mut(&(dp.x, dp.y)) {
            for index in 0..loot.len() {
                if loot[index].static_item { continue; }
                let element = loot.remove(index);

                if element.name.to_lowercase() == data.primary_currency {
                    add_to_character_currency(instance_index, element.amount as f32, data);
                    data.action_subject_text = element.name;
                } else
                if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
                    if let Some(mut inv) = mess.write_lock::<Inventory>() {
                        if element.name.is_empty() == false {
                            if element.state.is_none() {
                                inv.add(element.name.as_str(), element.amount);
                            } else {
                                inv.add_item(element.clone());
                            }
                            data.action_subject_text = element.name;
                        }
                    }
                }
                rc = BehaviorNodeConnector::Success;
                break;
            }
        }
    }

    data.instances[instance_index].action = None;
    rc
}

/// Player wants to drop something
pub fn player_drop(instance_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut index: Option<usize> = None;
    if let Some(action) = &data.instances[instance_index].action {
        if let Some(inventory_index) = action.inventory_index {
            index = Some(inventory_index as usize);
        }
    }

    if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
        if let Some(mut inv) = mess.write_lock::<Inventory>() {

            if let Some(index) = index {
                if index < inv.items.len() {
                    let mut item = inv.items[index].clone();
                    inv.items.remove(index);

                    if let Some(p) = &data.instances[instance_index].position {

                        if let Some(mut light) = item.light.clone() {
                            light.position = (p.x, p.y);
                            item.light = Some(light);
                        }

                        let loot = item.clone();

                        if let Some(existing_loot) = data.loot.get_mut(&(p.x, p.y)) {
                            existing_loot.push(loot);
                        } else {
                            data.loot.insert((p.x, p.y), vec![loot]);
                        }

                        return BehaviorNodeConnector::Success;
                    }
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// Player wants to drop something
pub fn player_target(instance_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut dp:Option<Position> = None;
    if let Some(p) = &data.instances[instance_index].position {
        if let Some(action) = &data.instances[instance_index].action {
            if action.direction == PlayerDirection::North {
                dp = Some(Position::new(p.region, p.x, p.y - 1));
                data.action_direction_text = "North".to_string();
            } else
            if action.direction == PlayerDirection::South {
                dp = Some(Position::new(p.region, p.x, p.y + 1));
                data.action_direction_text = "South".to_string();
            } else
            if action.direction == PlayerDirection::East {
                dp = Some(Position::new(p.region, p.x + 1, p.y));
                data.action_direction_text = "East".to_string();
            } else
            if action.direction == PlayerDirection::West {
                dp = Some(Position::new(p.region, p.x - 1, p.y));
                data.action_direction_text = "West".to_string();
            }
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(dp) = &dp {
        for inst_index in 0..data.instances.len() {
            if inst_index != instance_index {
                // Only track if the state is normal
                if data.instances[inst_index].state == BehaviorInstanceState::Normal {
                    if let Some(pos) = &data.instances[inst_index].position {
                        if *dp == *pos {
                            data.instances[instance_index].target_instance_index = Some(inst_index);
                            rc = BehaviorNodeConnector::Success;
                        }
                    }
                }
            }
        }
    }

    rc
}

/// Player wants to drop something
pub fn player_equip(instance_index: usize, _id: (Uuid, Uuid), data: &mut RegionInstance, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    // Get the inventory index of the item to equip
    let mut index: Option<usize> = None;
    if let Some(action) = &data.instances[instance_index].action {
        if let Some(inventory_index) = action.inventory_index {
            index = Some(inventory_index as usize);
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    let mut to_equip: Option<Item> = None;
    let mut to_add_back_to_inventory: Vec<Item> = vec![];

    // Remove the item to equip from the inventory
    if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
        if let Some(mut inv) = mess.write_lock::<Inventory>() {
            if let Some(index) = index {
                to_equip = Some(inv.items.remove(index));
            }
        }
    }

    if let Some(to_equip) = to_equip {
        let item_type = to_equip.item_type.clone().to_lowercase();
        if let Some(slot) = to_equip.slot.clone() {
            if item_type == "weapon" {
                if let Some(mess) = data.scopes[instance_index].get_mut("weapons") {
                    if let Some(mut weapons) = mess.write_lock::<Weapons>() {
                        // Remove existing item in the slot
                        if let Some(w) = weapons.slots.remove(&slot) {
                            to_add_back_to_inventory.push(w);
                        }
                        // Insert the new weapon into the slot
                        weapons.slots.insert(slot, to_equip);
                        rc = BehaviorNodeConnector::Success;
                    }
                }
            } else
            if item_type == "gear" {
                if let Some(mess) = data.scopes[instance_index].get_mut("gear") {
                    if let Some(mut gear) = mess.write_lock::<Gear>() {
                        // Remove existing item in the slot
                        if let Some(g) = gear.slots.remove(&slot) {
                            to_add_back_to_inventory.push(g);
                        }
                        // Insert the new weapon into the slot
                        gear.slots.insert(slot, to_equip);
                        rc = BehaviorNodeConnector::Success;
                    }
                }
            }
        }
    }

    // Add removed items in the equipped slot(s) back into the inventory
    if to_add_back_to_inventory.is_empty() == false {
        if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
            if let Some(mut inv) = mess.write_lock::<Inventory>() {
                for item in to_add_back_to_inventory {
                    inv.items.push(item);
                }
            }
        }
    }

    rc
}