use rhai::Scope;

use crate::gamedata::behavior::{ BehaviorNodeConnector };
use crate::gamedata::GameData;
use crate::script_types::*;
//use crate::gamedata::nodes_utility::get_node_name;

use super::behavior::{ BehaviorType };
//use crate::gamedata::get_node_value;
//use crate::asset::TileUsage;

//use crate::gamedata::nodes_utility::*;
use crate::gamedata::script::*;

/// Screen
pub fn screen(instance_index: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if data.custom_scopes.contains_key(&id.1) == false {
        let mut scope = Scope::new();

        scope.set_value("messages", ScriptMessages::new());
        scope.set_value("draw", ScriptDraw::new());

        scope.set_value("x", 0 as i64);
        scope.set_value("y", 0 as i64);
        scope.set_value("width", data.game_screen_width as i64);
        scope.set_value("height", data.game_screen_height as i64);
        scope.set_value("screen_width", data.game_screen_width as i64);
        scope.set_value("screen_height", data.game_screen_height as i64);

        data.custom_scopes.insert(id.1, scope);
        data.custom_scopes_ordered.push(id.1);
    }

    _ = eval_dynamic_script_instance_for_custom_scope(instance_index, (behavior_type, id.0, id.1, "script".to_string()), data, id.1);
    BehaviorNodeConnector::Bottom
}

/// Widget
pub fn widget(instance_index: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> BehaviorNodeConnector {

    if data.custom_scopes.contains_key(&id.1) == false {
        let mut scope = Scope::new();

        scope.set_value("messages", ScriptMessages::new());
        scope.set_value("draw", ScriptDraw::new());

        scope.set_value("x", 0 as i64);
        scope.set_value("y", 0 as i64);
        scope.set_value("width", data.game_screen_width as i64);
        scope.set_value("height", data.game_screen_height as i64);
        scope.set_value("screen_width", data.game_screen_width as i64);
        scope.set_value("screen_height", data.game_screen_height as i64);

        data.custom_scopes.insert(id.1, scope);
        data.custom_scopes_ordered.push(id.1);
    }

    _ = eval_dynamic_script_instance_for_custom_scope(instance_index, (behavior_type, id.0, id.1, "script".to_string()), data, id.1);
    BehaviorNodeConnector::Bottom
}