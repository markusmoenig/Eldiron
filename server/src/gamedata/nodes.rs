
use crate::gamedata::behavior:: { BehaviorNode, BehaviorInstance };

#[derive(PartialEq)]
pub enum NodeResult {
    Fail,
    Success,
    InProgress,
}

pub fn dice_roll(_inst: &mut BehaviorInstance, _node: &BehaviorNode) -> NodeResult {
    println!("inside dice_roll");
    NodeResult::Fail
}