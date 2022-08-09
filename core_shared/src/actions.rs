use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum PlayerActions {
    Login,
    Signup,
    Move,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum PlayerDirection {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PlayerAction {
    pub action                  : String,
    pub direction               : PlayerDirection,
    pub player_id               : Uuid,
    pub text                    : String,
}

/// Packs the given action into JSON
pub fn pack_action(player_id: Uuid, action: String, direction: PlayerDirection, text: String) -> Option<String> {
    let action = PlayerAction{ action, player_id, direction, text };
    return serde_json::to_string(&action).ok()
}