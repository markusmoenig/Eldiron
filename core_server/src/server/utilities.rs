use crate::prelude::*;


/// Retrieves a node value
pub fn get_node_value(id: (usize, usize, &str), data: &mut GameData, behavior_type: BehaviorType, region_id: usize) -> Option<(f64, f64, f64, f64, String)> {
    if behavior_type == BehaviorType::Regions {

        if let Some(region) = data.regions.get_mut(&region_id) {
            let behavior = &mut region.behaviors[id.0];
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        } else
        if let Some(behavior) = data.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::Behaviors {
        if let Some(behavior) = data.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::Systems {
        if let Some(system) = data.systems.get_mut(&id.0) {
            if let Some(node) = system.data.nodes.get_mut(&id.1) {
                if let Some(value) = node.values.get_mut(id.2) {
                    return Some(value.clone());
                }
            }
        }
    } else
    if behavior_type == BehaviorType::GameLogic {
        let game = &mut data.game.behavior;
        if let Some(node) = game.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut(id.2) {
                return Some(value.clone());
            }
        }
    }
    None
}