
use crate::gamedata::behavior:: { BehaviorNodeConnector };
use crate::gamedata::GameData;
use crate::asset::TileUsage;

use crate::gamedata::nodes_utility::*;
use crate::gamedata::script::*;

use pathfinding::prelude::bfs;

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
        //println!("{}", value.4);
        data.say.push(format!("{} says \"{}\".", data.instances[instance_index].name, value.4));
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
    let delay = 10.0 - speed;
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
    if distance.round() == 0.0 {
        return BehaviorNodeConnector::Success;
    }

    if let Some(p) = p {

        let can_go = |x: isize, y: isize| -> bool {

            if let Some(tile) = data.get_tile_at((p.0, x, y)) {
                if tile.3 == TileUsage::Environment {
                    return true;
                }
            }
            false
        };

        if let Some(dp) = dp {

            let result = bfs(&(p.1, p.2),
                                |&(x, y)| {
                                let mut v : Vec<(isize, isize)> = vec![];
                                if can_go(x + 1, y) { v.push((x + 1, y))};
                                if can_go(x, y + 1) { v.push((x, y + 1))};
                                if can_go(x - 1, y) { v.push((x - 1, y))};
                                if can_go(x, y - 1) { v.push((x, y - 1))};
                                v
                                },
                                |&p| p.0 == dp.1 && p.1 == dp.2);

            if let Some(result) = result {
                //println!("{:?}", result);
                if result.len() > 1 {
                    data.instances[instance_index].position = Some((p.0, result[1].0, result[1].1));
                    if let Some(value) = &mut get_node_value((id.0, id.1, "destination"), data) {
                        value.3 = 0.0;
                        set_node_value((id.0, id.1, "destination"), data, value.clone());
                    }
                    return BehaviorNodeConnector::Right;
                } else
                if result.len() == 1 && dp.1 == result[0].0 && dp.2 == result[0].1 {
                    if let Some(value) = &mut get_node_value((id.0, id.1, "destination"), data) {
                        value.3 = 1.0;
                        set_node_value((id.0, id.1, "destination"), data, value.clone());
                    }
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }
    BehaviorNodeConnector::Fail
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

    // TODO, evaluate the expression

    for inst_ind in &chars {
        //println!("targetting {}", data.instances[*inst_ind].name);
        data.instances[instance_index].target = Some(*inst_ind);
        return BehaviorNodeConnector::Success;
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
    let delay = 10.0 - speed;
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

    // Perform the pathfinding
    if let Some(p) = p {

        let can_go = |x: isize, y: isize| -> bool {

            if let Some(tile) = data.get_tile_at((p.0, x, y)) {
                if tile.3 == TileUsage::Environment {
                    return true;
                }
            }
            false
        };

        if let Some(dp) = dp {

            let result = bfs(&(p.1, p.2),
                                |&(x, y)| {
                                let mut v : Vec<(isize, isize)> = vec![];
                                if can_go(x + 1, y) { v.push((x + 1, y))};
                                if can_go(x, y + 1) { v.push((x, y + 1))};
                                if can_go(x - 1, y) { v.push((x - 1, y))};
                                if can_go(x, y - 1) { v.push((x, y - 1))};
                                v
                                },
                                |&p| p.0 == dp.1 && p.1 == dp.2);

            if let Some(result) = result {
                //println!("{:?}", result);
                if result.len() > 1 {
                    data.instances[instance_index].position = Some((p.0, result[1].0, result[1].1));
                    return BehaviorNodeConnector::Right;
                } else
                if result.len() == 1 && dp.1 == result[0].0 && dp.2 == result[0].1 {
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// Attack
pub fn attack(_instance_index: usize, _id: (usize, usize), _data: &mut GameData) -> BehaviorNodeConnector {
    BehaviorNodeConnector::Fail
}
