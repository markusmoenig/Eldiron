
use rhai::{ Dynamic };
use std::{collections::HashMap};

use super::behavior::{ BehaviorNodeType, BehaviorType };
use crate::gamedata::*;
use regex::bytes::Regex;

#[derive(Debug, Clone)]
struct InstanceVariables {
    pub numbers: HashMap<String, f64>
}

impl InstanceVariables {
    fn get_number(&mut self, index: String) -> f64 {
        if let Some(value) = self.numbers.get(&index) {
            return *value;
        }
        0.0
    }
    fn set_number(&mut self, index: String, value: f64) {
        if self.numbers.contains_key(&index) {
            self.numbers.insert(index, value);
        }
    }

    fn new() -> Self {
        Self { numbers: HashMap::new() }
    }
}

/// Adds the given target variables to the scope
pub fn add_target_to_scope(instance_index: usize, data: &mut GameData) {
    if let Some(target_index) = data.instances[instance_index].target_instance_index {
        data.engine.register_type::<InstanceVariables>()
            .register_fn("new_instance", InstanceVariables::new)
            .register_indexer_get(InstanceVariables::get_number)
            .register_indexer_set(InstanceVariables::set_number);


        let original_target = data.scopes[target_index].clone();
        let mut target = InstanceVariables::new();

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

/// Read out the target variables and if changed apply them
pub fn apply_scope_to_target(instance_index: usize, data: &mut GameData) {
    if let Some(target_index) = data.instances[instance_index].target_instance_index {
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
}

/// Replace the target strings, only called once before compilation for each script
pub fn replace_target_variables(input: String) -> String {
    let output = input.clone();
    if input.contains("${Target}") {
        if let Some(re) = Regex::new(r"\$\{Target\}.(?P<v>\w*)").ok() {
            let t = re.replace_all(output.as_bytes(), "target[\"$v\"]".as_bytes());
            if let Some(tt) = std::str::from_utf8(t.as_ref()).ok() {
                return tt.to_string();
            }
        }
    }
    output
}

/// Evaluates a boolean expression in the given instance.
pub fn eval_bool_expression_instance(instance_index: usize, id: (BehaviorType, usize, usize, String), data: &mut GameData) -> Option<bool> {
    add_target_to_scope(instance_index, data);

    if let Some(ast) = data.ast.get(&id) {
        let r = data.engine.eval_ast_with_scope(&mut  data.scopes[instance_index], ast);
        if r.is_ok() {
            return Some(r.unwrap());
        } else {
            println!("{:?}", r);
        }
    } else {
        if let Some(value) = get_node_value((id.1, id.2, &id.3), data, id.0) {
            let script = replace_target_variables(value.4);
            if let Some(ast) = data.engine.compile_expression_with_scope(&mut  data.scopes[instance_index], script.as_str()).ok() {
                let r = data.engine.eval_ast_with_scope(&mut  data.scopes[instance_index], &ast);
                if r.is_ok() {
                    data.ast.insert(id.clone(), ast);
                    return Some(r.unwrap());
                } else {
                    println!("{:?}", r);
                }
            }
        }
    }

    None
}

/// Evaluates a numerical expression in the given instance.
pub fn eval_number_expression_instance(instance_index: usize, id: (BehaviorType, usize, usize, String), data: &mut GameData) -> Option<f64> {
    add_target_to_scope(instance_index, data);

    if let Some(ast) = data.ast.get(&id) {
        let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut  data.scopes[instance_index], ast);
        if r.is_ok() {
            let nn = r.unwrap();
            if let Some(n) = nn.as_float().ok() {
                return Some(n);
            }
            if let Some(n) = nn.as_int().ok() {
                return Some(n as f64);
            }
        } else {
            println!("{:?}", r);
        }
    } else {
        if let Some(value) = get_node_value((id.1, id.2, &id.3), data, id.0) {
            let script = replace_target_variables(value.4);
            if let Some(ast) = data.engine.compile_expression_with_scope(&mut  data.scopes[instance_index], script.as_str()).ok() {
                let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut  data.scopes[instance_index], &ast);
                if r.is_ok() {
                    data.ast.insert(id.clone(), ast);
                    let nn = r.unwrap();
                    if let Some(n) = nn.as_float().ok() {
                        return Some(n);
                    }
                    if let Some(n) = nn.as_int().ok() {
                        return Some(n as f64);
                    }
                } else {
                    println!("{:?}", r);
                }
            }
        }
    }

    None
}

/// Evaluates a dynamic script in the given instance.
pub fn eval_dynamic_script_instance(instance_index: usize, id: (BehaviorType, usize, usize, String), data: &mut GameData) -> bool {

    if data.runs_in_editor {
        return eval_dynamic_expression_instance_editor(instance_index, id, data);
    }

    add_target_to_scope(instance_index, data);

    if let Some(ast) = data.ast.get(&id) {
        let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut  data.scopes[instance_index], ast);
        if r.is_ok() {
            apply_scope_to_target(instance_index, data);
            return true
        } else {
            println!("{:?}", r);
        }
    } else {
        if let Some(value) = get_node_value((id.1, id.2, &id.3), data, id.0) {
            let script = replace_target_variables(value.4);
            if let Some(ast) = data.engine.compile_with_scope(&mut  data.scopes[instance_index], script.as_str()).ok() {
                let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut  data.scopes[instance_index], &ast);
                if r.is_ok() {
                    data.ast.insert(id.clone(), ast);
                    apply_scope_to_target(instance_index, data);
                    return true
                } else {
                    println!("{:?}", r);
                }
            }
        }
    }

    false
}

/// Evaluates a dynamic script in the given instance.
/// We have to send the editor the variables which have been updated for visual display.
pub fn eval_dynamic_expression_instance_editor(instance_index: usize, id: (BehaviorType, usize, usize, String), data: &mut GameData) -> bool {
    add_target_to_scope(instance_index, data);

    let original = data.scopes[instance_index].clone();

    if let Some(ast) = data.ast.get(&id) {
        let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut  data.scopes[instance_index], ast);
        if r.is_ok() {
            apply_scope_to_target(instance_index, data);

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
                    if let Some(behavior) = data.behaviors.get_mut(&id.1) {

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
    } else {
        if let Some(value) = get_node_value((id.1, id.2, &id.3), data, id.0) {
            let script = replace_target_variables(value.4);
            if let Some(ast) = data.engine.compile_with_scope(&mut data.scopes[instance_index], script.as_str()).ok() {
                let r = data.engine.eval_ast_with_scope::<Dynamic>(&mut data.scopes[instance_index], &ast);
                if r.is_ok() {

                    data.ast.insert(id.clone(), ast);
                    apply_scope_to_target(instance_index, data);

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
                            if let Some(behavior) = data.behaviors.get_mut(&id.1) {

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
            } else {
                println!("failed to compile {}", script);
            }
        }
    }
    false
}
