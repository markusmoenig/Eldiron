
use crate::gamedata::behavior:: { BehaviorNode, BehaviorInstance };

#[derive(PartialEq)]
pub enum NodeResult {
    Ok,
    Failure,
    Success,
    InProgress,
}

pub fn dice_check(_inst: &mut BehaviorInstance, _node: &BehaviorNode) -> NodeResult {
    println!("inside check");
    NodeResult::Failure
}

pub fn expression(_inst: &mut BehaviorInstance, _node: &BehaviorNode) -> NodeResult {
    println!("inside expression");
    NodeResult::Failure
}

pub fn say(_inst: &mut BehaviorInstance, _node: &BehaviorNode) -> NodeResult {
    println!("inside say");
    NodeResult::Ok
}