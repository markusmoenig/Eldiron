use crate::prelude::*;

/// Screen
pub fn screen(_instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    data.instances[data.curr_player_inst_index].curr_player_widgets = vec![];

    if let Some(curr_screen_id) = &data.instances[data.curr_player_inst_index].curr_player_screen_id {
        if *curr_screen_id == id.1 {
            return BehaviorNodeConnector::Bottom;
        }
    }

    data.instances[data.curr_player_inst_index].curr_player_screen_id = Some(id.1);

    if let Some(value) = get_node_value((id.0, id.1, &"script".to_owned()), data, behavior_type) {
        if let Some(script) = value.to_string() {
            data.instances[data.curr_player_inst_index].curr_player_screen = script;
        }
    }

    BehaviorNodeConnector::Bottom
}

/// Widget
pub fn widget(_instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(value) = get_node_value((id.0, id.1, &"script".to_owned()), data, behavior_type) {
        if let Some(script) = value.to_string() {
            data.instances[data.curr_player_inst_index].curr_player_widgets.push(script);
        }
    }

    BehaviorNodeConnector::Bottom
}