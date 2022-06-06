use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };
use crate::gamedata::get_node_value;
use crate::asset::TileUsage;

/// Inside Area
pub fn inside_area(_behavior_id: usize, id: (usize, usize), data: &mut GameData, _behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(region) = data.regions.get_mut(&data.curr_region_id) {
        for instance_index in &data.active_instance_indices {
            if let Some(position) = data.instances[*instance_index].position {
                if region.data.areas[id.0].area.contains(&(position.1, position.2)) {
                    println!("{} is in area {}", data.instances[*instance_index].name, region.data.areas[id.0].name);
                    return BehaviorNodeConnector::Right;
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}

/// Displace Tiles
pub fn displace_tiles(_behavior_id: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(value) = get_node_value((id.0, id.1, "tile"), data, behavior_type) {
        let tile_id = (value.0 as usize, value.1 as usize, value.2 as usize, TileUsage::Environment);

        // Filter based ?
        if let Some(value) = get_node_value((id.0, id.1, "filter"), data, behavior_type) {
            let filter_id = (value.0 as usize, value.1 as usize, value.2 as usize);

            if let Some(region) = data.regions.get_mut(&data.curr_region_id) {

                for (x,y) in &region.data.areas[id.0].area {
                    let tiles = region.get_value_without_displacements((*x, *y));

                    for tile in tiles {
                        if tile.0 == filter_id.0 && tile.1 == filter_id.1 && tile.2 == filter_id.2 {
                            region.displacements.insert((*x, *y), tile_id.clone());
                        }
                    }
                }
            }
        } else {
            // No filter, displace all
            if let Some(region) = data.regions.get_mut(&data.curr_region_id) {
                for (x,y) in &region.data.areas[id.0].area {
                    region.displacements.insert((*x, *y), tile_id.clone());
                }
            }
        }
    }

    BehaviorNodeConnector::Fail
}