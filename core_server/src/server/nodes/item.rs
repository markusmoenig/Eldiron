use crate::prelude::*;

/// Light
pub fn light_item(_instance_index: usize, id: (Uuid, Uuid), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector {
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
    }
    BehaviorNodeConnector::Bottom
}