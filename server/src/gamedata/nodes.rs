
use crate::gamedata::behavior:: { BehaviorNodeConnector, BehaviorInstance, BehaviorNodeType };
use crate::gamedata::GameData;
use evalexpr::*;


/// expression
pub fn expression(_inst: &mut BehaviorInstance, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {

        // Insert the variables
        let mut cont = HashMapContext::new();
        for n in &behavior.data.nodes {
            if n.1.behavior_type == BehaviorNodeType::VariableNumber {
                let t = format!("{} = {}", n.1.name, n.1.values.get("value").unwrap().0);
                let _ = eval_empty_with_context_mut(t.as_str(), &mut cont);
            }
        }

        // Evaluate the expression
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            let exp = eval_boolean_with_context(&node.values.get("expression").unwrap().4, &cont);
            if exp.is_ok() {
                if exp == Ok(true) {
                    return BehaviorNodeConnector::Success;
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// say
pub fn say(_inst: &mut BehaviorInstance, id: (usize, usize), data: &mut GameData) -> BehaviorNodeConnector {
    if let Some(behavior) = data.behaviors.get_mut(&id.0) {
        if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
            if let Some(value) = node.values.get("text") {
                println!("{}", value.4);
            }
        }
    }
    BehaviorNodeConnector::Bottom
}