
use rhai::{ Scope, Dynamic };
use rand::prelude::*;

use super::behavior::BehaviorNodeType;
use crate::gamedata::GameData;

/// Updates the dices for the givem scope
fn update_dices(instance_index: usize, data: &mut GameData) {
    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        data.scopes[instance_index].set_value( format!("d{}", d), random as f64);
    }
    data.scopes[instance_index].set_value( "d100", rng.gen_range(1..=100) as f64);
}

/// Evaluates a boolean expression in the given instance.
pub fn eval_bool_expression_instance(instance_index: usize, expression: &str, data: &mut GameData) -> Option<bool> {

    update_dices(instance_index, data);

    let r = data.engine.eval_expression_with_scope::<bool>(&mut  data.scopes[instance_index], expression);
    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a numerical expression in the given instance.
pub fn eval_number_expression_instance(instance_index: usize, expression: &str, data: &mut GameData) -> Option<f64> {

    update_dices(instance_index, data);

    let r = data.engine.eval_expression_with_scope::<f64>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a dynamic expression in the given instance.
pub fn eval_dynamic_expression_instance(instance_index: usize, id: (usize, usize), expression: &str, data: &mut GameData) -> bool {

    if data.runs_in_editor {
        return eval_dynamic_expression_instance_editor(instance_index, id, expression, data);
    }

    update_dices(instance_index, data);

    let r = data.engine.eval_with_scope::<Dynamic>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {
        return true
    } else {
        println!("{:?}", r);
    }

    false
}

/// Evaluates a numerical expression in the given instance in the editor. We have to send the editor the variables which have been updated for visual disolay.
pub fn eval_dynamic_expression_instance_editor(instance_index: usize, id: (usize, usize), expression: &str, data: &mut GameData) -> bool {

    update_dices(instance_index, data);

    let original = data.scopes[instance_index].clone();

    let r = data.engine.eval_with_scope::<Dynamic>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {

        let mut key_to_change: Option<String> = None;
        let mut new_value : Option<f64> = None;

        if let Some(behavior) = data.behaviors.get_mut(&data.instances[instance_index].behavior_id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::VariableNumber {

                    let o = original.get_value::<f64>(node.name.as_str());
                    let n = data.scopes[instance_index].get_value::<f64>(node.name.as_str());

                    if n.is_some() && o.is_some() && Some(o) != Some(n) {
                        key_to_change = Some(node.name.clone());
                        new_value = Some(n.unwrap());
                    }
                }
            }
        }

        if let Some(key) = key_to_change {
            if let Some(value) = new_value {
                if let Some(behavior) = data.behaviors.get_mut(&id.0) {

                    // Insert the node id of the changed variable to the list
                    // Note: Only need todo when run in editor
                    for (_index, node) in &behavior.data.nodes {
                        if node.name == key && node.behavior_type == BehaviorNodeType::VariableNumber {
                            data.changed_variables.push((instance_index, behavior.data.id, node.id, value));
                            //println!("{:?}", (instance_index, behavior.data.id, node.id));
                        }
                    }
                }
            }
        }

        return true
    } else {
        println!("{:?}", r);
    }

    false
}

/// Evaluates a boolean expression for the given behavior.
/// This is used to verify an expression in the expression editor and not used in game (which would be instance based).
pub fn eval_bool_expression_behavior(expression: &str, behavior_id: usize, data: &mut GameData) -> Option<bool> {

    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    if let Some(behavior) = data.behaviors.get_mut(&behavior_id) {
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                let mut value : f64 = 0.0;
                if let Some(v) = n.1.values.get("value") {
                    value = v.0;
                }
                scope.push( n.1.name.as_str(), value);
            }
        }
    }
    //println!("{:?}", scope);

    let r = data.engine.eval_expression_with_scope::<bool>(&mut scope, expression);

    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a boolean expression for the given behavior.
/// This is used to verify an expression in the expression editor and not used in game (which would be instance based).
pub fn eval_number_expression_behavior(expression: &str, behavior_id: usize, data: &mut GameData) -> Option<f64> {

    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    if let Some(behavior) = data.behaviors.get_mut(&behavior_id) {
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                let mut value : f64 = 0.0;
                if let Some(v) = n.1.values.get("value") {
                    value = v.0;
                }
                scope.push( n.1.name.as_str(), value);
            }
        }
    }
    //println!("{:?}", scope);

    let r = data.engine.eval_expression_with_scope::<f64>(&mut scope, expression);

    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a boolean expression for the given behavior.
/// This is used to verify an expression in the expression editor and not used in game (which would be instance based).
pub fn eval_dynamic_expression_behavior(expression: &str, behavior_id: usize, data: &mut GameData) -> bool {

    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    if let Some(behavior) = data.behaviors.get_mut(&behavior_id) {
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                let mut value : f64 = 0.0;
                if let Some(v) = n.1.values.get("value") {
                    value = v.0;
                }
                scope.push( n.1.name.as_str(), value);
            }
        }
    }
    //println!("{:?}", scope);

    let r = data.engine.eval_with_scope::<Dynamic>(&mut scope, expression);

    if r.is_ok() {
        return true;
    } else {
        println!("{:?}", r);
    }

    false
}