
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{regiondata::GameRegionData, characterdata::CharacterData, asset::TileUsage, message::MessageData};

#[derive(Serialize, Deserialize)]
pub struct GameUpdate {

    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,

    /// The script for the current screen which handles the drawing
    pub screen                  : Option<String>,

    /// A region
    pub region                  : Option<GameRegionData>,

    /// Tile displacements for the region
    #[serde(with = "vectorize")]
    pub displacements           : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,

    /// Character information
    pub characters              : Vec<CharacterData>,

    /// Messages
    pub messages                : Vec<MessageData>,
}

impl GameUpdate {

    pub fn new() -> Self {

        Self {
            position            : None,
            tile                : None,
            screen              : None,
            region              : None,
            displacements       : HashMap::new(),
            characters          : vec![],
            messages            : vec![]
        }
    }
}