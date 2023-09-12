extern crate ref_thread_local;

use ref_thread_local::RefThreadLocal;
use crate::prelude::*;


pub fn node_script(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    eval_script(id, "script", nodes);
    BehaviorNodeConnector::Bottom
}

/// expression
pub fn node_expression(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    if eval_script_bool(id, "expression", nodes) {
        return BehaviorNodeConnector::Success;
    }
    BehaviorNodeConnector::Fail
}

pub fn node_message(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let mut message_type : MessageType = MessageType::Status;
    let mut text;

    if let Some(value) = get_node_integer(id, "type", nodes) {
        message_type = match value {
            1 => MessageType::Say,
            2 => MessageType::Yell,
            3 => MessageType::Tell,
            4 => MessageType::Debug,
            _ => MessageType::Status
        }
    }

    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(value) = get_node_string(id, "text", nodes) {
        text = value;
    } else {
        text = "Message".to_string()
    }

    if text.contains("${DIRECTION}") {
        text = text.replace("${DIRECTION}", &data.action_direction_text);
    }
    if text.contains("${CONTEXT}") {
        text = text.replace("${CONTEXT}", &data.action_subject_text);
    }
    if text.contains("${DEF_CONTEXT}") {
        let def = if text.starts_with("${DEF_CONTEXT}") { "The ".to_string() } else { "the ".to_string() };
        let string = def + data.action_subject_text.to_lowercase().as_str();
        text = text.replace("${DEF_CONTEXT}", &string);
    }
    if text.contains("${TARGET}") {
        let mut target_text = "".to_string();
        if let Some(target_index) = data.character_instances[data.curr_index].target_instance_index {
            target_text = data.character_instances[target_index].name.clone();
        }
        text = text.replace("${TARGET}", &target_text);
    }
    if text.contains("${DEF_TARGET}") {
        let mut target_text = "".to_string();
        if let Some(target_index) = data.character_instances[data.curr_index].target_instance_index {
            let def = if text.starts_with("${DEF_TARGET}") { "The ".to_string() } else { "the ".to_string() };
            target_text = def + data.character_instances[target_index].name.to_lowercase().as_str();
        }
        text = text.replace("${DEF_TARGET}", &target_text);
    }
    /*
    if text.contains("${DAMAGE}") {
        let mut damage_text = "".to_string();
        if let Some(target_index) = data.instances[instance_index].target_instance_index {
            if let Some(damage) = data.instances[target_index].damage_to_be_dealt {
                damage_text = damage.to_string();
            }
        } else
        if let Some(damage) = data.instances[instance_index].damage_to_be_dealt {
            damage_text = damage.to_string();
        }

        text = text.replace("${DAMAGE}", &damage_text);
    }
    if text.contains("${HEALING}") {
        let mut healing_text = "".to_string();
        if let Some(target_index) = data.instances[instance_index].target_instance_index {
            if let Some(healing) = data.instances[target_index].healing_to_be_dealt {
                healing_text = healing.to_string();
            }
        } else
        if let Some(healing) = data.instances[instance_index].healing_to_be_dealt {
            healing_text = healing.to_string();
        }

        text = text.replace("${HEALING}", &healing_text);
    }*/

    // Formating if needed
    text = match message_type {
        MessageType::Say => format!("{} says \"{}\".", data.character_instances[data.curr_index].name, text),
        _ => text
    };

    let message_data = MessageData {
        message_type,
        message         : text,
        from            : data.character_instances[data.curr_index].name.clone(),
        right           : None,
        center          : None,
        buffer          : None
    };


    data.character_instances[data.curr_index].messages.push(message_data);

    BehaviorNodeConnector::Bottom
}

pub fn node_random_walk(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if wait_for(data.curr_index, id, data) {
        let mut p : Option<Position> = None;

        let mut distance = i32::MAX;

        if let Some(v) = &mut data.character_instances[data.curr_index].position {
            p = Some(v.clone());
        }

        let mut max_distance : i32 = 0;
        if let Some(d) = eval_script_integer(id, "max_distance", nodes) {
            max_distance = d;
        }

        let mut dp = data.sheets[data.curr_index].home_location.clone();

        if let Some(p) = &p {
            distance = compute_distance(p, &dp);
        }

        // If we are within the max distance, do a random walk, otherwise just go back towards the position
        if distance <= max_distance {
            if let Some(p) = &p {
                dp = p.clone();
            }

            let mut rng = thread_rng();
            let random = rng.gen_range(0..4);

            if random == 0 {
                dp.y -= 1;
            } else
            if random == 1 {
                dp.x += 1;
            } else
            if random == 2 {
                dp.y += 1;
            } else
            if random == 3 {
                dp.x -= 1;
            }
        }

        let mut delay : i32 = 0;
        if let Some(d) = eval_script_integer(id, "walk_delay", nodes) {
            delay = d.clamp(0, i32::MAX);
        }

        // Apply the delay
        data.character_instances[data.curr_index].sleep_cycles = delay as usize;

        _ = walk_towards(p, Some(dp),false, data);

        data.character_instances[data.curr_index].max_transition_time = delay as usize + 1;
        data.character_instances[data.curr_index].curr_transition_time = 1;

        let mut delay_between_movement : i32 = 10;
        if let Some(d) = eval_script_integer(id, "delay", nodes) {
            delay_between_movement = d;
        }

        wait_start(data.curr_index, delay_between_movement as usize, id, data);
    }

    BehaviorNodeConnector::Bottom
}

