extern crate ref_thread_local;
use ref_thread_local::{RefThreadLocal};

use crate::prelude::*;

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
                // let curr_scope = self.scopes[inst_index].clone();
                // self.scopes[inst_index] = Scope::new();

                execute_node(behavior_id, node_id, item_nodes);
                // let scope = self.scopes[inst_index].clone();
                // self.scopes[inst_index] = curr_scope;
                // let mut buffer = ScopeBuffer::new();
                // buffer.read_from_scope(&scope);
                // item.state = Some(buffer);
            }
            /*
            if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                if let Some(mut inv) = mess.write_lock::<Inventory>() {
                    // Test if the item is queued to be equipped
                    if let Some(queued_index) = to_equip_queued.iter().position(|name| *name == item.name) {
                        to_equip_queued.remove(queued_index);
                        to_equip.push(item);
                    } else {
                        inv.add_item(item);
                    }
                }
            }*/

            break;
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
                for id in &data.character_instances[inst_index].tree_ids {
                    if let Some(node) = behavior.nodes.get(&id) {
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
                            for (value_name, value) in &node.values {
                                if *value_name == "execute".to_string() {
                                    if let Some(v) = value.to_integer() {
                                        if v == 0 {
                                            // "Always execute" only tree
                                            for c in &system.connections {
                                                if c.0 == *id {
                                                    execute_node(*system_id, c.0, systems);
                                                    return true;
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
                    //self.executed_connections.push((BehaviorType::Items, c.0, c.1));
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
