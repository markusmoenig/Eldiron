use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };

/// Inside Area
pub fn inside_area(_behavior_id: usize, id: (usize, usize), data: &mut GameData, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(region) = data.regions.get_mut(&data.curr_region_id) {
        for instance_index in &data.active_instance_indices {
            if let Some(position) = data.instances[*instance_index].position {
                if region.data.areas[id.0].area.contains(&(position.1, position.2)) {
                    println!("{} is in area", data.instances[*instance_index].name);
                    return BehaviorNodeConnector::Right;
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}