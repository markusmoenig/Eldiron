extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;
use crate::prelude::*;


/// Player invokes an action
pub fn node_player_action(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let mut dp:Option<Position> = None;
    {
        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        if let Some(p) = &data.character_instances[data.curr_index].position {
            if let Some(action) = &data.character_instances[data.curr_index].action {
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
                } else
                if action.direction == PlayerDirection::Coordinate {
                    if let Some(coord) = action.coordinate {
                        dp = Some(Position::new(p.region, coord.0, coord.1));
                        data.action_direction_text = "".to_string();
                    }
                }
            }
        }
        data.character_instances[data.curr_index].action = None;
    }

    let mut action_name: String = "".to_string();

    if let Some(value) = get_node_string(id, "action", nodes) {
        action_name = value;
    }

    execute_targetted_action(action_name, dp)
}

/// Player move
pub fn node_player_move(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut delay : i32 = 0;
    if let Some(d) = eval_script_integer(id, "delay", nodes) {
        delay = d.clamp(0, 10);
    }

    let mut position:Option<Position> = None;
    let mut dp:Option<Position> = None;

    // Apply the speed delay
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    data.character_instances[data.curr_index].sleep_cycles = delay as usize;

    if let Some(p) = &data.character_instances[data.curr_index].position {
        position = Some(p.clone());
        if let Some(action) = &data.character_instances[data.curr_index].action {
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
            } else
            if action.direction == PlayerDirection::Coordinate {
                if let Some(coord) = action.coordinate {
                    dp = Some(Position::new(p.region, coord.0, coord.1));
                    data.action_direction_text = "".to_string();
                }
            }
        }
    }
    data.character_instances[data.curr_index].action = None;

    let mut rc = walk_towards(position, dp, false, data);
    if rc == BehaviorNodeConnector::Right {
        data.character_instances[data.curr_index].max_transition_time = delay as usize;
        data.character_instances[data.curr_index].curr_transition_time = 1;
        rc = BehaviorNodeConnector::Success;
    }
    rc
}