/// Pathfinder
pub fn node_pathfinder(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut p : Option<Position> = None;
    let mut dp : Option<Position> = None;

    let mut distance = i32::MAX;

    if let Some(v) = &mut data.character_instances[data.curr_index].position {
        p = Some(v.clone());
    }

    if let Some(value) = get_node_value2(id, "destination", nodes) {
        dp = match value {
            Value::Position(v) => {
                Some(v.clone())
            },
            _ => None
        };

        if let Some(dp) = &dp {
            if let Some(p) = &p {
                distance = compute_distance(p, dp);
            }
        }
    }

    let mut delay : i32 = 0;
    if let Some(d) = eval_script_integer(id, "walk_delay", nodes) {
        delay = d.clamp(0, i32::MAX);
    }

    // Apply the delay
    data.character_instances[data.curr_index].sleep_cycles = delay as usize;

    // Apply the speed delay
    data.character_instances[data.curr_index].sleep_cycles = delay as usize;

    // Success if we reached the to_distance already
    if distance == 0 {
        if let Some(dp) = &dp {
            // If we reached the target, set the home location to the target location
            // So that nodes like Random Walk can walk around this location
            data.sheets[data.curr_index].home_location = dp.clone();
        }
        return BehaviorNodeConnector::Success;
    }

    let rc  = walk_towards(p, dp,false, data);
    if  rc == BehaviorNodeConnector::Right {
        data.character_instances[data.curr_index].max_transition_time = delay as usize + 1;
        data.character_instances[data.curr_index].curr_transition_time = 1;
    }

    rc
}

/// Lookout
pub fn node_lookout(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut state = 0;
    if let Some(value) = get_node_integer(id, "state", nodes) {
        state = value;
    }

    let mut max_distance : i32 = 7;
    if let Some(d) = eval_script_integer(id, "max_distance", nodes) {
        max_distance = d;
    }

    // Find the chars within the given distance

    let mut chars : Vec<usize> = vec![];
    let index;

    {
        let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

        if let Some(position) = &data.character_instances[data.curr_index].position {
            for inst_index in 0..data.character_instances.len() {
                if inst_index != data.curr_index {
                    if (data.character_instances[inst_index].state == BehaviorInstanceState::Normal && state == 0) || (data.character_instances[inst_index].state == BehaviorInstanceState::Killed && state == 1) || (data.character_instances[inst_index].state == BehaviorInstanceState::Sleeping && state == 3) || (data.character_instances[inst_index].state == BehaviorInstanceState::Intoxicated && state == 4) {
                        if let Some(pos) = &data.character_instances[inst_index].position {
                            if pos.region == position.region {
                                let dx = position.x - pos.x;
                                let dy = position.y - pos.y;
                                let d = ((dx * dx + dy * dy) as f32).sqrt() as i32;
                                if d <= max_distance {
                                    chars.push(inst_index);
                                }
                            }
                        }
                    }
                }
            }
        }
        index = data.curr_index;
    }

    // Evaluate the expression on the characters who are in range
    for inst_ind in &chars {
        {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.curr_index = *inst_ind;
        }
        if eval_script_bool(id, "expression", nodes) {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.curr_index = index;
            data.character_instances[data.curr_index].target_instance_index = Some(*inst_ind);
            return BehaviorNodeConnector::Success;
        }
    }

    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    data.curr_index = index;
    data.character_instances[data.curr_index].target_instance_index = None;
    BehaviorNodeConnector::Fail
}

/// CloseIn
pub fn node_close_in(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut p : Option<Position> = None;
    let mut dp : Option<Position> = None;

    let mut distance = i32::MAX;
    let mut to_distance = 1;

    if let Some(d) = eval_script_integer(id, "to_distance", nodes) {
        to_distance = d;
    }

    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(v) = &mut data.character_instances[data.curr_index].position {
        p = Some(v.clone());
    }

    let target_index = data.character_instances[data.curr_index].target_instance_index;

    if let Some(target_index) = target_index {
        if let Some(v) = &mut data.character_instances[target_index].position {
            dp = Some(v.clone());
            if let Some(p) = &p {
                distance = compute_distance(p, v) as i32;
            }
        }
    }

    let mut delay : i32 = 2;
    if let Some(d) = eval_script_integer(id, "delay", nodes) {
        delay = d.clamp(0, i32::MAX);
    }

    // Apply the speed delay
    data.character_instances[data.curr_index].sleep_cycles = delay as usize;

    // Success if we reached the to_distance already
    if distance <= to_distance {
        return BehaviorNodeConnector::Success;
    }

    let rc = walk_towards(p, dp, true, data);
    if  rc == BehaviorNodeConnector::Right {
        data.character_instances[data.curr_index].max_transition_time = delay as usize + 1;
        data.character_instances[data.curr_index].curr_transition_time = 1;
    }
    rc
}

// Multi choice
pub fn node_multi_choice(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if data.character_instances[data.curr_index].multi_choice_answer.is_some() {
        if Some(id.1) == data.character_instances[data.curr_index].multi_choice_answer {

            let npc_index = data.character_instances[data.curr_index].communication[0].npc_index;
            drop_communication(data.curr_index, npc_index, data);
            data.character_instances[data.curr_index].target_instance_index = Some(npc_index);

            BehaviorNodeConnector::Bottom
        }
        else {
            BehaviorNodeConnector::Right
        }
    } else {

        // A new communication started

        if let Some(npc_index) = data.character_instances[data.curr_index].target_instance_index {

            let mut header = "".to_string();
            let mut text = "".to_string();
            let mut answer = "".to_string();

            if let Some(value) = get_node_string(id, "header", nodes) {
                header = value;
            }

            if let Some(value) = get_node_string(id, "text", nodes) {
                text = value;
            }

            if let Some(value) = get_node_string(id, "answer", nodes) {
                answer = value;
            }

            let mcd = MultiChoiceData {
                id              : id.1,
                header,
                text,
                answer,
                pos             : None,
                buffer          : None,

                item_amount     : None,
                item_behavior_id: None,
                item_price      : None,
            };

            let player_index = data.curr_index;
            data.character_instances[player_index].multi_choice_data.push(mcd);

            let com = PlayerCommunication {
                player_index,
                npc_index,
                npc_behavior_id         : id,
                player_answer           : None,
                start_time              : DATE.borrow().clone(),
                end_time                : DATE.borrow().future_time(10),
            };

            // Each NPC can only talk to one player at the same time
            if data.character_instances[npc_index].communication.is_empty() && data.character_instances[player_index].communication.is_empty(){
                data.character_instances[npc_index].communication.push(com.clone());
                data.character_instances[player_index].communication.push(com);
            }
        }

        BehaviorNodeConnector::Right
    }
}

