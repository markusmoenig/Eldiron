extern crate ref_thread_local;
use ref_thread_local::{RefThreadLocal};
use crate::prelude::*;

/// Light
pub fn node_light_item(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let mut state_value = 0;
    if let Some(value) = get_node_integer(id, "state", nodes) {
        state_value = value;
    }

    let mut state = STATE.borrow_mut();

    if state_value == 1 {
        state.light = Some(LightData {
                light_type              : LightType::PointLight,
                position                : (0, 0),
                intensity               : 255,
        });
    } else {
        state.light = None;
    }

    BehaviorNodeConnector::Bottom
}

/// Tile
pub fn node_set_item_tile(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
    let mut tile : Option<TileData> = None;

        if let Some(value) = get_node_value2(id, "tile", nodes) {
        tile = value.to_tile_data();
    }

    // TODO if let Some(tile) = tile {
    //     if let Some(curr_loot_item) = &data.curr_loot_item {
    //         if let Some(loot) = data.loot.get_mut(&(curr_loot_item.0, curr_loot_item.1)) {
    //             loot[curr_loot_item.2].tile = Some(tile);
    //         }
    //     } else
    //     if let Some(inventory_index) = &data.curr_inventory_index {
    //         if let Some(mess) = data.curr_player_scope.get_mut("inventory") {
    //             if let Some(mut inv) = mess.write_lock::<Inventory>() {
    //                 inv.items[*inventory_index].tile = Some(tile);
    //             }
    //         }
    //     }
    // }

    BehaviorNodeConnector::Bottom
}
