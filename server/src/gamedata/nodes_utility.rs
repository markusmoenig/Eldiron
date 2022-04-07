use crate::gamedata::GameData;

/// Retrieves a number instance value
pub fn get_number_variable(instance_index: usize, variable: String, data: &mut GameData) -> Option<f64> {
    if let Some(value) = data.instances[instance_index].values.get(&variable) {
        return Some(value.clone());
    }
    None
}

/// Sets a number instance value
pub fn set_number_variable(instance_index: usize, variable: String, value: f64, data: &mut GameData) {
    if let Some(v) = data.instances[instance_index].values.get_mut(&variable) {
        *v = value;
    }
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