// Sell
pub fn node_sell(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if data.character_instances[data.curr_index].multi_choice_answer.is_some() {
        if let Some(id) = data.character_instances[data.curr_index].multi_choice_answer {

            let npc_index = data.character_instances[data.curr_index].communication[0].npc_index;
            let mut traded_item : Option<Item> = None;

            // Remove the item

            if let Some(item) = data.sheets[npc_index].inventory.remove_item(id, 1) {
                traded_item = Some(item);
            }

            let mut rc = BehaviorNodeConnector::Success;

            // Add the item to the player
            if let Some(item) = traded_item {
                let price = item.value;

                let sheet = &mut data.sheets[data.curr_index];

                if sheet.can_afford(price) {
                    sheet.wealth.remove(price);
                    sheet.inventory.add_item(item);
                    let npc_sheet = &mut data.sheets[npc_index];
                    npc_sheet.wealth.add(price);
                } else {
                    // Not enough money add back t the NPC
                    data.sheets[npc_index].inventory.add_item(item);
                    rc = BehaviorNodeConnector::Fail;
                }
            } else {
                // If the item was no longer available, just quit
                rc = BehaviorNodeConnector::Bottom;
            }

            drop_communication(data.curr_index, npc_index, data);
            rc
        }
        else {
           BehaviorNodeConnector::Right
        }
    } else {

        let mut header = "".to_string();
        let mut exit = "Exit".to_string();

        if let Some(value) = get_node_string(id, "header", nodes) {
            header = value;
        }

        if let Some(value) = get_node_string(id, "exit", nodes) {
            exit = value;
        }

        if let Some(npc_index) = data.character_instances[data.curr_index].target_instance_index {
            data.character_instances[data.curr_index].multi_choice_data = vec![];

            let mut index = 1;
            let mut added_items = vec![];

            for item in & data.sheets[npc_index].inventory.items {
                if item.value.absolute() > 0 && added_items.contains(&item.id) == false {
                    let amount = 1;

                    let mcd = MultiChoiceData {
                        id                  : item.id,
                        header              : if index == 1 { header.clone() } else { "".to_string() },
                        text                : item.name.clone(),
                        answer              : index.to_string(),
                        pos                 : None,
                        buffer              : None,

                        item_behavior_id    : Some(item.id),
                        item_price          : Some(item.value),
                        item_amount         : Some(amount),
                    };

                    added_items.push(item.id);
                    data.character_instances[data.curr_index].multi_choice_data.push(mcd);
                    index += 1;
                }
            }

            let player_index = data.curr_index;
            if data.character_instances[player_index].multi_choice_data.is_empty() == false {

                // Exit Text

                let mcd = MultiChoiceData {
                    id                  : Uuid::new_v4(),
                    header              : "".to_string(),
                    text                : exit,
                    answer              : "0".to_string(),
                    pos                 : None,
                    buffer              : None,

                    item_behavior_id    : None,
                    item_price          : None,
                    item_amount         : None,
                };
                data.character_instances[player_index].multi_choice_data.push(mcd);

                //

                let com = PlayerCommunication {
                    player_index,
                    npc_index,
                    npc_behavior_id         : id,
                    player_answer           : None,
                    start_time              : DATE.borrow().clone(),
                    end_time                : DATE.borrow().future_time(10),
                };

                if data.character_instances[npc_index].communication.is_empty() && data.character_instances[player_index].communication.is_empty() {
                    data.character_instances[npc_index].communication.push(com.clone());
                    data.character_instances[player_index].communication.push(com);
                }
            }
        }

        BehaviorNodeConnector::Right
    }
}

/// Systems Call
pub fn node_call_system(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut systems_id : Option<Uuid> = None;

    if let Some(system_name) = get_node_string(id, "system", nodes) {
        for (id, data) in SYSTEMS.borrow().iter() {
            if data.name == system_name {
                systems_id = Some(*id);
                break
            }
        }
    }

    let mut behavior_tree_id : Option<Uuid> = None;
    if let Some(systems_id) = systems_id {
        if let Some(tree_name) = get_node_string(id, "tree", nodes) {
            // Get the behavior this node chain is running on
            if let Some(behavior) = SYSTEMS.borrow().get(&systems_id) {
                for (node_id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                        behavior_tree_id = Some(*node_id);
                        break;
                    }
                }
            }
        }
    }

    if let Some(systems_id) = systems_id {
        if let Some(behavior_tree_id) = behavior_tree_id {
            execute_node(systems_id, behavior_tree_id, &mut SYSTEMS.borrow_mut());
            return BehaviorNodeConnector::Success;
        }
    }

    BehaviorNodeConnector::Fail
}

