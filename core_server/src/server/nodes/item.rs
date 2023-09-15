extern crate ref_thread_local;
use crate::prelude::*;
use ref_thread_local::RefThreadLocal;

/// Light
pub fn node_light_item(
    id: (Uuid, Uuid),
    nodes: &mut FxHashMap<Uuid, GameBehaviorData>,
) -> BehaviorNodeConnector {
    let mut state_value = 0;
    if let Some(value) = get_node_integer(id, "state", nodes) {
        state_value = value;
    }

    let mut state = STATE.borrow_mut();

    if state_value == 1 {
        state.light = Some(LightData {
            light_type: LightType::PointLight,
            position: (0, 0),
            intensity: 255,
        });
    } else {
        state.light = None;
    }

    BehaviorNodeConnector::Bottom
}

/// Tile
pub fn node_set_item_tile(
    id: (Uuid, Uuid),
    nodes: &mut FxHashMap<Uuid, GameBehaviorData>,
) -> BehaviorNodeConnector {
    if let Some(value) = get_node_value2(id, "tile", nodes) {
        if let Some(tile) = value.to_tile_data() {
            let mut state = STATE.borrow_mut();
            state.tile = Some(tile);
        } else {
            let mut state = STATE.borrow_mut();
            state.tile = None;
        }
    }
    BehaviorNodeConnector::Bottom
}
