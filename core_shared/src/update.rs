use crate::prelude::*;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GameUpdate {

    pub id                      : Uuid,

    pub position                : Option<(usize, isize, isize)>,
    pub old_position            : Option<(usize, isize, isize)>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : Option<(usize, usize, usize)>,

    /// The script for the current screen which handles the drawing
    pub screen                  : Option<String>,

    /// A region
    pub region                  : Option<GameRegionData>,

    /// Current lights in the region
    pub lights                  : Vec<Light>,

    /// Tile displacements for the region
    #[serde(with = "vectorize")]
    pub displacements           : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,

    /// Character information
    pub characters              : Vec<CharacterData>,

    /// Messages
    pub messages                : Vec<MessageData>,

    /// Audio files to play
    pub audio                   : Vec<String>,
}

impl GameUpdate {

    pub fn new() -> Self {

        Self {
            id                  : Uuid::new_v4(),
            position            : None,
            old_position        : None,
            max_transition_time : 0,
            curr_transition_time: 0,
            tile                : None,
            screen              : None,
            region              : None,
            lights              : vec![],
            displacements       : HashMap::new(),
            characters          : vec![],
            messages            : vec![],
            audio               : vec![],
        }
    }
}