extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;
use crate::prelude::*;
use pathfinding::prelude::bfs;

/// Returns an integer value for the given node.
pub fn get_node_value2(id: (Uuid, Uuid), value_name: &str, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> Option<Value> {
    if let Some(item) = nodes.get_mut(&id.0) {
        if let Some(node) = item.nodes.get_mut(&id.1) {
            for (name, value) in &node.values {
                if *name == value_name {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}

/// Returns an integer value for the given node.
pub fn get_node_integer(id: (Uuid, Uuid), value_name: &str, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> Option<i32> {
    if let Some(item) = nodes.get_mut(&id.0) {
        if let Some(node) = item.nodes.get_mut(&id.1) {
            for (name, value) in &node.values {
                if *name == value_name {
                    if let Some(int) = value.to_integer() {
                        return Some(int);
                    }
                    break;
                }
            }
        }
    }
    None
}

/// Returns an integer value for the given node.
pub fn get_node_string(id: (Uuid, Uuid), value_name: &str, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> Option<String> {
    if let Some(item) = nodes.get_mut(&id.0) {
        if let Some(node) = item.nodes.get_mut(&id.1) {
            for (name, value) in &node.values {
                if *name == value_name {
                    if let Some(v) = value.to_string() {
                        return Some(v);
                    }
                    break;
                }
            }
        }
    }
    None
}

/// Retrieves a number instance value
pub fn get_number_variable(instance_index: usize, variable: String, data: &mut RegionInstance) -> Option<f32> {
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return Some(value);
    }
    None
}

/// Retrieves a number instance value or 0
pub fn get_number_variable_or_zero(instance_index: usize, variable: String, data: &RegionInstance) -> f32 {
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return value;
    }
    0.0
}

/// Retrieves an i32 variable
pub fn get_i32_variable(instance_index: usize, variable: String, data: &mut RegionInstance) -> Option<i32> {
    if let Some(value) = data.scopes[instance_index].get_value::<i32>(&variable) {
        return Some(value);
    }
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return Some(value as i32);
    }
    None
}

/// Retrieves an i32 variable
pub fn get_f32_variable(instance_index: usize, variable: String, data: &mut RegionInstance) -> Option<f32> {
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return Some(value);
    }
    if let Some(value) = data.scopes[instance_index].get_value::<i32>(&variable) {
        return Some(value as f32);
    }
    None
}

/// Sets a number instance value
pub fn set_number_variable(instance_index: usize, variable: String, value: f32, data: &mut RegionInstance) {
    data.scopes[instance_index].set_value(&variable, value);
}

/// Retrieves a node value
pub fn get_node_value(id: (Uuid, Uuid, &str), data: &mut RegionInstance, behavior_type: BehaviorType) -> Option<Value> {
    if behavior_type == BehaviorType::Regions {

        for behavior in &data.region_behavior {
            //if behavior.id == id.0 {
                if let Some(node) = behavior.nodes.get(&id.1) {
                    if let Some(value) = node.values.get(id.2) {
                        return Some(value.clone());
                    }
                }
            //}
        }
    } else
    if behavior_type == BehaviorType::Behaviors {
        if let Some(behavior) = data.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::Systems {
        if let Some(system) = data.systems.get_mut(&id.0) {
            if let Some(node) = system.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::Items {
        if let Some(item) = data.items.get_mut(&id.0) {
            if let Some(node) = item.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::Spells {
        if let Some(item) = data.spells.get_mut(&id.0) {
            if let Some(node) = item.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::GameLogic {
        let game = &mut data.game_data;
        if let Some(node) = game.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut(id.2) {
                return Some(value.clone());
            }
        }
    }
    None
}

/// Computes the distance between two locations
pub fn compute_distance(p0: &Position, p1: &Position) -> i32 {
    let dx = p0.x - p1.x;
    let dy = p0.y - p1.y;
    ((dx * dx + dy * dy) as f32).sqrt().floor() as i32
}

/// Returns the current position of the instance_index, takes into account an ongoing animation
pub fn get_instance_position(inst_index: usize, instances: &Vec<BehaviorInstance>) -> Option<Position> {
    if let Some(old_position) = &instances[inst_index].old_position {
        return Some(old_position.clone());
    }
    instances[inst_index].position.clone()
}

/// Walk towards a destination position
pub fn walk_towards(p: Option<Position>, dp: Option<Position>, exclude_dp: bool, data: &mut RegionData) -> BehaviorNodeConnector {

    // Cache the character positions
    let mut char_positions : Vec<Position> = vec![];

    if let Some(p) = &p {
        for inst_index in 0..data.character_instances.len() {
            if inst_index != data.curr_index {
                // Only track if the state is normal
                if data.character_instances[data.curr_index].state == BehaviorInstanceState::Normal {
                    if let Some(pos) = &data.character_instances[data.curr_index].position {
                        if p.region == pos.region {
                            if exclude_dp == false {
                                char_positions.push(pos.clone());
                            } else {
                                // Exclude dp, otherwise the Close In tracking function does not find a route
                                if let Some(dp) = &dp {
                                    if *dp != *pos {
                                        char_positions.push(pos.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(p) = &p {

        let can_go = |x: isize, y: isize| -> bool {

            // Check tiles
            let tiles = data.get_tile_at((x, y));
            if tiles.is_empty() { return false; }
            for tile in tiles {
                if tile.usage == TileUsage::EnvBlocking || tile.usage == TileUsage::Water {
                    return false;
                }
            }

            // Check characters
            for char_p in &char_positions {
                if char_p.x == x && char_p.y == y {
                    return false;
                }
            }

            true
        };

        if let Some(dp) = dp {

            let result = bfs(&(p.x, p.y),
                                |&(x, y)| {
                                let mut v : Vec<(isize, isize)> = vec![];
                                if can_go(x + 1, y) { v.push((x + 1, y))};
                                if can_go(x, y + 1) { v.push((x, y + 1))};
                                if can_go(x - 1, y) { v.push((x - 1, y))};
                                if can_go(x, y - 1) { v.push((x, y - 1))};
                                v
                                },
                                |&p| p.0 == dp.x && p.1 == dp.y);

            if let Some(result) = result {
                if result.len() > 1 {
                    if data.pixel_based_movement == true {
                        data.character_instances[data.curr_index].old_position = data.character_instances[data.curr_index].position.clone();
                    }
                    data.character_instances[data.curr_index].position = Some(Position::new(p.region, result[1].0, result[1].1));
                    return BehaviorNodeConnector::Right;
                } else
                if result.len() == 1 && dp.x == result[0].0 && dp.y == result[0].1 {
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// Executes the given action in the given direction, checking for areas, loot items and NPCs
pub fn execute_targeted_action(action_name: String, dp: Option<Position>, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {

    // Find areas which contains the destination position and check if it has a fitting action node

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(dp) = &dp {

        let mut area_to_execute: Vec<(Uuid, usize, Uuid)> = vec![];

        // Check areas
        {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            let mut ids: Vec<(Uuid, usize, Uuid)> = vec![];

            for (index, area) in data.region_data.areas.iter().enumerate() {
                for p in &area.area {
                    if p.0 == dp.x && p.1 == dp.y {
                        if let Some(behavior) = data.region_area_behavior.get(index) {
                            for (id, node) in &behavior.nodes {
                                if node.behavior_type == BehaviorNodeType::ActionArea {
                                    ids.push((area.behavior, index, *id));
                                }
                            }
                        }
                    }
                }
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

            for id in ids {
                let nodes: &mut HashMap<Uuid, BehaviorNode, std::hash::BuildHasherDefault<rustc_hash::FxHasher>> = &mut data.region_area_behavior[id.1].nodes;
                if let Some(name) = get_node_string(id.2, "action", nodes) {
                    if name == action_name {
                        data.curr_action_character_index = Some(data.curr_index);
                        area_to_execute.push(id);
                    }
                }
            }
        }

        // Need to execute an area node ?
        for id in area_to_execute {
            execute_area_node(id.0, id.1, id.2);
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.curr_action_character_index = None;
            return BehaviorNodeConnector::Success;
        }

        // Check loot items
        let mut loot = vec![];

        {
            let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
            if let Some(l) = data.loot.get(&(dp.x, dp.y)) {
                loot = l.clone();
            }
        }

        for index in 0..loot.len() {
            if loot[index].state.is_none() {
                // Check if we have to create the item state
                loot[index].state = check_and_create_item_state(loot[index].id);
            }

            let mut to_execute = vec![];
            let mut item_nodes = ITEMS.borrow_mut();

            if let Some(behavior) = item_nodes.get(&loot[index].id) {
                for (id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorTree {
                        if node.name == action_name {
                            to_execute.push((behavior.id, *id));
                        }
                    }
                }
            }

            for (behavior_id, node_id) in to_execute {
                if let Some(state) = &loot[index].state {
                    *STATE.borrow_mut() = state.clone();
                    execute_node(behavior_id, node_id, &mut item_nodes);
                    loot[index].state = Some(STATE.borrow().clone());
                    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    data.loot.insert((dp.x, dp.y), loot);
                    return BehaviorNodeConnector::Success;
                } else {
                    execute_node(behavior_id, node_id, &mut item_nodes);
                    return BehaviorNodeConnector::Success;
                }
            }
        }

        // Check for characters at the dp

        let mut to_execute = vec![];

        {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            for inst_index in 0..data.character_instances.len() {
                if inst_index != data.curr_index {
                    // Only track if the state is normal
                    if data.character_instances[inst_index].state == BehaviorInstanceState::Normal {
                        if let Some(pos) = &data.character_instances[inst_index].position {
                            if *dp == *pos {
                                if let Some(behavior) = nodes.get(&data.character_instances[inst_index].behavior_id) {
                                    for (id, node) in &behavior.nodes {
                                        if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                            if node.name == action_name.clone() + " (P)" {
                                                // Install the communication partner as the target for the player
                                                data.character_instances[data.curr_index].target_instance_index = Some(inst_index);
                                                to_execute.push((inst_index, behavior.id, *id));
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        for (_inst_index, behavior_id, node_id) in to_execute {
            execute_node(behavior_id, node_id, nodes);
            //data.curr_redirected_inst_index = Some(inst_index);
            //data.execute_node(inst_index, node_id, Some(instance_index));
            //data.curr_redirected_inst_index = None;
            return BehaviorNodeConnector::Success;
        }
    }

    rc
}

/// Executes the given action in the given direction, checking for areas, loot items and NPCs
pub fn execute_region_action(instance_index: usize, action_name: String, dp: Option<Position>, data: &mut RegionInstance) -> BehaviorNodeConnector {

    // Find areas which contains the destination position and check if it has a fitting action node

    let mut rc = BehaviorNodeConnector::Fail;

    if let Some(dp) = &dp {

        let mut ids = vec![];

        for (index, area) in data.region_data.areas.iter().enumerate() {
            for p in &area.area {
                if p.0 == dp.x && p.1 == dp.y {
                    if let Some(behavior) = data.region_behavior.get(index) {
                        for (id, node) in &behavior.nodes {
                            if node.behavior_type == BehaviorNodeType::ActionArea {
                                ids.push((area.behavior, index, *id));
                            }
                        }
                    }
                }
            }
        }

        for id in ids {
            if let Some(value) = get_node_value((id.0, id.2, "action"), data, BehaviorType::Regions) {
                if let Some(name) = value.to_string() {
                    if name == action_name {
                        data.curr_action_inst_index = Some(instance_index);
                        data.execute_area_node(id.0, id.1, id.2);
                        data.curr_action_inst_index = None;
                        return BehaviorNodeConnector::Success;
                    }
                }
            }
        }

        // Check loot items

        let mut loot = vec![];
        if let Some(l) = data.loot.get(&(dp.x, dp.y)) {
            loot = l.clone();
        }

        for index in 0..loot.len() {
            if loot[index].state.is_none() {
                // Check if we have to create the item state
                //TODO loot[index].state = check_and_create_item_state(instance_index, loot[index].id, data);
            }

            let mut to_execute = vec![];

            if let Some(behavior) = data.get_behavior(loot[index].id, BehaviorType::Items) {
                for (id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorTree {
                        if node.name == action_name {
                            to_execute.push((behavior.id, *id));
                        }
                    }
                }
            }

            for (behavior_id, node_id) in to_execute {

                let has_state = loot[index].state.is_some();
                if has_state {
                    let user_scope = data.scopes[instance_index].clone();
                    let mut item_scope = rhai::Scope::new();
                    if let Some(state) = &loot[index].state {
                        // TODO state.write_to_scope(&mut item_scope);
                    }
                    data.scopes[instance_index] = item_scope;
                    data.curr_loot_item = Some((dp.x, dp.y, index));
                    data.execute_item_node(instance_index, behavior_id, node_id);
                    data.curr_loot_item = None;
                    let scope = data.scopes[instance_index].clone();
                    data.scopes[instance_index] = user_scope;
                    let mut new_buffer = ScopeBuffer::new();
                    new_buffer.read_from_scope(&scope);
                    // TODO loot[index].state = Some(new_buffer);
                    rc = BehaviorNodeConnector::Success;
                } else {
                    data.execute_item_node(instance_index, behavior_id, node_id);
                    return BehaviorNodeConnector::Success;
                }
            }
        }

        // Copy the state back
        if let Some(l) = data.loot.get_mut(&(dp.x, dp.y)) {
            for index in 0..l.len() {
                // TODO l[index].state = loot[index].state.clone();
            }
        }

        // Check for characters at the dp

        for inst_index in 0..data.instances.len() {
            if inst_index != instance_index {
                // Only track if the state is normal
                if data.instances[inst_index].state == BehaviorInstanceState::Normal {
                    if let Some(pos) = &data.instances[inst_index].position {
                        if *dp == *pos {

                            let mut to_execute = vec![];

                            if let Some(behavior) = data.get_behavior(data.instances[inst_index].behavior_id, BehaviorType::Behaviors) {
                                for (id, node) in &behavior.nodes {
                                    if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                        if node.name == action_name.clone() + " (P)" {
                                            to_execute.push((inst_index, *id));
                                        }
                                    }
                                }
                            }

                            for (inst_index, node_id) in to_execute {
                                data.curr_redirected_inst_index = Some(inst_index);
                                data.execute_node(inst_index, node_id, Some(instance_index));
                                data.curr_redirected_inst_index = None;
                                return BehaviorNodeConnector::Success;
                            }

                            break;
                        }
                    }
                }
            }
        }
    }

    rc
}

/// Get the current local instance
pub fn get_local_instance_index(instance_index: usize, data: &mut RegionInstance) -> usize {
    if let Some(redirected) = data.curr_redirected_inst_index {
        redirected
    } else {
        instance_index
    }
}

/// Drops the communication between a player and an NPC
pub fn drop_communication(instance_index: usize, npc_index: usize, data: &mut RegionData) {
    // Drop Communication for the player

    data.character_instances[instance_index].multi_choice_answer = None;
    data.character_instances[instance_index].communication = vec![];
    data.character_instances[instance_index].multi_choice_data = vec![];

    // Drop comm for the NPC

    let mut com_to_drop : Option<usize> = None;
    for c_index in 0..data.character_instances[npc_index].communication.len() {
        if data.character_instances[npc_index].communication[c_index].player_index == instance_index {
            // Drop this communication for the NPC
            com_to_drop = Some(c_index);
            break;
        }
    }

    if let Some(index) = com_to_drop {
        data.character_instances[npc_index].communication.remove(index);
    }
}

/// Check if we have to create the state for the given item
pub fn check_and_create_item_state(item_behavior_id: Uuid) -> Option<State> {

    let mut states_to_execute = vec![];
    let mut item_nodes = ITEMS.borrow_mut();

    if let Some(behavior) = item_nodes.get(&item_behavior_id) {

        let mut sink : Option<PropertySink> = None;

        // Get the default tile for the item
        for (_index, node) in &behavior.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorType {
                if let Some(value) = node.values.get(&"settings".to_string()) {
                    if let Some(str) = value.to_string() {
                        let mut s = PropertySink::new();
                        s.load_from_string(str.clone());
                        sink = Some(s);
                    }
                }
            }
        }

        // Add state ?

        if let Some(sink) = sink {
            if let Some(state) = sink.get("state") {
                if let Some(value) = state.as_bool() {
                    if value == true {
                        for (node_id, node) in &behavior.nodes {
                            if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                for (value_name, value) in &node.values {
                                    if *value_name == "execute".to_string() {
                                        if let Some(v) = value.to_integer() {
                                            if v == 1 {
                                                // Startup only tree
                                                states_to_execute.push((behavior.id, *node_id));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Create the state
    for (behavior_id, node_id) in states_to_execute {
        *STATE.borrow_mut() = State::new();
        execute_node(behavior_id, node_id, &mut item_nodes);
        return Some(STATE.borrow().clone());
    }

    None
}

/// Returns the character currency
pub fn character_currency(inst_index: usize, data: &mut RegionInstance) -> Option<i32> {
    if let Some(value) = data.scopes[inst_index].get_value::<i32>(&data.primary_currency) {
        return Some(value);
    }
    if let Some(value) = data.scopes[inst_index].get_value::<f32>(&data.primary_currency) {
        return Some(value as i32);
    }
    None
}

/// Removes the given amount from the character currency
pub fn add_to_character_currency(inst_index: usize, amount: f32, data: &mut RegionInstance) {
    if let Some(value) = data.scopes[inst_index].get_value::<i32>(&data.primary_currency) {
        let mut v = value as f32;
        v += amount;
        data.scopes[inst_index].set_value(&data.primary_currency, v);
    } else
    if let Some(value) = data.scopes[inst_index].get_value::<f32>(&data.primary_currency) {
        let mut v = value;
        v += amount;
        data.scopes[inst_index].set_value(&data.primary_currency, v);
    }
}

/// Adds the given amount to the character currency
pub fn remove_from_character_currency(inst_index: usize, amount: f32, data: &mut RegionInstance) -> bool {
    if let Some(value) = data.scopes[inst_index].get_value::<i32>(&data.primary_currency) {
        let mut v = value as f32;
        if v >= amount {
            v -= amount;
            data.scopes[inst_index].set_value(&data.primary_currency, v);
            return true;
        }
    } else
    if let Some(value) = data.scopes[inst_index].get_value::<f32>(&data.primary_currency) {
        let mut v = value;
        if v >= amount {
            v -= amount;
            data.scopes[inst_index].set_value(&data.primary_currency, v);
            return true;
        }
    }
    false
}

/// Starts to wait for the given amount of ticks
pub fn wait_start(instance_index: usize, ticks: usize, id: (Uuid, Uuid), data: &mut RegionData) {
    data.character_instances[instance_index].node_values.insert(id, Value::USize(ticks + *TICK_COUNT.borrow() as usize));
}

/// Waits for the given ticks to pass before returning true
pub fn wait_for(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionData) -> bool {
    let mut rc = true;

    if let Some(value) = data.character_instances[instance_index].node_values.get(&id) {
        match value {
            Value::USize(until) => {
                if *until >= *TICK_COUNT.borrow() as usize {
                    rc = false;
                } else {
                    data.character_instances[instance_index].node_values.clear();
                }
            },
            _ => {
            }
        }
    }
    rc
}

/// Increases the weapon skill value in the given slot
pub fn increase_weapon_skill_value(instance_index: usize, slot: String, data: &mut RegionInstance) {
    if let Some(v) = data.scopes[instance_index].get("weapons") {

        // Get the weapon skill

        let mut skill_name : String = "Unarmed".to_string();

        if let Some(weapons) = v.read_lock::<Weapons>() {
            if let Some(weapon) = weapons.slots.get(&slot) {
                if let Some(sk) = get_item_skill_tree(data, weapon.id) {
                    skill_name = sk;
                }
            }
        }

        // Increase the skill value

        if let Some(s) = data.scopes[instance_index].get_mut("skills") {
            if let Some(mut skills) = s.write_lock::<Skills>() {
                if let Some(skill) = skills.skills.get_mut(&skill_name) {
                    skill.value += 1;
                    // println!("[{}] Increased skill value {} to {}", data.instances[instance_index].name, skill_name, skill.value);

                    // Test if we need to increase the skill level

                    if let Some(tree) = data.skill_trees.get(&skill_name) {
                        let mut new_level = 0;
                        for lvl in 0..tree.len() {
                            if skill.value >= tree[lvl].0 {
                                new_level = lvl as i32;

                                // Send message
                                let message_data = MessageData {
                                    message_type    : MessageType::Status,
                                    message         : tree[lvl].2.clone(),
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
                        if new_level > skill.level {
                            skill.level = new_level;
                            // println!("[{}] Increased skill {} to level {}", data.instances[instance_index].name, skill_name, skill.level);
                        }
                    }
                }
            }
        }
    }
}

/// Returns the property script for the currently equipped weapon
pub fn get_weapon_script_id(instance_index: usize, slot: String, data: &mut RegionInstance) -> Option<(BehaviorType, Uuid, Uuid, String)> {
    if let Some(v) = data.scopes[instance_index].get("weapons") {

        // Get the weapon skill

        let mut skill_name : String = "Unarmed".to_string();
        let mut item_name : String = "Fists".to_string();

        if let Some(weapons) = v.read_lock::<Weapons>() {
            if let Some(weapon) = weapons.slots.get(&slot) {
                item_name = weapon.name.clone();
                if let Some(sk) = get_item_skill_tree(data, weapon.id) {
                    skill_name = sk;
                }
            }
        }

        // Get the skill level

        let mut skill_level = 0;

        // println!("1 skill name {:?}, item name {}", skill_name, item_name);

        if let Some(s) = data.scopes[instance_index].get("skills") {
            if let Some(skills) = s.read_lock::<Skills>() {
                if let Some(skill) = skills.skills.get(&skill_name) {
                    skill_level = skill.level;
                }
            }
        }

        // println!("2 {:?}", skill_level);

        // Get the weapon script id for the skill and level

        let skill_script_id = get_skill_script_id(data, item_name, skill_name, skill_level);

        // println!("3 {:?}", skill_script_id);

        return skill_script_id;
    }
    None
}

/// Returns the weapon distance for the given weapon slot
pub fn get_weapon_distance(slot: String, data: &mut RegionData) -> i32 {
    let mut weapon_distance = 1;

    let sheet: &mut Sheet = &mut data.sheets[data.curr_index];
    if let Some(weapon) = sheet.weapons.slots.get(&slot) {
        if weapon.weapon_distance > weapon_distance {
            weapon_distance = weapon.weapon_distance;
        }
    }

    weapon_distance
}

/// Returns the spell distance for the given spell name
pub fn get_spell_distance(instance_index: usize, name: String, data: &mut RegionData) -> i32 {
    let mut spell_distance = 3;

    let sheet: &mut Sheet = &mut data.sheets[data.curr_index];
    let spell = sheet.spells.get_spell(&name);
    spell_distance = spell.distance;


    spell_distance
}

/// Returns the PropertySink for the given item id
pub fn get_item_sink(data: &RegionInstance, id: Uuid) -> Option<PropertySink> {
    for (uuid, item) in &data.items {
        if *uuid == id {
            for (_index, node) in &item.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"settings".to_string()) {
                        if let Some(str) = value.to_string() {
                            let mut s = PropertySink::new();
                            s.load_from_string(str.clone());
                            return Some(s);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Returns the skill name (if any) for the given item id
pub fn get_item_skill_tree(data: &RegionInstance, id: Uuid) -> Option<String> {
    for (uuid, item) in &data.items {
        if *uuid == id {
            for (_index, node) in &item.nodes {
                if node.behavior_type == BehaviorNodeType::SkillTree {
                    return Some(node.name.clone());
                }
            }
        }
    }
    None
}

/// Returns the script id for the given skill name and level
pub fn get_skill_script_id(data: &RegionInstance, item_name: String, _skill_name: String, skill_level: i32) -> Option<(BehaviorType, Uuid, Uuid, String)> {
    for (_uuid, behavior) in &data.items {
        if behavior.name == item_name {
            for (_index, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::SkillTree {

                    let mut rc : Option<(BehaviorType, Uuid, Uuid, String)> = None;
                    let mut parent_id = node.id;

                    for _lvl in 0..=skill_level {
                        for (id1, c1, id2, c2) in &behavior.connections {
                            if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                for (uuid, node) in &behavior.nodes {
                                    if *uuid == *id2 {
                                        rc = Some((BehaviorType::Items, behavior.id, node.id, "script".to_string()));
                                        parent_id = node.id;
                                        break;
                                    }
                                }
                                break;
                            } else
                            if *id2 == parent_id && *c2 == BehaviorNodeConnector::Bottom {
                                for (uuid, node) in &behavior.nodes {
                                    if *uuid == *id1 {
                                        rc = Some((BehaviorType::Items, behavior.id, node.id, "script".to_string()));
                                        parent_id = node.id;
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                    }

                    return rc;
                }
            }
        }
    }
    None
}

