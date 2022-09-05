use crate::prelude::*;
/*
/// Screen
pub fn screen(_instance_index: usize, id: (usize, usize), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(curr_screen_id) = &data.instances[data.curr_player_inst_index].curr_player_screen_id {
        if *curr_screen_id == id.1 {
            return BehaviorNodeConnector::Bottom;
        }
    }

    data.instances[data.curr_player_inst_index].curr_player_screen_id = Some(id.1);

    if let Some(value) = get_node_value((id.0, id.1, &"script".to_owned()), data, behavior_type) {
        data.instances[data.curr_player_inst_index].curr_player_screen = value.4.clone();
    }

    BehaviorNodeConnector::Bottom
}
*/