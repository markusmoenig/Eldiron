
use rhai::{ Engine, Scope, Dynamic };
use rand::prelude::*;

use super::behavior::BehaviorNodeType;
use crate::gamedata::GameData;

pub fn eval_bool_expression_instance(instance_index: usize, expression: &str, data: &mut GameData) -> Option<bool> {

    let engine = Engine::new();
    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    for (key, value) in &data.instances[instance_index].values {
        scope.push( key, value.clone());
    }

    // Evaluate the expression
    let r = engine.eval_expression_with_scope::<bool>(&mut scope, expression);

    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

pub fn eval_number_expression_instance(instance_index: usize, expression: &str, data: &mut GameData) -> Option<f64> {

    let engine = Engine::new();
    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    for (key, value) in &data.instances[instance_index].values {
        scope.push( key, value.clone());
    }

    // Evaluate the expression
    let r = engine.eval_expression_with_scope::<f64>(&mut scope, expression);

    if r.is_ok() {
        return Some(r.unwrap());
    } else {
        println!("{:?}", r);
    }

    None
}

pub fn eval_dynamic_expression_instance(instance_index: usize, id: (usize, usize), expression: &str, data: &mut GameData) -> bool {

    let engine = Engine::new();
    let mut scope = Scope::new();

    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.push( format!("d{}", d), random as f64);
    }
    scope.push( "d100", rng.gen_range(1..=100) as f64);

    // Number Variables
    for (key, value) in &data.instances[instance_index].values {
        scope.push( key, value.clone());
    }

    // Evaluate the expression
    let r = engine.eval_with_scope::<Dynamic>(&mut scope, expression);

    if r.is_ok() {

        let mut key_to_change: Option<String> = None;
        let mut new_value : Option<f64> = None;

        for (key, value) in &data.instances[instance_index].values {
            if let Some(v) = scope.get_value::<f64>(key) {
                if v != *value {
                    key_to_change = Some(key.clone());
                    new_value = Some(v);
                    break;
                }
            }
        }

        if let Some(key) = key_to_change {
            if let Some(value) = new_value {
                if let Some(behavior) = data.behaviors.get_mut(&id.0) {
                    data.instances[instance_index].values.insert(key.clone(), value);

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

    let engine = Engine::new();
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

    let r = engine.eval_expression_with_scope::<bool>(&mut scope, expression);

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

    let engine = Engine::new();
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

    let r = engine.eval_expression_with_scope::<f64>(&mut scope, expression);

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

    let engine = Engine::new();
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

    let r = engine.eval_with_scope::<Dynamic>(&mut scope, expression);

    if r.is_ok() {
        return true;
    } else {
        println!("{:?}", r);
    }

    false
}