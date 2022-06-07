

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone)]
pub struct CharacterData {
    pub name            : String,
    pub id              : usize,
    pub position        : (usize, isize, isize),
    pub tile            : (usize, usize, usize),
}
