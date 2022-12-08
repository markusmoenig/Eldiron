use crate::prelude::*;

/// Light
pub fn light_item(instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    let mut state = 0;
    if let Some(value) = get_node_value((id.0, id.1, "state"), data, behavior_type) {
        if let Some(index) = value.to_integer() {
            state = index;
        }
    }

    if let Some(curr_loot_item) = &data.curr_loot_item {
        if let Some(loot) = data.loot.get_mut(&(curr_loot_item.0, curr_loot_item.1)) {
            if state == 1 {
                loot[curr_loot_item.2].light = Some(LightData {
                    light_type              : LightType::PointLight,
                    position                : (curr_loot_item.0, curr_loot_item.1),
                    intensity               : 1,
                });
            } else {
                loot[curr_loot_item.2].light = None;
            }
        }
    } else
    if let Some(inventory_index) = &data.curr_inventory_index {
        if let Some(position) = &data.instances[instance_index].position {
            if let Some(mess) = data.curr_player_scope.get_mut("inventory") {
                if let Some(mut inv) = mess.write_lock::<Inventory>() {
                    if state == 1 {
                        inv.items[*inventory_index].light = Some(LightData {
                            light_type              : LightType::PointLight,
                            position                : (position.x, position.y),
                            intensity               : 1,
                        });
                    } else {
                        inv.items[*inventory_index].light = None;
                    }
                }
            }
        }
    }

    BehaviorNodeConnector::Bottom
}

/// Tile
pub fn set_item_tile(_instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
    let mut tile : Option<TileData> = None;
    if let Some(value) = get_node_value((id.0, id.1, "tile"), data, behavior_type) {
        tile = value.to_tile_data();
    }

    if let Some(tile) = tile {
        if let Some(curr_loot_item) = &data.curr_loot_item {
            if let Some(loot) = data.loot.get_mut(&(curr_loot_item.0, curr_loot_item.1)) {
                loot[curr_loot_item.2].tile = Some(tile);
            }
        } else
        if let Some(inventory_index) = &data.curr_inventory_index {
            if let Some(mess) = data.curr_player_scope.get_mut("inventory") {
                if let Some(mut inv) = mess.write_lock::<Inventory>() {
                    inv.items[*inventory_index].tile = Some(tile);
                }
            }
        }
    }

    BehaviorNodeConnector::Bottom
}
