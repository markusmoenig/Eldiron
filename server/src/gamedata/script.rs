
use rhai::{ Engine, Scope, Dynamic };
use rand::prelude::*;
use std::collections::HashMap;

use super::behavior::{ BehaviorNodeType, BehaviorType };
use crate::gamedata::GameData;

#[derive(Debug, Clone)]
struct InstanceVariables {
    pub numbers: HashMap<String, f64>
}

impl InstanceVariables {
    fn get_number(&mut self, index: String) -> f64 {
        self.numbers[&index]
    }
    fn set_number(&mut self, index: String, value: f64) {
        self.numbers.insert(index, value);
    }

    fn new() -> Self {
        Self { numbers: HashMap::new() }
    }
}

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

/// Updates the dices for the givem scope
pub fn update_dices_scope(scope: &mut Scope) {
    // Dices
    let mut rng = thread_rng();
    for d in (2..=20).step_by(2) {
        let random = rng.gen_range(1..=d);
        scope.set_value( format!("d{}", d), random as f64);
    }
    scope.set_value( "d100", rng.gen_range(1..=100) as f64);
}

// pub fn create_character_struct(character_index: usize, data: &mut GameData) -> CharacterVariables {


// }

/// Evaluates a boolean expression in the given instance.
pub fn eval_bool_expression_instance(instance_index: usize, expression: &str, data: &mut GameData) -> Option<bool> {

    update_dices(instance_index, data);

    let engine = Engine::new();

    let r = engine.eval_expression_with_scope::<bool>(&mut  data.scopes[instance_index], expression);
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

    let engine = Engine::new();
    let r = engine.eval_expression_with_scope::<Dynamic>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {
        let nn = r.unwrap().clone();
        if let Some(n) = nn.as_float().ok() {
            return Some(n);
        }
        if let Some(n) = nn.as_int().ok() {
            return Some(n as f64);
        }
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a dynamic script in the given instance.
pub fn eval_dynamic_script_instance(instance_index: usize, id: (usize, usize), expression: &str, data: &mut GameData) -> bool {

    if data.runs_in_editor {
        return eval_dynamic_expression_instance_editor(instance_index, id, expression, data);
    }

    let engine = Engine::new();
    update_dices(instance_index, data);

    let r = engine.eval_with_scope::<Dynamic>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {
        return true
    } else {
        println!("{:?}", r);
    }

    false
}

/// Evaluates a numerical expression in the given instance in the editor. We have to send the editor the variables which have been updated for visual disolay.
pub fn eval_dynamic_expression_instance_editor(instance_index: usize, id: (usize, usize), expression: &str, data: &mut GameData) -> bool {

    let mut engine = Engine::new();
    update_dices(instance_index, data);

    // Add indexer
    if let Some(target_index) = data.instances[instance_index].target {
        engine.register_type::<InstanceVariables>()
            .register_fn("new_instance", InstanceVariables::new)
            .register_indexer_get(InstanceVariables::get_number)
            .register_indexer_set(InstanceVariables::set_number);

        let target_result = engine.eval::<InstanceVariables>(
        "
            let target = new_instance();
            target
        ");

        // Add target variables
        if let Some(mut target) = target_result.ok() {
            let original_target = data.scopes[target_index].clone();

            if let Some(behavior) = data.get_mut_behavior(data.instances[target_index].behavior_id, BehaviorType::Behaviors) {
                for (_index, node) in &behavior.data.nodes {
                    if node.behavior_type == BehaviorNodeType::VariableNumber {

                        if let Some(v) = original_target.get_value::<f64>(node.name.as_str()) {
                            target.numbers.insert(node.name.clone(), v);
                        }
                    }
                }
            }

            data.scopes[instance_index].set_value("target", target);
        }
    }

    let original = data.scopes[instance_index].clone();

    let r = engine.eval_with_scope::<Dynamic>(&mut data.scopes[instance_index], expression);
    if r.is_ok() {

        // Read out the target variables and if changed apply them
        if let Some(target_index) = data.instances[instance_index].target {
            if let Some(target) = data.scopes[instance_index].get_value::<InstanceVariables>("target") {
                if let Some(behavior) = data.behaviors.get_mut(&data.instances[target_index].behavior_id) {
                    for (_index, node) in &behavior.data.nodes {
                        if node.behavior_type == BehaviorNodeType::VariableNumber {

                            let o = data.scopes[target_index].get_value::<f64>(node.name.as_str());
                            let n = target.numbers.get(&node.name);

                            if n.is_some() && o.is_some() && o.unwrap() != *n.unwrap() {
                                let value = n.unwrap().clone();
                                data.scopes[target_index].set_value(node.name.clone(), value);
                                data.changed_variables.push((target_index, behavior.data.id, node.id, value));
                            }
                        }
                    }
                }
            }
        }

        //

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

    let r = engine.eval_expression_with_scope::<Dynamic>(&mut scope, expression);

    if r.is_ok() {
        let nn = r.unwrap().clone();
        if let Some(n) = nn.as_float().ok() {
            return Some(n);
        }
        if let Some(n) = nn.as_int().ok() {
            return Some(n as f64);
        }
    } else {
        println!("{:?}", r);
    }

    None
}

/// Evaluates a boolean expression for the given behavior.
/// This is used to verify an expression in the expression editor and not used in game (which would be instance based).
pub fn eval_dynamic_script_behavior(expression: &str, behavior_id: usize, data: &mut GameData) -> bool {

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