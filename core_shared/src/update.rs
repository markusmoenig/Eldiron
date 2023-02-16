use crate::prelude::*;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameUpdate {

    pub id                      : Uuid,

    pub screen_size             : (i32, i32),
    pub def_square_tile_size    : i32,

    pub position                : Option<Position>,
    pub old_position            : Option<Position>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : Option<TileId>,

    /// The script for the current screen which handles the drawing
    pub screen                  : Option<String>,

    /// The widget scripts for the current screen
    pub widgets                 : Vec<String>,

    /// A region
    pub region                  : Option<GameRegionData>,

    /// Current lights in the region
    pub lights                  : Vec<LightData>,

    /// Tile displacements for the region
    #[serde(with = "vectorize")]
    pub displacements           : HashMap<(isize, isize), TileData>,

    /// Character information
    pub characters              : Vec<CharacterData>,

    /// Loot information
    #[serde(with = "vectorize")]
    pub loot                    : FxHashMap<(isize, isize), Vec<LootData>>,

    /// Messages
    pub messages                : Vec<MessageData>,

    /// Audio files to play
    pub audio                   : Vec<String>,

    /// Scope
    pub scope_buffer            : ScopeBuffer,

    /// Inventory
    pub inventory               : Inventory,

    /// Gear
    pub gear                    : Gear,

    /// Weapons
    pub weapons                 : Weapons,

    /// Skills
    pub skills                  : Skills,

    /// Experience
    pub experience              : Experience,

    /// Multiple Choice Data
    pub multi_choice_data       : Vec<MultiChoiceData>,

    /// Ongoing communications
    pub communication           : Vec<PlayerCommunication>
}

impl GameUpdate {

    pub fn new() -> Self {

        Self {
            id                  : Uuid::new_v4(),
            screen_size         : (1024, 608),
            def_square_tile_size: 32,
            position            : None,
            old_position        : None,
            max_transition_time : 0,
            curr_transition_time: 0,
            tile                : None,
            screen              : None,
            widgets             : vec![],
            region              : None,
            lights              : vec![],
            displacements       : HashMap::new(),
            characters          : vec![],
            loot                : FxHashMap::default(),
            messages            : vec![],
            audio               : vec![],
            scope_buffer        : ScopeBuffer::new(),
            inventory           : Inventory::new(),
            gear                : Gear::new(),
            weapons             : Weapons::new(),
            skills              : Skills::new(),
            experience          : Experience::new(),
            multi_choice_data   : vec![],
            communication       : vec![],

        }
    }
}