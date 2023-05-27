extern crate ref_thread_local;
use ref_thread_local::{RefThreadLocal};

use crate::prelude::*;

/// Returns the damage of the main hand
pub fn roll_weapon_damage(sheet: &mut Sheet) -> i32 {
    let weapon = sheet.weapons.slot(&"main hand");
    let mut skill_name = "Unarmed".to_string();

    if let Some(skill) = get_item_skill_name(weapon.name.clone()) {
        skill_name = skill;
    }

    let mut level = 0;
    if let Some(skill) = sheet.skills.skills.get(&skill_name) {
        level = skill.level;
    }

    if let Some((value, delay)) = get_item_skill_level_value_delay(weapon.name, level) {
        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.character_instances[data.curr_index].sleep_cycles = delay as usize;
        value
    } else {
        0
    }
}

/// Returns the skill name for the given item
pub fn get_item_skill_name(item_name: String) -> Option<String> {
    if let Some(items) = &ITEMS.try_borrow().ok() {
        for (_id, behavior) in items.iter() {
            if behavior.name == item_name {
                for (_id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::SkillTree {
                        return Some(node.name.clone());
                    }
                }
            }
        }
    }
    None
}

/// Get the value and delay data for the skill level of the given item.
pub fn get_item_skill_level_value_delay(item_name: String, level: i32) -> Option<(i32, i32)> {

    let mut skill_level_id : Option<(Uuid, Uuid)> = None;

    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    data.item_effects = None;

    if let Some(items) = &ITEMS.try_borrow_mut().ok() {
        for (id, behavior) in items.iter() {
            if  behavior.name == item_name {
                for (_id, node) in &behavior.nodes {
                    //println!("{:?}, {}, {}", node.behavior_type, node.name, item_name);
                    if node.behavior_type == BehaviorNodeType::SkillTree {
                        let mut parent_id = node.id;

                        for _lvl in 0..=level {
                            for (id1, c1, id2, c2) in &behavior.connections {
                                if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                    for (uuid, node) in &behavior.nodes {
                                        if *uuid == *id2 {
                                            skill_level_id = Some((behavior.id, node.id));
                                            parent_id = node.id;

                                            // Check if there are some effects on the right
                                            for (id1, c1, id2, _c2) in &behavior.connections {
                                                if *id1 == *uuid && *c1 == BehaviorNodeConnector::Right {
                                                    data.item_effects = Some((*id, *id2));
                                                    break;
                                                }
                                            }

                                            break;
                                        }
                                    }
                                    break;
                                } else
                                if *id2 == parent_id && *c2 == BehaviorNodeConnector::Bottom {
                                    for (uuid, node) in &behavior.nodes {
                                        if *uuid == *id1 {
                                            skill_level_id = Some((behavior.id, node.id));
                                            parent_id = node.id;

                                            // Check if there are some effects on the right
                                            for (_id1, _c1, id2, c2) in &behavior.connections {
                                                if *id2 == *uuid && *c2 == BehaviorNodeConnector::Right {
                                                    data.item_effects = Some((*id, *id2));
                                                    break;
                                                }
                                            }

                                            break;
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        break;
                    }
                }
                break;
            }
        }
    }

    let mut rc : Option<(i32, i32)> = None;

    if let Some(id) = skill_level_id {
        let mut value = 0;
        let mut delay = 0;

        if let Some(items) = &mut ITEMS.try_borrow_mut().ok() {
            if let Some(v) = eval_script_integer(id, "script", &mut *items) {
                value = v;
            }

            if let Some(v) = eval_script_integer(id, "delay", &mut *items) {
                delay = v;
            }
        }
        rc = Some((value, delay));
    }

    rc
}

/// Increases the given skill by the given amount
pub fn increase_skill_by(sheet: &mut Sheet, skill_name: String, amount: i32) {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(skill) = sheet.skills.skills.get_mut(&skill_name) {
        skill.value += amount;
        // println!("[{}] Increased skill value {} to {}", data.instances[instance_index].name, skill_name, skill.value);

        // Test if we need to increase the skill level

        if let Some(tree) = data.skill_trees.get(&skill_name) {
            let mut new_level = 0;
            for lvl in 0..tree.len() {
                if skill.value >= tree[lvl].0 {
                    new_level = lvl as i32;

                    if new_level > skill.level {
                        // Send message
                        let message_data = MessageData {
                            message_type    : MessageType::Status,
                            message         : tree[lvl].2.clone(),
                            from            : "System".to_string(),
                            right           : None,
                            center          : None,
                            buffer          : None
                        };

                        data.character_instances[data.curr_index].messages.push(message_data.clone());
                    }
                } else {
                    break;
                }
            }
            if new_level > skill.level {
                skill.level = new_level;
                //println!("[{}] Increased skill {} to level {}", data.character_instances[data.curr_index].name, skill_name, skill.level);
            }
        }
    }
}

/// Increases the experience by the given amount
pub fn increase_experience_by(sheet: &mut Sheet, amount: i32) {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    // Add the experience

    sheet.experience.experience += amount;

    let mut str = sheet.experience.experience_msg.clone();
    str = str.replace("{}", &amount.to_string());

    // Send message
    let message_data = MessageData {
        message_type    : MessageType::Status,
        message         : str,
        from            : "System".to_string(),
        right           : None,
        center          : None,
        buffer          : None
    };

    data.character_instances[data.curr_index].messages.push(message_data.clone());

    let mut new_level = 0;
    for lvl in 0..sheet.experience.levels.len() {
        if sheet.experience.experience >= sheet.experience.levels[lvl].0 {
            new_level = lvl as i32 + 1;

            if new_level > sheet.experience.level {
                // Send message
                let message_data = MessageData {
                    message_type    : MessageType::Status,
                    message         : sheet.experience.levels[lvl].1.clone(),
                    from            : "System".to_string(),
                    right           : None,
                    center          : None,
                    buffer          : None
                };

                data.character_instances[data.curr_index].messages.push(message_data.clone());
            }
        } else {
            break;
        }
    }
    if new_level > sheet.experience.level {
        sheet.experience.level = new_level;

        eval_script((sheet.experience.level_behavior_id, sheet.experience.levels[new_level as usize-1].2), "script", &mut SYSTEMS.borrow_mut());

        println!("[{}] Advanced to level {}", data.character_instances[data.curr_index].name, sheet.experience.level);
    }
}

/// Add the item identified by its name to the inventory.
pub fn inventory_add(sheet: &mut Sheet, item_name: &str, amount: i32, item_nodes: &mut FxHashMap<Uuid, GameBehaviorData>) {
    for (_id, behavior) in item_nodes.iter() {
        if behavior.name == item_name {
            let mut tile_data : Option<TileData> = None;
            let mut sink : Option<PropertySink> = None;

            // Get the default tile for the item
            for (_index, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"tile".to_string()) {
                        tile_data = value.to_tile_data();
                    }
                    if let Some(value) = node.values.get(&"settings".to_string()) {
                        if let Some(str) = value.to_string() {
                            let mut s = PropertySink::new();
                            s.load_from_string(str.clone());
                            sink = Some(s);
                        }
                    }
                }
            }

            let mut item = Item::new(behavior.id, behavior.name.clone());
            item.item_type = "gear".to_string();
            item.tile = tile_data;
            item.amount = amount;
            item.stackable = 1;

            // Add state ?

            let mut states_to_execute = vec![];

            if let Some(sink) = sink {
                if let Some(state) = sink.get("state") {
                    if let Some(value) = state.as_bool() {
                        if value == true {
                            item.state = Some(State::new());
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
                item.read_from_sink(&sink);
            }

            sheet.inventory.add_item(item);

            for (behavior_id, node_id) in states_to_execute {
                execute_node(behavior_id, node_id, item_nodes);
            }

            break;
        }
    }
}

/// Equip an item in the inventory identified by its name
pub fn inventory_equip(sheet: &mut Sheet, item_name: &str) {
    // Get the inventory index of the item to equip
    let mut index: Option<usize> = None;
    for (i, item) in sheet.inventory.items.iter().enumerate() {
        if item.name == item_name {
            index = Some(i);
            break;
        }
    }

    let mut to_equip: Option<Item> = None;
    let mut to_add_back_to_inventory: Vec<Item> = vec![];

    // Remove the item to equip from the inventory

    if let Some(index) = index {
        to_equip = Some(sheet.inventory.items.remove(index));
    }

    if let Some(to_equip) = to_equip {
        let item_type = to_equip.item_type.clone().to_lowercase();
        if let Some(slot) = to_equip.slot.clone() {
            if item_type == "weapon" {
                // Remove existing item in the slot
                if let Some(w) = sheet.weapons.slots.remove(&slot) {
                    to_add_back_to_inventory.push(w);
                }
                // Insert the new weapon into the slot
                sheet.weapons.slots.insert(slot, to_equip);
            } else
            if item_type == "gear" {
                // Remove existing item in the slot
                if let Some(g) = sheet.gear.slots.remove(&slot) {
                    to_add_back_to_inventory.push(g);
                }
                // Insert the new weapon into the slot
                sheet.gear.slots.insert(slot, to_equip);
            }
        }
    }

    // Add removed items in the equipped slot(s) back into the inventory
    if to_add_back_to_inventory.is_empty() == false {
        for item in to_add_back_to_inventory {
            sheet.inventory.items.push(item);
        }
    }
}

/// Returns the inventory item at the given inventory index for the current player and optionally sets the state.
pub fn get_inventory_item_at(index: usize, set_state: bool) -> Option<Item> {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let sheet = &mut data.sheets[data.curr_index];
    if index < sheet.inventory.items.len(){
        if set_state {
            if let Some(state) = sheet.inventory.items[index as usize].state.clone() {
                *STATE.borrow_mut() = state;
            }
        }
        return Some(sheet.inventory.items[index as usize].clone());
    }
    None
}

/// Sets the state for the inventory item at the given index
pub fn set_inventory_item_state_at(index: usize) {
    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let sheet = &mut data.sheets[data.curr_index];
    if index < sheet.inventory.items.len() {
        if sheet.inventory.items[index as usize].state.is_some() {
            sheet.inventory.items[index as usize].state = Some(STATE.borrow().clone());
        }
    }
}

/// Get the items creating light in the inventory
pub fn get_inventory_lights(data: &RegionData) -> Vec<LightData> {
    let sheet = &data.sheets[data.curr_index];

    let mut lights = vec![];

    for item in &sheet.inventory.items {
        if let Some(state) = &item.state {
            if let Some(light) = &state.light {
                lights.push(light.clone());
            }
        }
    }

    lights
}

/// Executes the tree (given by its name) inside the character, if no tree is found, check the system class of the
/// character, after that check the system race of the character.
pub fn execute_behavior(inst_index: usize, tree_name: &str) -> bool {
    let behavior_id;
    let class_name;
    let race_name;

    // This fill contain the Uuid of the tree we will execute, once we have found it in the behaviors or system class / race.
    let mut tree_id : Option<Uuid> = None;

    {
        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        behavior_id = data.character_instances[inst_index].behavior_id;
        class_name = data.sheets[inst_index].class_name.clone();
        race_name = data.sheets[inst_index].race_name.clone();

        if let Some(behaviors) = BEHAVIORS.try_borrow().ok() {
            if let Some(behavior) = behaviors.get(&behavior_id) {
                for (id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name {
                        if node.name == tree_name {
                            tree_id = Some(*id);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Execute the tree inside the characters behavior
    if let Some(tree_id) = tree_id {
        if let Some(mut behavior) = BEHAVIORS.try_borrow_mut().ok() {
            execute_node(behavior_id, tree_id, &mut behavior);
        }
        return true;
    }

    // Try to execute the tree in the system class
    fn execute_system(system_name: &str, tree_name: &str) -> bool {
        if system_name.is_empty() == false && tree_name.is_empty() == false {
            let systems = &mut SYSTEMS.borrow_mut();
            for (system_id, system) in systems.iter() {
                if system.name == system_name {
                    for (id, node) in &system.nodes {
                        if node.behavior_type == BehaviorNodeType::BehaviorTree && node.name == tree_name{
                            //for (value_name, value) in &node.values {
                                //if *value_name == "execute".to_string() {
                                    //if let Some(v) = value.to_integer() {
                                        //if v == 0 {
                                            // "Always execute" only tree
                                            for c in &system.connections {
                                                if c.0 == *id {
                                                    execute_node(*system_id, c.0, systems);
                                                    return true;
                                                }
                                            }
                                        //}
                                    //}
                                //}
                            //}
                        }
                    }
                }
            }
        }
        false
    }

    // Look for tree in character class
    let mut rc = execute_system(class_name.as_str(), tree_name);
    if rc {
        return true;
    }

    // Look for tree in character race
    rc = execute_system(race_name.as_str(), tree_name);
    if rc {
        return true;
    }

    false
}

/// Executes the given item node and follows the connection chain
pub fn execute_node(behavior_id: Uuid, node_id: Uuid, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> Option<BehaviorNodeConnector> {

    let mut connectors : Vec<BehaviorNodeConnector> = vec![];
    let mut connected_node_ids : Vec<Uuid> = vec![];

    let mut rc : Option<BehaviorNodeConnector> = None;

    let mut behavior_type : Option<BehaviorNodeType> = None;
    if let Some(item) = nodes.get(&behavior_id) {
        if let Some(node) = item.nodes.get(&node_id) {
            behavior_type = Some(node.behavior_type);
        }
    }

    if let Some(behavior_type) = behavior_type {
        // Handle behavior tree
        if behavior_type == BehaviorNodeType::BehaviorTree {
            connectors.push(BehaviorNodeConnector::Bottom1);
            connectors.push(BehaviorNodeConnector::Bottom2);
            connectors.push(BehaviorNodeConnector::Bottom);
        } else {
            let mut node_call: Option<NodeDataCall> = None;

            {
                let data = REGION_DATA.borrow();
                if let Some(call) = data[*CURR_INST.borrow()].nodes.get(&behavior_type) {
                    node_call = Some(call.clone());
                }
            }

            if let Some(node_call) = node_call {
                let connector = node_call((behavior_id, node_id), nodes);
                rc = Some(connector);
                connectors.push(connector);
            } else {
                connectors.push(BehaviorNodeConnector::Bottom);
            }
        }
    }

    // Search the connections to check if we can find an ongoing node connection
    for connector in connectors {
        if let Some(item) = nodes.get(&behavior_id) {

            for c in &item.connections {
                if c.0 == node_id && c.1 == connector {
                    connected_node_ids.push(c.2);

                    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    data.executed_connections.push((c.0, c.1));
                }
            }
        }
    }

    // And if yes execute it
    for (_index, connected_node_id) in connected_node_ids.iter().enumerate() {
        if let Some(_connector) = execute_node(behavior_id, *connected_node_id, nodes) {
        }
    }

    /*
    // Call the node and get the resulting BehaviorNodeConnector
    if let Some(item) = self.items.get_mut(&behavior_id) {
        if let Some(node) = item.nodes.get_mut(&node_id) {

            // Handle special nodes
            if node.behavior_type == BehaviorNodeType::BehaviorTree {
                connectors.push(BehaviorNodeConnector::Bottom1);
                connectors.push(BehaviorNodeConnector::Bottom2);
                connectors.push(BehaviorNodeConnector::Bottom);
            } else {
                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    let connector = node_call(instance_index, (behavior_id, node_id), self, BehaviorType::Items);
                    rc = Some(connector);
                    connectors.push(connector);
                } else {
                    connectors.push(BehaviorNodeConnector::Bottom);
                }
            }
        }
    }*/

    /*
    // Search the connections to check if we can find an ongoing node connection
    for connector in connectors {
        if let Some(item) = self.items.get_mut(&behavior_id) {

            for c in &item.connections {
                if c.0 == node_id && c.1 == connector {
                    connected_node_ids.push(c.2);
                    //self.executed_connections.push((BehaviorType::Items, c.0, c.1));
                }
            }
        }
    }

    // And if yes execute it
    for (index, connected_node_id) in connected_node_ids.iter().enumerate() {
        if let Some(connector) = self.execute_item_node(instance_index, item_id, *connected_node_id) {
        }
    }
    */
    rc
}