/// Behavior Call
pub fn node_call_behavior(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut behavior_id : Uuid = Uuid::new_v4();
    let mut behavior_tree_id : Option<Uuid> = None;
    if let Some(tree_name) = get_node_string(id, "tree", nodes) {
        // Get the behavior this node chain is running on
        if let Some(behavior) = nodes.get(&id.0) {
            behavior_id = behavior.id;
            for (node_id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                    behavior_tree_id = Some(*node_id);
                    break;
                }
            }
        }
    }

    // println!("behavior instance {:?}", behavior_instance);
    // println!("behavior_tree_id id {:?}", behavior_tree_id);

    if let Some(behavior_tree_id) = behavior_tree_id {
        execute_node(behavior_id, behavior_tree_id, nodes);
        return BehaviorNodeConnector::Success;
    }

    /*
    if let Some(tree_name) = get_node_string(id, "tree", nodes) {
        let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        println!("call {}", data.curr_index);
        data.to_execute.push((data.curr_index, tree_name));
        return BehaviorNodeConnector::Success;
    }*/

    BehaviorNodeConnector::Fail
}

/// Lock Tree
pub fn node_lock_tree(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let behavior_instance : Option<usize> = Some(data.curr_index);
    let mut behavior_tree_id : Option<Uuid> = None;
    let is_target = false;

    if let Some(value) = get_node_string(id, "tree", nodes) {
        if let Some(behavior_instance) = behavior_instance {
            if let Some(behavior) = BEHAVIORS.borrow().get(&data.character_instances[behavior_instance].behavior_id) {
                for (node_id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == value {
                        behavior_tree_id = Some(*node_id);
                        break;
                    }
                }
            }
        }
    }

    // println!("behavior instance {:?}", behavior_instance);
    // println!("behavior_tree_id id {:?}", behavior_tree_id);

    if let Some(behavior_instance) = behavior_instance {
        if let Some(behavior_tree_id) = behavior_tree_id {
            // Lock the tree
            data.character_instances[behavior_instance].locked_tree = Some(behavior_tree_id);
            if is_target {
                // If we call lock on a target, we target ourself for the target
                data.character_instances[behavior_instance].target_instance_index = Some(data.curr_index);
            }
            return BehaviorNodeConnector::Success;
        }
    }
    BehaviorNodeConnector::Fail
}

/// Unlock Tree
pub fn node_unlock_tree(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let behavior_instance : Option<usize> = Some(data.curr_index);

    if let Some(behavior_instance) = behavior_instance {
        // Unlock the tree
        data.character_instances[behavior_instance].locked_tree = None;
        data.character_instances[behavior_instance].target_instance_index = None;
    }
    BehaviorNodeConnector::Bottom
}

/// Query State
pub fn node_query_state(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut state = 0;
    if let Some(value) = get_node_integer(id, "state", nodes) {
        state = value;
    }

    if (data.character_instances[data.curr_index].state == BehaviorInstanceState::Normal && state == 0) || (data.character_instances[data.curr_index].state == BehaviorInstanceState::Killed && state == 1) || (data.character_instances[data.curr_index].state == BehaviorInstanceState::Sleeping && state == 3) || (data.character_instances[data.curr_index].state == BehaviorInstanceState::Intoxicated && state == 4) {
        BehaviorNodeConnector::Success
    } else {
        BehaviorNodeConnector::Fail
    }
}

/// Set State
pub fn node_set_state(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let behavior_instance : Option<usize> = Some(data.curr_index);

    if let Some(value) = get_node_integer(id, "state", nodes) {
        if let Some(behavior_instance) = behavior_instance {
            //println!("behavior instance {:?}", behavior_instance);
            data.character_instances[behavior_instance].state = match value {
                1 => BehaviorInstanceState::Killed,
                2 => BehaviorInstanceState::Purged,
                3 => BehaviorInstanceState::Sleeping,
                4 => BehaviorInstanceState::Intoxicated,

                _ => BehaviorInstanceState::Normal,
            };


            // If target is dead, clean this instance from all targets
            if data.character_instances[behavior_instance].state.is_dead() {
                for i in 0..data.character_instances.len() {
                    if data.character_instances[i].target_instance_index == Some(behavior_instance) {
                        data.character_instances[i].locked_tree = None;
                    }
                }
            }
        }
    }

    BehaviorNodeConnector::Bottom
}

/// Has Target ?
pub fn node_has_target(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
    if data.character_instances[data.curr_index].target_instance_index.is_some() {
        BehaviorNodeConnector::Success
    } else {
        BehaviorNodeConnector::Fail
    }
}

