use crate::prelude::*;
use pathfinding::prelude::bfs;

/// Retrieves a number instance value
pub fn get_number_variable(instance_index: usize, variable: String, data: &mut RegionInstance) -> Option<f32> {
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return Some(value.clone());
    }
    None
}

/// Retrieves a number instance value or 0
pub fn get_number_variable_or_zero(instance_index: usize, variable: String, data: &RegionInstance) -> f32 {
    if let Some(value) = data.scopes[instance_index].get_value::<f32>(&variable) {
        return value.clone();
    }
    0.0
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
pub fn compute_distance(p0: &Position, p1: &Position) -> f64 {
    let dx = p0.x - p1.x;
    let dy = p0.y - p1.y;
    ((dx * dx + dy * dy) as f64).sqrt()
}

/// Returns the current position of the instance_index, takes into account an ongoing animation
pub fn get_instance_position(inst_index: usize, instances: &Vec<BehaviorInstance>) -> Option<Position> {
    if let Some(old_position) = &instances[inst_index].old_position {
        return Some(old_position.clone());
    }
    instances[inst_index].position.clone()
}

pub fn walk_towards(instance_index: usize, p: Option<Position>, dp: Option<Position>, exclude_dp: bool, data: &mut RegionInstance) -> BehaviorNodeConnector {

    // Cache the character positions
    let mut char_positions : Vec<Position> = vec![];

    if let Some(p) = &p {
        for inst_index in 0..data.instances.len() {
            if inst_index != instance_index {
                // Only track if the state is normal
                if data.instances[inst_index].state == BehaviorInstanceState::Normal {
                    if let Some(pos) = &data.instances[inst_index].position {
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
                    data.instances[instance_index].old_position = data.instances[instance_index].position.clone();
                    data.instances[instance_index].position = Some(Position::new(p.region, result[1].0, result[1].1));
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

pub fn execute_region_action(instance_index: usize, action_name: String, dp: Option<Position>, data: &mut RegionInstance) -> BehaviorNodeConnector {

    // Find areas which contains the destination position and check if it has a fitting action node

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
    }

    BehaviorNodeConnector::Fail
}