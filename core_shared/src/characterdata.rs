use crate::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterData {
    pub name                    : String,
    pub id                      : Uuid,
    pub index                   : usize,

    pub position                : Position,
    pub old_position            : Option<Position>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : TileId,
    pub effects                 : Vec<TileId>
}

