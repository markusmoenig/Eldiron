use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };
//use crate::gamedata::get_node_value;
//use crate::asset::TileUsage;

//use crate::gamedata::nodes_utility::*;
use crate::gamedata::script::*;

/// Inside Area
pub fn screen(instance_index: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    _ = eval_dynamic_script_instance(instance_index, (behavior_type, id.0, id.1, "script".to_string()), data);

    BehaviorNodeConnector::Bottom
}