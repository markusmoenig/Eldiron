use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum PlayerDirection {
    None,
    North,
    East,
    South,
    West,
    Up,
    Down,
    Coordinate
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PlayerAction {
    pub action                  : String,
    pub direction               : PlayerDirection,
    pub player_id               : Uuid,
    pub gear_index              : Option<u16>,
    pub inventory_index         : Option<u16>,
    pub multi_choice_uuid       : Option<Uuid>,
    pub coordinate              : Option<(isize, isize)>,
    pub spell                   : Option<String>,
}

/// Packs the given action into JSON
pub fn pack_action(player_id: Uuid, action: String, direction: PlayerDirection, spell: Option<String>) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction, gear_index: None, inventory_index: None, multi_choice_uuid: None, coordinate: None, spell };
    return serde_json::to_string(&action).ok()
}

/// Packs the given coordinate based action into JSON
pub fn pack_action_coordinate(player_id: Uuid, action: String, coordinate: (isize, isize), spell: Option<String>) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction : PlayerDirection::Coordinate, gear_index: None, inventory_index: None, multi_choice_uuid: None, coordinate: Some(coordinate), spell };
    return serde_json::to_string(&action).ok()
}

/// Packs an action on the given inventory index into JSON
pub fn pack_inventory_action(player_id: Uuid, action: String, inv_index: u16) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction: PlayerDirection::None, gear_index: None, inventory_index: Some(inv_index), multi_choice_uuid: None, coordinate: None, spell: None };
    return serde_json::to_string(&action).ok()
}

/// Packs an action on the given inventory index into JSON
pub fn pack_gear_action(player_id: Uuid, action: String, gear_index: u16) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction: PlayerDirection::None, gear_index: Some(gear_index), inventory_index: None, multi_choice_uuid: None, coordinate: None, spell: None };
    return serde_json::to_string(&action).ok()
}

/// Packs an action for the given answer
pub fn pack_multi_choice_answer_action(player_id: Uuid, action: String, multi_choice_uuid: Uuid) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction: PlayerDirection::None, gear_index: None, inventory_index: None, multi_choice_uuid: Some(multi_choice_uuid), coordinate: None, spell: None };
    return serde_json::to_string(&action).ok()
}