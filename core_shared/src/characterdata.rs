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
}

/// Represents a placed loot instance in the region
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LootData {
    pub id                      : Uuid,
    pub name                    : Option<String>,
    pub tile                    : Option<TileData>,
    pub state                   : Option<ScopeBuffer>,
    pub light                   : Option<LightData>,
    pub amount                  : i32,
    pub stackable               : i32,
    pub static_item             : bool,
}
