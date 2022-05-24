use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };
//use crate::gamedata::get_node_value;
//use crate::asset::TileUsage;

/// Inside Area
pub fn screen(_behavior_id: usize, _id: (usize, usize), _data: &mut GameData, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    BehaviorNodeConnector::Fail
}