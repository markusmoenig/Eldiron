use crate::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterData {
    pub name                    : String,
    pub id                      : Uuid,
    pub index                   : usize,

    pub position                : (usize, isize, isize),
    pub old_position            : Option<(usize, isize, isize)>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : (usize, usize, usize),
}
