
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{regiondata::GameRegionData, characterdata::CharacterData, asset::TileUsage};

#[derive(Serialize, Deserialize)]
pub struct GameUpdate {

    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,

    pub region                  : Option<GameRegionData>,
    #[serde(with = "vectorize")]
    pub displacements           : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,

    pub characters              : Vec<CharacterData>,
}