/// Player wants to take something
pub fn node_player_take(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut dp:Option<Position> = None;
    if let Some(p) = &data.character_instances[data.curr_index].position {
        if let Some(action) = &data.character_instances[data.curr_index].action {
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
            } else
            if action.direction == PlayerDirection::Coordinate {
                if let Some(coord) = action.coordinate {
                    dp = Some(Position::new(p.region, coord.0, coord.1));
                    data.action_direction_text = "".to_string();
                }
            }
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(dp) = dp {
        if let Some(position) = &data.character_instances[data.curr_index].position {
            // Make sure the distance to the loot is 0 or 1
            let distance = compute_distance(&position, &dp);
            if  distance == 1 || distance == 0 {
                if let Some(loot) = data.loot.get_mut(&(dp.x, dp.y)) {
                    for index in (0..loot.len()).rev() {
                        if loot[index].static_item { continue; }
                        let element = loot.remove(index);

                        // TODO
                        // if element.name.to_lowercase() == data.primary_currency {
                        //     add_to_character_currency(instance_index, element.amount as f32, data);
                        //     data.action_subject_text = element.name;
                        // } else
                        // if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
                        //     if let Some(mut inv) = mess.write_lock::<Inventory>() {
                        //         if element.name.is_empty() == false {
                        //             if element.state.is_none() {
                        //                 inv.add(element.name.as_str(), element.amount);
                        //             } else {
                        //                 inv.add_item(element.clone());
                        //             }
                        //             data.action_subject_text = element.name;
                        //         }
                        //     }
                        // }
                        rc = BehaviorNodeConnector::Success;
                        break;
                    }
                }
            }
        }
    }

    data.character_instances[data.curr_index].action = None;
    rc
}

/// Player wants to drop something
pub fn node_player_drop(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut index: Option<usize> = None;
    if let Some(action) = &data.character_instances[data.curr_index].action {
        if let Some(inventory_index) = action.inventory_index {
            index = Some(inventory_index as usize);
        }
    }

    // TODO
    // if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
    //     if let Some(mut inv) = mess.write_lock::<Inventory>() {

    //         if let Some(index) = index {
    //             if index < inv.items.len() {
    //                 let mut item = inv.items[index].clone();
    //                 inv.items.remove(index);

    //                 if let Some(p) = &data.instances[instance_index].position {

    //                     if let Some(mut light) = item.light.clone() {
    //                         light.position = (p.x, p.y);
    //                         item.light = Some(light);
    //                     }

    //                     let loot = item.clone();

    //                     if let Some(existing_loot) = data.loot.get_mut(&(p.x, p.y)) {
    //                         existing_loot.push(loot);
    //                     } else {
    //                         data.loot.insert((p.x, p.y), vec![loot]);
    //                     }

    //                     return BehaviorNodeConnector::Success;
    //                 }
    //             }
    //         }
    //     }
    // }

    BehaviorNodeConnector::Fail
}

/// Assign target
pub fn node_player_target(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let mut dp:Option<Position> = None;
    if let Some(p) = &data.character_instances[data.curr_index].position {
        if let Some(action) = &data.character_instances[data.curr_index].action {
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
            } else
            if action.direction == PlayerDirection::Coordinate {
                if let Some(coord) = action.coordinate {
                    dp = Some(Position::new(p.region, coord.0, coord.1));
                    data.action_direction_text = "".to_string();
                }
            }
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    //data.scopes[instance_index].set_value("failure", FailureEnum::No);

    if let Some(dp) = &dp {
        if let Some(position) = &data.character_instances[data.curr_index].position {
            // Make sure the target is within weapons range
            let distance = compute_distance(&position, &dp);
            let weapon_distance = get_weapon_distance(data.curr_index, "main hand".to_string(), data);
            if  distance as i32 <= weapon_distance {
                for inst_index in 0..data.character_instances.len() {
                    if inst_index != data.curr_index {
                        // Only track if the state is OK
                        if data.character_instances[inst_index].state.is_alive() {
                            if let Some(pos) = &data.character_instances[inst_index].position {
                                if *dp == *pos {
                                    data.character_instances[data.curr_index].target_instance_index = Some(inst_index);
                                    rc = BehaviorNodeConnector::Success;
                                }
                            }
                        }
                    }
                }
            } else {
                //data.scopes[instance_index].set_value("failure", FailureEnum::TooFarAway);
            }
        }
    }

    rc
}

/// Player wants to drop something
pub fn node_player_equip(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    // Get the inventory index of the item to equip
    let mut index: Option<usize> = None;
    if let Some(action) = &data.character_instances[data.curr_index].action {
        if let Some(inventory_index) = action.inventory_index {
            index = Some(inventory_index as usize);
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    let mut to_equip: Option<Item> = None;
    let mut to_add_back_to_inventory: Vec<Item> = vec![];

    // TODO
    // // Remove the item to equip from the inventory
    // if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
    //     if let Some(mut inv) = mess.write_lock::<Inventory>() {
    //         if let Some(index) = index {
    //             to_equip = Some(inv.items.remove(index));
    //         }
    //     }
    // }

    // if let Some(to_equip) = to_equip {
    //     let item_type = to_equip.item_type.clone().to_lowercase();
    //     if let Some(slot) = to_equip.slot.clone() {
    //         if item_type == "weapon" {
    //             if let Some(mess) = data.scopes[instance_index].get_mut("weapons") {
    //                 if let Some(mut weapons) = mess.write_lock::<Weapons>() {
    //                     // Remove existing item in the slot
    //                     if let Some(w) = weapons.slots.remove(&slot) {
    //                         to_add_back_to_inventory.push(w);
    //                     }
    //                     // Insert the new weapon into the slot
    //                     weapons.slots.insert(slot, to_equip);
    //                     rc = BehaviorNodeConnector::Success;
    //                 }
    //             }
    //         } else
    //         if item_type == "gear" {
    //             if let Some(mess) = data.scopes[instance_index].get_mut("gear") {
    //                 if let Some(mut gear) = mess.write_lock::<Gear>() {
    //                     // Remove existing item in the slot
    //                     if let Some(g) = gear.slots.remove(&slot) {
    //                         to_add_back_to_inventory.push(g);
    //                     }
    //                     // Insert the new weapon into the slot
    //                     gear.slots.insert(slot, to_equip);
    //                     rc = BehaviorNodeConnector::Success;
    //                 }
    //             }
    //         }
    //     }
    //}

    // Add removed items in the equipped slot(s) back into the inventory
    if to_add_back_to_inventory.is_empty() == false {
        // TODO
        // if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
        //     if let Some(mut inv) = mess.write_lock::<Inventory>() {
        //         for item in to_add_back_to_inventory {
        //             inv.items.push(item);
        //         }
        //     }
        // }
    }

    rc
}