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
                            item.state = Some(ScopeBuffer::new());
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

/// Executes the given item node and follows the connection chain
 fn execute_node(behavior_id: Uuid, node_id: Uuid, nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> Option<BehaviorNodeConnector> {

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