/// Untarget (based on distance)
pub fn node_untarget(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if data.character_instances[data.curr_index].target_instance_index.is_some() {
        let mut distance : i32 = 0;
        if let Some(d) = eval_script_integer(id, "distance", nodes) {
            distance = d;
        }

        if let Some(p1) = data.get_instance_position(data.curr_index) {
            if let Some(p2) = data.get_instance_position(data.character_instances[data.curr_index].target_instance_index.unwrap()) {
                let d = compute_distance(&p1, &p2) as i32;
                if d > distance {
                    data.character_instances[data.curr_index].target_instance_index = None;
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }
    BehaviorNodeConnector::Fail
}
/*
/// Deal damage :)
pub fn deal_damage(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut damage : i32 = 0;
    let mut speed : f32 = 4.0;

    let mut attack_rating = 0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "attack_rating".to_string()), data) {
        attack_rating = rc as i32;
    }

    if let Some(id) = get_weapon_script_id(instance_index, "main hand".to_string(), data) {
        if let Some(rc) = eval_number_expression_instance(instance_index, id.clone(), data) {
            damage = rc as i32;
            // println!("[{}] damage {}", data.instances[instance_index].name, damage);
        }
        let mut id_speed = id.clone();
        id_speed.3 = "speed".to_string();
        if let Some(rc) = eval_number_expression_instance(instance_index, id_speed, data) {
            // println!("[{}] speed {}", data.instances[instance_index].name, rc);
            speed = rc;
        }
    }

    // Apply the speed delay
    let delay = speed.clamp(0.0, f32::MAX);

    let mut rc = BehaviorNodeConnector::Fail;

    if data.instances[instance_index].target_instance_index.is_some() {
        let target_index = data.instances[instance_index].target_instance_index.unwrap();
        data.instances[target_index].damage_to_be_dealt = Some(damage);

        let mut behavior_tree_id : Option<Uuid> = None;

        let tree_name = "onHit";
        if let Some(behavior) = data.behaviors.get(&data.instances[target_index].behavior_id) {
            for (node_id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                    behavior_tree_id = Some(*node_id);
                    break;
                }
            }
        }

        data.instances[instance_index].sleep_cycles = delay as usize;

        if let Some(behavior_tree_id) = behavior_tree_id {
            data.action_subject_text = data.instances[instance_index].name.clone();
            data.scopes[target_index].set_value("_attack_rating", attack_rating);
            let _rc = data.execute_node(target_index, behavior_tree_id, None);
            if data.dealt_damage_success {
                increase_weapon_skill_value(instance_index, "main hand".to_string(), data);
            }
            if data.instances[target_index].state == BehaviorInstanceState::Normal {
                rc = BehaviorNodeConnector::Right;
            } else {
                rc = BehaviorNodeConnector::Success;
            }
        }
    }

    // We killed the opponent, we call the LevelTree node to add experience
    if rc == BehaviorNodeConnector::Success {

        let mut system_name : Option<String> = None;
        let mut tree_name : Option<String> = None;

        if let Some(e) = data.scopes[instance_index].get_mut("experience") {
            if let Some(exp) = e.read_lock::<Experience>() {
                system_name = exp.system_name.clone();
                tree_name = exp.tree_name.clone();
            }
        }

        if let Some(system_name) = system_name {
            if let Some(tree_name) = tree_name {

                let mut exp_to_add = 0;

                if let Some(index) = data.system_names.iter().position(|r| *r == system_name) {
                    if let Some(behavior) = data.systems.get(&data.system_ids[index]) {
                        for (node_id, node) in &behavior.nodes {
                            if node.name == tree_name {

                                if let Some(value) = eval_number_expression_instance(instance_index, (BehaviorType::Systems, behavior.id, *node_id, "experience_kill".to_string()), data) {
                                    exp_to_add = value as i32;
                                }

                                break;
                            }
                        }
                    }
                }

                if exp_to_add > 0 {


                    let mut script_id = (BehaviorType::Systems, Uuid::nil(), Uuid::nil(), "".to_string());

                    // Add the experience
                    if let Some(e) = data.scopes[instance_index].get_mut("experience") {
                        if let Some(mut exp) = e.write_lock::<Experience>() {

                            exp.experience += exp_to_add;

                            let mut str = exp.experience_msg.clone();
                            str = str.replace("{}", &exp_to_add.to_string());

                            // Send message
                            let message_data = MessageData {
                                message_type    : MessageType::Status,
                                message         : str,
                                from            : "System".to_string(),
                                right           : None,
                                center          : None,
                                buffer          : None
                            };

                            data.instances[instance_index].messages.push(message_data.clone());

                            let mut new_level = 0;
                            for lvl in 0..exp.levels.len() {
                                if exp.experience >= exp.levels[lvl].0 {
                                    new_level = lvl as i32 + 1;

                                    // Send message
                                    let message_data = MessageData {
                                        message_type    : MessageType::Status,
                                        message         : exp.levels[lvl].1.clone(),
                                        from            : "System".to_string(),
                                        right           : None,
                                        center          : None,
                                        buffer          : None
                                    };

                                    data.instances[instance_index].messages.push(message_data.clone());
                                } else {
                                    break;
                                }
                            }
                            if new_level > exp.level {
                                exp.level = new_level;

                                script_id = (BehaviorType::Systems, exp.level_tree_id, exp.levels[new_level as usize-1].2, "script".to_string());

                                println!("[{}] Advanced to level {}", data.instances[instance_index].name, exp.level);
                            }
                        }
                    }

                    // Execute level script
                    if script_id.3.is_empty() == false {
                        //println!("Execute level script: {:?}", script_id);
                        let _rc = eval_dynamic_script_instance(instance_index, script_id, data);
                        // println!("Execute level script: {:?}", rc);
                    }
                }
            }
        }
    }

    rc
}

/// Take damage :(
pub fn take_damage(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut reduce_by : i32 = 0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "reduce by".to_string()), data) {
        reduce_by = rc as i32;
    }

    let mut rc = BehaviorNodeConnector::Success;
    data.dealt_damage_success = false;

    if let Some(mut damage) = data.instances[instance_index].damage_to_be_dealt {
        damage -= reduce_by;

        if let Some(mut value) = data.scopes[instance_index].get_value::<i32>(&data.hitpoints) {
            value -= damage;

            if value > 0 {
                data.dealt_damage_success = true;
            }

            data.instances[instance_index].damage_to_be_dealt = Some(damage);
            value = value.max(0);
            data.scopes[instance_index].set_value(&data.hitpoints, value);
            if value <= 0 {
                rc = BehaviorNodeConnector::Fail;
            }
        } else
        if let Some(v) = data.scopes[instance_index].get_value::<f32>(&data.hitpoints) {
            let mut value = v as i32;
            value -= damage;

            if value > 0 {
                data.dealt_damage_success = true;
            }

            data.instances[instance_index].damage_to_be_dealt = Some(damage);
            value = value.max(0);
            data.scopes[instance_index].set_value(&data.hitpoints, value);
            if value <= 0 {
                rc = BehaviorNodeConnector::Fail;
            }
        }
    }
    rc
}*/

/// Assign target for magic
pub fn node_magic_target(_id: (Uuid, Uuid), _nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if data.character_instances[data.curr_index].instance_type == BehaviorInstanceType::NonPlayerCharacter {
        return BehaviorNodeConnector::Success;
    }

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
            // Make sure the target is within spell range
            let spell_name = data.character_instances[data.curr_index].action.clone().unwrap().spell.unwrap();
            let distance = compute_distance(&position, &dp);
            let spell_distance = get_spell_distance(spell_name, data);
            if  distance as i32 <= spell_distance {
                for inst_index in 0..data.character_instances.len() {
                    if inst_index != data.curr_index {
                        // Only track if the state is OK
                        if data.character_instances[inst_index].state.is_alive() {
                            if let Some(pos) = &data.character_instances[inst_index].position {
                                if *dp == *pos {
                                    data.character_instances[data.curr_index].target_instance_index = Some(inst_index);
                                    rc = BehaviorNodeConnector::Success;
                                    break;
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

/*
/// Deal magic damage
pub fn magic_damage(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let speed : f32 = 4.0;

    let mut damage = 0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "damage".to_string()), data) {
        damage = rc as i32;
    }

    // Apply the speed delay
    let delay = speed.clamp(0.0, f32::MAX);

    let mut rc = BehaviorNodeConnector::Fail;

    if data.instances[instance_index].target_instance_index.is_some() {
        let target_index = data.instances[instance_index].target_instance_index.unwrap();
        data.instances[target_index].damage_to_be_dealt = Some(damage);

        let mut behavior_tree_id : Option<Uuid> = None;

        let tree_name = "onHit";
        if let Some(behavior) = data.behaviors.get(&data.instances[target_index].behavior_id) {
            for (node_id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                    behavior_tree_id = Some(*node_id);
                    break;
                }
            }
        }

        data.instances[instance_index].sleep_cycles = delay as usize;

        if let Some(behavior_tree_id) = behavior_tree_id {
            data.action_subject_text = data.instances[instance_index].name.clone();
            data.scopes[target_index].set_value("_attack_rating", 1);
            let _rc = data.execute_node(target_index, behavior_tree_id, None);
            if data.instances[target_index].state == BehaviorInstanceState::Normal {
                rc = BehaviorNodeConnector::Right;
            } else {
                rc = BehaviorNodeConnector::Success;
            }
        }
    }

    // We killed the opponent, we call the LevelTree node to add experience
    if rc == BehaviorNodeConnector::Success {

        let mut system_name : Option<String> = None;
        let mut tree_name : Option<String> = None;

        if let Some(e) = data.scopes[instance_index].get_mut("experience") {
            if let Some(exp) = e.read_lock::<Experience>() {
                system_name = exp.system_name.clone();
                tree_name = exp.tree_name.clone();
            }
        }

        if let Some(system_name) = system_name {
            if let Some(tree_name) = tree_name {

                let mut exp_to_add = 0;

                if let Some(index) = data.system_names.iter().position(|r| *r == system_name) {
                    if let Some(behavior) = data.systems.get(&data.system_ids[index]) {
                        for (node_id, node) in &behavior.nodes {
                            if node.name == tree_name {

                                if let Some(value) = eval_number_expression_instance(instance_index, (BehaviorType::Systems, behavior.id, *node_id, "experience_kill".to_string()), data) {
                                    exp_to_add = value as i32;
                                }

                                break;
                            }
                        }
                    }
                }

                if exp_to_add > 0 {


                    let mut script_id = (BehaviorType::Systems, Uuid::nil(), Uuid::nil(), "".to_string());

                    // Add the experience
                    if let Some(e) = data.scopes[instance_index].get_mut("experience") {
                        if let Some(mut exp) = e.write_lock::<Experience>() {

                            exp.experience += exp_to_add;

                            let mut str = exp.experience_msg.clone();
                            str = str.replace("{}", &exp_to_add.to_string());

                            // Send message
                            let message_data = MessageData {
                                message_type    : MessageType::Status,
                                message         : str,
                                from            : "System".to_string(),
                                right           : None,
                                center          : None,
                                buffer          : None
                            };

                            data.instances[instance_index].messages.push(message_data.clone());

                            let mut new_level = 0;
                            for lvl in 0..exp.levels.len() {
                                if exp.experience >= exp.levels[lvl].0 {
                                    new_level = lvl as i32 + 1;

                                    // Send message
                                    let message_data = MessageData {
                                        message_type    : MessageType::Status,
                                        message         : exp.levels[lvl].1.clone(),
                                        from            : "System".to_string(),
                                        right           : None,
                                        center          : None,
                                        buffer          : None
                                    };

                                    data.instances[instance_index].messages.push(message_data.clone());
                                } else {
                                    break;
                                }
                            }
                            if new_level > exp.level {
                                exp.level = new_level;

                                script_id = (BehaviorType::Systems, exp.level_tree_id, exp.levels[new_level as usize-1].2, "script".to_string());

                                println!("[{}] Advanced to level {}", data.instances[instance_index].name, exp.level);
                            }
                        }
                    }

                    // Execute level script
                    if script_id.3.is_empty() == false {
                        //println!("Execute level script: {:?}", script_id);
                        let _rc = eval_dynamic_script_instance(instance_index, script_id, data);
                        // println!("Execute level script: {:?}", rc);
                    }
                }
            }
        }
    }

    rc
}*/

/// Drop Inventory :(
pub fn node_drop_inventory(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let mut drop_type = 0;

    if let Some(value) = get_node_integer(id, "drop", nodes) {
        drop_type = value;
    }

    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let sheet: &mut Sheet = &mut data.sheets[data.curr_index];

    let total = sheet.inventory.items.len();

    let mut drop = |index: usize| {
        if index < sheet.inventory.items.len() {
            let mut item = sheet.inventory.items[index].clone();
            sheet.inventory.items.remove(index);

            if let Some(p) = &data.character_instances[data.curr_index].position {
                if let Some(mut light) = item.light.clone() {
                    light.position = (p.x, p.y);
                    item.light = Some(light);
                }

                if let Some(existing_loot) = data.loot.get_mut(&(p.x, p.y)) {
                    existing_loot.push(item);
                } else {
                    data.loot.insert((p.x, p.y), vec![item]);
                }
            }
        }
    };

    if drop_type == 0 {
        // Drop items
        for _ in 0..total {
            drop(0);
        }
    } else {
        let mut rng = thread_rng();
        let random = rng.gen_range(0..total);
        drop(random);
    }

    // Drop gold

    if sheet.wealth.gold > 0 {
        for (id, behavior) in ITEMS.borrow().iter() {
            if behavior.name.to_lowercase() == "gold" {
                let mut loot = Item::new(*id, behavior.name.clone());
                loot.amount = sheet.wealth.gold;

                sheet.wealth.gold = 0;

                for (_index, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorType {
                        if let Some(value) = node.values.get(&"tile".to_string()) {
                            loot.tile = value.to_tile_data();
                        }
                    }
                }

                if let Some(p) = &data.character_instances[data.curr_index].position {
                    if let Some(existing_loot) = data.loot.get_mut(&(p.x, p.y)) {
                        existing_loot.push(loot);
                    } else {
                        data.loot.insert((p.x, p.y), vec![loot]);
                    }
                }
            }
        }
    }

    BehaviorNodeConnector::Bottom
}

/// Teleport
pub fn node_teleport(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(value) = get_node_value2(id, "position", nodes) {
        match &value {
            Value::Position(position) => {
                data.character_instances[data.curr_index].position = Some(position.clone());
            }
            _ => {},
        }
        data.character_instances[data.curr_index].old_position = None;
        data.character_instances[data.curr_index].max_transition_time = 0;
        data.character_instances[data.curr_index].curr_transition_time = 0;
    }
    BehaviorNodeConnector::Bottom
}

/// Play effect for the character
pub fn node_effect(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut index = data.curr_index;

    if let Some(value) = get_node_integer(id, "for", nodes) {
        if value == 1 {
            if let Some(target) = data.character_instances[data.curr_index].target_instance_index {
                index = target;
            }
        }
    }

    if let Some(value) = get_node_value2(id, "effect", nodes) {
        if let Some(tile) = value.to_tile_id() {
            data.character_instances[index].effects.push(tile);
        }
    }
    BehaviorNodeConnector::Bottom
}

/// Play audio
pub fn node_audio(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    if let Some(audio_name) = get_node_string(id, "audio", nodes) {
        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.character_instances[data.curr_index].audio.push(audio_name.clone());
    }
    BehaviorNodeConnector::Bottom
}

/*
/// Heal, opposite of deal damage
pub fn heal(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut behavior_instance : Option<usize> = None;

    // The id's were not yet computed search the system trees, get the ids and store them.
    if let Some(v) = get_node_value((id.0, id.1, "for"), data, behavior_type) {
        if let Some(value) = v.to_integer() {
            if value == 0 {
                // Run the behavior on myself
                behavior_instance = Some(get_local_instance_index(instance_index, data));
            } else {
                // Run the behavior on the target
                if let Some(target_index) = data.instances[instance_index].target_instance_index {
                    behavior_instance = Some(target_index);
                }
            }
        }
    }

    let mut amount : i32 = 0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "amount".to_string()), data) {
        amount = rc as i32;
    }

    let mut speed : f32 = 8.0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "speed".to_string()), data) {
        speed = rc;
    }

    // Apply the speed delay
    let delay = speed.clamp(0.0, f32::MAX);

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(target_index) = behavior_instance {
        data.instances[target_index].healing_to_be_dealt = Some(amount);

        let mut behavior_tree_id : Option<Uuid> = None;

        let tree_name = "onHeal";
        if let Some(behavior) = data.behaviors.get(&data.instances[target_index].behavior_id) {
            for (node_id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                    behavior_tree_id = Some(*node_id);
                    break;
                }
            }
        }

        data.instances[instance_index].sleep_cycles = delay as usize;

        if let Some(behavior_tree_id) = behavior_tree_id {
            data.action_subject_text = data.instances[instance_index].name.clone();
            let _rc = data.execute_node(target_index, behavior_tree_id, None);
            if data.instances[target_index].state == BehaviorInstanceState::Normal {
                rc = BehaviorNodeConnector::Right;
            } else {
                rc = BehaviorNodeConnector::Success;
            }
        }
    }
    rc
}

/// Take Heal :)
pub fn take_heal(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    let mut increase_by : i32 = 0;
    if let Some(rc) = eval_number_expression_instance(instance_index, (behavior_type, id.0, id.1, "increase by".to_string()), data) {
        increase_by = rc as i32;
    }

    let mut rc = BehaviorNodeConnector::Success;

    if let Some(mut healing) = data.instances[instance_index].healing_to_be_dealt {
        healing += increase_by;

        if let Some(mut value) = data.scopes[instance_index].get_value::<i32>(&data.hitpoints) {
            value += healing;
            data.instances[instance_index].healing_to_be_dealt = Some(healing);
            value = value.max(0);

            // Compare to max hitpoints
            if let Some(max) = data.scopes[instance_index].get_value::<i32>(&data.max_hitpoints) {
                if value > max {
                    value = max;
                }
            }

            data.scopes[instance_index].set_value(&data.hitpoints, value);
            if value <= 0 {
                rc = BehaviorNodeConnector::Fail;
            }
        } else
        if let Some(v) = data.scopes[instance_index].get_value::<f32>(&data.hitpoints) {
            let mut value = v as i32;
            value += healing;
            data.instances[instance_index].healing_to_be_dealt = Some(healing);
            value = value.max(0);

            // Compare to max hitpoints
            if let Some(max) = data.scopes[instance_index].get_value::<f32>(&data.max_hitpoints) {
                if value > max as i32 {
                    value = max as i32;
                }
            }

            data.scopes[instance_index].set_value(&data.hitpoints, value);
            if value <= 0 {
                rc = BehaviorNodeConnector::Fail;
            }
        }
    }
    rc
}*/

pub fn node_respawn(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let mut ticks : i32 = 0;
    if let Some(rc) = eval_script_integer(id, "minutes", nodes) {
        ticks = rc;
    }
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    let mut respawn_tick = *TICK_COUNT.borrow_mut() as usize;
    respawn_tick = respawn_tick.wrapping_add(ticks as usize * data.ticks_per_minute);
    if let Some(d) = &data.character_instances[data.curr_index].instance_creation_data {
        data.respawn_instance.insert(data.character_instances[data.curr_index].behavior_id, (respawn_tick, d.clone()));
    }

    BehaviorNodeConnector::Right
}

/// Set Level Tree
pub fn node_set_level_tree(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    let mut system_name : Option<String> = None;
    let mut tree_name : Option<String> = None;

    // Get the system name
    if let Some(value) = get_node_value2(id, "system", nodes) {
        if let Some(sys_name) = value.to_string() {
            system_name = Some(sys_name);
        }
    }

    if let Some(value) = get_node_value2(id, "tree", nodes) {
        if let Some(str) = value.to_string() {
            tree_name = Some(str);
        }
    }

    let mut levels : Vec<(i32, String, Uuid)> = vec![];
    let mut level_behavior_id = Uuid::new_v4();
    let mut experience_msg : String = "You gained {} experience.".to_string();

    if let Some(system_name) = system_name {
        if let Some(tree_name) = tree_name {
            for (id, behavior) in SYSTEMS.borrow().iter() {
                if behavior.name == system_name {
                    for (_id, node) in &behavior.nodes {
                        if node.name == tree_name {

                            if let Some(value) = node.values.get(&"message".to_string()) {
                                if let Some(m) = value.to_string() {
                                    experience_msg = m;
                                }
                            }
                            // Store the levels

                            let mut rc : Vec<(i32, String, Uuid)> = vec![];
                            let mut parent_id = node.id;

                            level_behavior_id = *id;

                            loop {
                                let mut found = false;
                                for (id1, c1, id2, c2) in &behavior.connections {
                                    if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                        for (uuid, node) in &behavior.nodes {
                                            if *uuid == *id2 {
                                                let mut start = 0;
                                                if let Some(value) = node.values.get(&"start".to_string()) {
                                                    if let Some(i) = value.to_integer() {
                                                        start = i;
                                                    }
                                                }
                                                let mut message = "".to_string();
                                                if let Some(value) = node.values.get(&"message".to_string()) {
                                                    if let Some(m) = value.to_string() {
                                                        message = m;
                                                    }
                                                }

                                                parent_id = node.id;
                                                found = true;

                                                rc.push((start, message, parent_id));
                                            }
                                        }
                                    } else
                                    if *id2 == parent_id && *c2 == BehaviorNodeConnector::Bottom {
                                        for (uuid, node) in &behavior.nodes {
                                            if *uuid == *id1 {
                                                let mut start = 0;
                                                if let Some(value) = node.values.get(&"start".to_string()) {
                                                    if let Some(i) = value.to_integer() {
                                                        start = i;
                                                    }
                                                }
                                                let mut message = "".to_string();
                                                if let Some(value) = node.values.get(&"message".to_string()) {
                                                    if let Some(m) = value.to_string() {
                                                        message = m;
                                                    }
                                                }
                                                parent_id = node.id;
                                                found = true;

                                                rc.push((start, message, parent_id));
                                            }
                                        }
                                    }
                                }
                                if found == false {
                                    break;
                                }
                            }

                            levels = rc;
                        }
                    }
                }
            }

            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            let sheet = &mut data.sheets[data.curr_index];
            sheet.experience.system_name = Some(system_name);
            sheet.experience.tree_name = Some(tree_name);
            sheet.experience.levels = levels;
            sheet.experience.experience_msg = experience_msg;
            sheet.experience.level_behavior_id = level_behavior_id;
        }
    }

    BehaviorNodeConnector::Bottom
}

/// Schedule
pub fn node_schedule(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let date = DATE.borrow().clone();
    // Get the system name
    if let Some(from) = get_node_value2(id, "from", nodes) {
        if let Some(to) = get_node_value2(id, "to", nodes) {
            if let Some(f) = from.to_date() {
                if let Some(t) = to.to_date() {
                    if date >= f && date <= t {
                        return BehaviorNodeConnector::Right;
                    }
                }
            }

        }
    }

    BehaviorNodeConnector::Bottom
}