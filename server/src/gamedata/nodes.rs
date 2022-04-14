use crate::gamedata::behavior:: { BehaviorNodeConnector };
use crate::gamedata::GameData;

use crate::gamedata::nodes_utility::*;
use crate::gamedata::script::*;

/// expression
pub fn expression(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(value) = get_node_value((id.0, id.1, "expression"), data) {
        let rc = eval_bool_expression_instance(instance_index, value.4.as_str(), data);
        if let Some(rc) = rc {
            if rc == true {
                return BehaviorNodeConnector::Success;
            }
        }
    }
    BehaviorNodeConnector::Fail
}

/// script
pub fn script(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(value) = get_node_value((id.0, id.1, "script"), data) {
        eval_dynamic_script_instance(instance_index, id, value.4.as_str(), data);
    }
    BehaviorNodeConnector::Bottom
}

/// say
pub fn say(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(value) = get_node_value((id.0, id.1, "text"), data) {
        data.say.push(format!("{} says \"{}\".", data.instances[instance_index].name, value.4));
    } else {
        data.say.push(format!("{} says \"{}\".", data.instances[instance_index].name, "Hello".to_string()));
    }
    BehaviorNodeConnector::Bottom
}

/// Pathfinder
pub fn pathfinder(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {

    let mut p : Option<(usize, isize, isize)> = None;
    let mut dp : Option<(usize, isize, isize)> = None;

    let mut distance = 100000_f64;

    if let Some(v) = &mut data.instances[instance_index].position {
        p = Some(*v);
    }

    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get("destination") {
                dp = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                if let Some(p) = p {
                    distance = compute_distance(&p, &dp.unwrap()).round();
                }
            }
        }
    }

    let mut speed : f64 = 8.0;
    if let Some(value) = get_node_value((id.0, id.1, "speed"), data) {
        let rc = eval_number_expression_instance(instance_index, value.4.as_str(), data);
        if let Some(rc) = rc {
            speed = rc;
        }
    }

    // Apply the speed delay
    let delay = 10.0 - speed.clamp(0.0, 10.0);
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut("speed") {
                if value.0 >= delay {
                    value.0 = 0.0;
                } else {
                    value.0 += 1.0;
                    if distance > 0.0 {
                        return BehaviorNodeConnector::Right;
                    } else {
                        return BehaviorNodeConnector::Success;
                    }
                }
            }
        }
    }

    // Success if we reached the to_distance already
    if distance == 0.0 {
        return BehaviorNodeConnector::Success;
    }

    walk_towards(instance_index, p, dp,false, data)
}

/// Lookout
pub fn lookout(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {

    let mut max_distance : f64 = 7.0;
    if let Some(value) = get_node_value((id.0, id.1, "max_distance"), data) {
        let rc = eval_number_expression_instance(instance_index, value.4.as_str(), data);
        if let Some(rc) = rc {
            max_distance = rc;
        }
    }

    // Find the chars within the given distance

    let mut chars : Vec<usize> = vec![];

    if let Some(position) = data.instances[instance_index].position {
        for index in 0..data.active_instance_indices.len() {
            let inst_index = data.active_instance_indices[index];
            if inst_index != instance_index {
                if let Some(pos) = data.instances[inst_index].position {
                    let dx = position.1 - pos.1;
                    let dy = position.2 - pos.2;
                    let d = ((dx * dx + dy * dy) as f64).sqrt();
                    if d <= max_distance {
                        chars.push(inst_index);
                        //println!("distance {}", d);
                    }
                }
            }
        }
    }

    if let Some(value) = get_node_value((id.0, id.1, "expression"), data) {
        for inst_ind in &chars {
            let r = data.engine.eval_expression_with_scope::<bool>(&mut  data.scopes[*inst_ind], &value.4);
            if let Some(rc) = r.ok() {
                if rc {
                    data.instances[instance_index].target = Some(*inst_ind);
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }

    data.instances[instance_index].target = None;
    BehaviorNodeConnector::Fail
}

/// CloseIn
pub fn close_in(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {

    let mut p : Option<(usize, isize, isize)> = None;
    let mut dp : Option<(usize, isize, isize)> = None;

    let mut distance = 100000_f64;
    let mut to_distance = 1_f64;

    if let Some(value) = get_node_value((id.0, id.1, "to_distance"), data) {
        let rc = eval_number_expression_instance(instance_index, value.4.as_str(), data);
        if let Some(rc) = rc {
            to_distance = rc;
        }
    }

    if let Some(v) = &mut data.instances[instance_index].position {
        p = Some(*v);
    }

    let target_index = data.instances[instance_index].target;

    if let Some(target_index) = target_index {
        if let Some(v) = &mut data.instances[target_index].position {
            dp = Some(*v);
            if let Some(p) = p {
                distance = compute_distance(&p, v);
            }
        }
    }

    let mut speed : f64 = 8.0;
    if let Some(value) = get_node_value((id.0, id.1, "speed"), data) {
        let rc = eval_number_expression_instance(instance_index, value.4.as_str(), data);
        if let Some(rc) = rc {
            speed = rc;
        }
    }

    // Apply the speed delay
    let delay = 10.0 - speed.clamp(0.0, 10.0);
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut("speed") {
                if value.0 >= delay {
                    value.0 = 0.0;
                } else {
                    value.0 += 1.0;
                    if distance > to_distance {
                        return BehaviorNodeConnector::Right;
                    } else {
                        return BehaviorNodeConnector::Success;
                    }
                }
            }
        }
    }

    // Success if we reached the to_distance already
    if distance <= to_distance {
        return BehaviorNodeConnector::Success;
    }

    walk_towards(instance_index, p, dp, true, data)
}

/// Systems Call
pub fn systems_call(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {

    let systems_id : Option<usize>;
    let mut systems_tree_id : Option<usize> = None;

    if let Some(value) = get_node_value((id.0, id.1, "system"), data) {
        systems_id = Some(data.systems_ids[value.0 as usize]);
    } else {
        systems_id = Some(data.systems_ids[0]);
    }

    if let Some(value) = get_node_value((id.0, id.1, "tree"), data) {
        systems_tree_id = Some(value.0 as usize);
    }

    if let Some(systems_id) = systems_id {
        if let Some(systems_tree_id) = systems_tree_id {
            //println!("{} {}", systems_id, systems_tree_id);

            let buffer = data.instances[instance_index].behavior_id;
            data.instances[instance_index].behavior_id = systems_id;
            data.execute_systems_node(instance_index, systems_tree_id);
            data.instances[instance_index].behavior_id = buffer;
        }
    }

    BehaviorNodeConnector::Bottom
}
