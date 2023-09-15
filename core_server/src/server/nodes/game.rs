extern crate ref_thread_local;
use crate::prelude::*;
use ref_thread_local::RefThreadLocal;

/// Screen
pub fn node_screen(
    id: (Uuid, Uuid),
    nodes: &mut FxHashMap<Uuid, GameBehaviorData>,
) -> BehaviorNodeConnector {
    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

    if let Some(screen_name) = get_node_string(id, "script_name", nodes) {
        if screen_name != data.character_instances[data.curr_player_inst_index].curr_player_script {
            data.character_instances[data.curr_player_inst_index].new_player_script =
                Some(screen_name);
        }
    }

    BehaviorNodeConnector::Bottom
}
