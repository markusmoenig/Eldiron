
use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;

use super::behavior::{ BehaviorType };
use super::nodes_utility::get_node_value;

/// Screen
pub fn screen(_instance_index: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if let Some(curr_screen_id) = &data.instances[data.curr_player_inst_index].curr_player_screen_id {
        if *curr_screen_id == id.1 {
            return BehaviorNodeConnector::Bottom;
        }
    }

    data.instances[data.curr_player_inst_index].curr_player_screen_id = Some(id.1);

    if let Some(value) = get_node_value((id.0, id.1, &"script".to_owned()), data, behavior_type) {
        data.instances[data.curr_player_inst_index].curr_player_screen = value.4.clone();
    }

    /*
    if data.custom_scopes.contains_key(&id.1) == false {
        let mut scope = Scope::new();

        scope.set_value("messages", ScriptMessages::new());
        scope.set_value("draw", ScriptDraw::new());
        scope.set_value("background", ScriptRGB::new(0, 0, 0));

        scope.set_value("width", data.game_screen_width as i64);
        scope.set_value("height", data.game_screen_height as i64);
        scope.set_value("tile_size", 32 as i64);

        data.custom_scopes.insert(id.1, scope);
        data.custom_scopes_ordered.push(id.1);
    }

    _ = eval_dynamic_script_instance_for_custom_scope(instance_index, (behavior_type, id.0, id.1, "script".to_string()), data, id.1);

    if let Some(scope) = data.custom_scopes.get(&id.1) {
        if let Some(width) = scope.get_value::<i64>("width") {
            data.game_screen_width = width as usize;
        }
        if let Some(height) = scope.get_value::<i64>("height") {
            data.game_screen_height = height as usize;
        }
        if let Some(tile_size) = scope.get_value::<i64>("tile_size") {
            data.game_screen_tile_size = tile_size as usize;
        }
    }

    // Create the screen
    if data.game_screens.contains_key(&id.1) == false {
        if let Some(value) = get_node_value((id.0, id.1, "layout"), data, behavior_type) {
            let game_screen = serde_json::from_str(&value.4.clone())
                .unwrap_or(GameScreen::new() );
            data.game_screens.insert(id.1, game_screen);
        }
    }

    // Draw it
    if let Some(mut screen) = data.game_screens.remove(&id.1) {
        screen.draw(id.1, false, data);
        data.game_screens.insert(id.1, screen);
    }
    */

    BehaviorNodeConnector::Bottom
}
