
use crate::gamedata::behavior:: { BehaviorNode, BehaviorInstance };

#[derive(PartialEq)]
pub enum NodeResult {
    Failure,
    Success,
    InProgress,
}

pub fn dice_check(_inst: &mut BehaviorInstance, _node: &BehaviorNode) -> NodeResult {
    println!("inside check");
    NodeResult::Failure
}