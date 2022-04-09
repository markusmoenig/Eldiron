use crate::gamedata::{ GameData, BehaviorNodeConnector };
use crate::asset::TileUsage;

use pathfinding::prelude::bfs;

/// Retrieves a number instance value
pub fn get_number_variable(instance_index: usize, variable: String, data: &mut GameData) -> Option<f64> {
    if let Some(value) = data.scopes[instance_index].get_value::<f64>(&variable) {
        return Some(value.clone());
    }
    None
}

/// Sets a number instance value
pub fn set_number_variable(instance_index: usize, variable: String, value: f64, data: &mut GameData) {
    data.scopes[instance_index].set_value(&variable, value);
}

/// Retrieves a node value
pub fn get_node_value(id: (usize, usize, &str), data: &mut GameData) -> Option<(f64, f64, f64, f64, String)> {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut(id.2) {
                return Some(value.clone());
            }
        }
    }
    None
}

/// Sets a node value
pub fn set_node_value(id: (usize, usize, &str), data: &mut GameData, value: (f64, f64, f64, f64, String)) {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(v) = node.values.get_mut(id.2) {
                *v = value;
            }
        }
    }
}

/// Computes the distance between two locations
pub fn compute_distance(p0: &(usize, isize, isize), p1: &(usize, isize, isize)) -> f64 {
    let dx = p0.1 - p1.1;
    let dy = p0.2 - p1.2;
    ((dx * dx + dy * dy) as f64).sqrt()
}

pub fn walk_towards(instance_index: usize, p: Option<(usize, isize, isize)>, dp: Option<(usize, isize, isize)>, data: &mut GameData) -> BehaviorNodeConnector {

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