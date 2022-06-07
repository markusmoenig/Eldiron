
use serde::{Deserialize, Serialize};

use crate::{regiondata::GameRegionData, characterdata::CharacterData};

#[derive(Serialize, Deserialize)]
pub struct GameUpdate {

    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,

    pub region                  : Option<GameRegionData>,
    pub characters              : Vec<CharacterData>,
}