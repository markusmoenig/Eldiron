use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };

/// Inside Area
pub fn inside_area(_instance_index: usize, _id: (usize, usize), _data: &mut GameData, _behavior_type: BehaviorType) -> BehaviorNodeConnector {
    println!("here");
    BehaviorNodeConnector::Right
}