extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;
use crate::prelude::*;

/// Screen
pub fn node_screen(id: (Uuid, Uuid), nodes: &mut FxHashMap<Uuid, GameBehaviorData>) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(curr_screen_id) = &data.character_instances[data.curr_player_inst_index].curr_player_screen_id {
        if *curr_screen_id == id.1 {
            return BehaviorNodeConnector::Bottom;
        }
    }

    data.character_instances[data.curr_player_inst_index].curr_player_screen_id = Some(id.1);

    if let Some(value) = get_node_string(id, "script_name", nodes) {
        data.character_instances[data.curr_player_inst_index].curr_player_screen = value;
    }

    BehaviorNodeConnector::Bottom
}

