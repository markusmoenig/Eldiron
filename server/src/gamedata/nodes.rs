
use crate::gamedata::behavior:: { BehaviorNodeConnector, BehaviorNodeType };
use crate::gamedata::GameData;
use crate::asset::TileUsage;

use crate::gamedata::nodes_utility::*;

use pathfinding::prelude::bfs;

use evalexpr::*;
use rand::prelude::*;

/// expression
pub fn expression(_instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {

        // Insert the variables
        let mut cont = HashMapContext::new();
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                if let Some(value) = n.1.values.get("value") {
                    let t = format!("{} = {}", n.1.name, value.0);
                    let _ = eval_empty_with_context_mut(t.as_str(), &mut cont);
                }
            }
        }

        // d2 - d20
        let mut rng = thread_rng();
        for d in (2..=20).step_by(2) {
            let random = rng.gen_range(1..=d);
            let t = format!("{} = {}", format!("d{}", d), random);
            let _ = eval_empty_with_context_mut(t.as_str(), &mut cont);
        }

        // Evaluate the expression
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get("expression") {
                let exp = eval_boolean_with_context(&value.4, &cont);
                if exp.is_ok() {
                    if exp == Ok(true) {
                        return BehaviorNodeConnector::Success;
                    }
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// say
pub fn say(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get("text") {
                //println!("{}", value.4);
                data.say.push(format!("{} says \"{}\".", data.instances[instance_index].name, value.4));
            }
        }
    }
    BehaviorNodeConnector::Bottom
}

/// Pathfinder
pub fn pathfinder(instance_index: usize, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {

    let mut p : Option<(usize, isize, isize)> = None;
    let mut dp : Option<(usize, isize, isize)> = None;

    let delay= eval_expression_as_number(id, data, "delay", 1.0);
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut("delay") {
                if value.0 >= delay {
                    value.0 = 0.0;
                } else {
                    value.0 += 1.0;
                    if let Some(value) = &mut get_value((id.0, id.1, "destination"), data) {
                        if value.3 == 0.0 {
                            return BehaviorNodeConnector::Right;
                        } else {
                            return BehaviorNodeConnector::Success;
                        }
                    }
                }
            }
        }
    }

    if let Some(v) = &mut data.instances[instance_index].position {
        p = Some(*v);
    }

    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get("destination") {
                dp = Some((value.0 as usize, value.1 as isize, value.2 as isize));
            }
        }
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
                    if let Some(value) = &mut get_value((id.0, id.1, "destination"), data) {
                        value.3 = 0.0;
                        set_value((id.0, id.1, "destination"), data, value.clone());
                    }
                    return BehaviorNodeConnector::Right;
                } else
                if result.len() == 1 && dp.1 == result[0].0 && dp.2 == result[0].1 {
                    if let Some(value) = &mut get_value((id.0, id.1, "destination"), data) {
                        value.3 = 1.0;
                        set_value((id.0, id.1, "destination"), data, value.clone());
                    }
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }
    BehaviorNodeConnector::Fail
}
