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

                if element.name.is_some() && element.name.clone().unwrap().to_lowercase() == data.primary_currency {
                    add_to_character_currency(instance_index, element.amount as f32, data);
                    data.action_subject_text = element.name.clone().unwrap();
                } else
                if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
                    if let Some(mut inv) = mess.write_lock::<Inventory>() {
                        if let Some(name) = element.name {
                            if element.state.is_none() {
                                inv.add(name.as_str(), element.amount);
                            } else {
                                let item = InventoryItem {
                                    id          : element.id,
                                    name        : name.clone(),
                                    item_type   : element.item_type,
                                    tile        : element.tile,
                                    state       : element.state,
                                    light       : element.light,
                                    amount      : element.amount,
                                    stackable   : element.stackable,
                                    static_item : element.static_item,
                                    price       : element.price,
                                    weight      : element.weight,
                                };
                                inv.add_item(item);
                            }
                            data.action_subject_text = name;
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

                        let loot = LootData {
                            id          : item.id,
                            name        : Some(item.name),
                            item_type   : item.item_type,
                            tile        : item.tile,
                            state       : item.state,
                            light       : item.light,
                            amount      : item.amount as i32,
                            stackable   : item.stackable as i32,
                            static_item : item.static_item,
                            price       : item.price,
                            weight      : item.weight
                        };

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

    let mut index: Option<usize> = None;
    if let Some(action) = &data.instances[instance_index].action {
        if let Some(inventory_index) = action.inventory_index {
            index = Some(inventory_index as usize);
        }
    }

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(mess) = data.scopes[instance_index].get_mut("inventory") {
        if let Some(mut inv) = mess.write_lock::<Inventory>() {

            if let Some(index) = index {
                let item_type = inv.items[index].item_type.clone().to_lowercase();

                if item_type == "weapon" {
                    let _item = inv.items.remove(index);
                } else
                if item_type == "gear" {
                    let _item = inv.items.remove(index);
                }
            }
        }
    }

    rc
}