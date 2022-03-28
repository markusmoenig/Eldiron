use crate::gamedata::behavior:: { BehaviorNodeType };
use crate::gamedata::GameData;
use evalexpr::*;
use rand::prelude::*;

pub fn get_value(id: (usize, usize, &str), data: &mut GameData) -> Option<(f64, f64, f64, f64, String)> {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get_mut(id.2) {
                return Some(value.clone());
            }
        }
    }
    None
}

pub fn set_value(id: (usize, usize, &str), data: &mut GameData, value: (f64, f64, f64, f64, String)) {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(v) = node.values.get_mut(id.2) {
                *v = value;
            }
        }
    }
}

pub fn eval_expression_as_number(id: (usize, usize), data: &mut GameData, value_id: &str, default: f64) -> f64 {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {

        // Insert the variables
        let mut cont = HashMapContext::new();
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                let t = format!("{} = {}", n.1.name, n.1.values.get("value").unwrap().0);
                let _ = eval_empty_with_context_mut(t.as_str(), &mut cont);
            }
        }

        // d1 - d2
        let mut rng = thread_rng();
        for d in (2..=20).step_by(2) {
            let random = rng.gen_range(1..=d);
            let t = format!("{} = {}", format!("d{}", d), random);
            let _ = eval_empty_with_context_mut(t.as_str(), &mut cont);
        }

        // Evaluate the expression as a number
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            let exp = eval_with_context(&node.values.get(value_id).unwrap().4, &cont);
            if exp.is_ok() {
                let rc = exp.unwrap().as_number();
                if rc.is_ok() {
                    return rc.unwrap();
                }
            }
            /*
            return match exp {
                Ok(v) => {
                    match v.as_number() {
                        Ok(fv) => {
                            Some(fv)
                        },
                        Err(e) => None
                    }
                },
                Err(e) => None
            }*/
        }
    }
    default
}