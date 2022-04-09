use crate::gamedata::GameData;

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
