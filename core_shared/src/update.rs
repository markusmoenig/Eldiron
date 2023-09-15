use crate::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameUpdate {
    pub id: Uuid,

    pub screen_size: (i32, i32),
    pub def_square_tile_size: i32,

    pub position: Option<Position>,
    pub old_position: Option<Position>,
    pub max_transition_time: usize,
    pub curr_transition_time: usize,

    /// The character sheet of the clients character
    pub sheet: Sheet,

    /// The script for the current screen which handles the screen drawing
    pub screen_script_name: Option<String>,

    /// The screen scripts and their utility scripts, will be send only once
    pub screen_scripts: Option<FxHashMap<String, String>>,

    /// A region
    pub region: Option<GameRegionData>,

    /// Current lights in the region
    pub lights: Vec<LightData>,

    /// Tile displacements for the region
    #[serde(with = "vectorize")]
    pub displacements: FxHashMap<(isize, isize), TileData>,

    /// Character information
    pub characters: Vec<CharacterData>,

    /// Loot information
    #[serde(with = "vectorize")]
    pub loot: FxHashMap<(isize, isize), Vec<Item>>,

    /// Messages
    pub messages: Vec<MessageData>,

    /// Audio files to play
    pub audio: Vec<String>,

    /// Multiple Choice Data
    pub multi_choice_data: Vec<MultiChoiceData>,

    /// Ongoing communications
    pub communication: Vec<PlayerCommunication>,

    /// Date
    pub date: Date,
}

impl GameUpdate {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            screen_size: (1024, 608),
            def_square_tile_size: 32,
            position: None,
            old_position: None,
            max_transition_time: 0,
            curr_transition_time: 0,
            sheet: Sheet::new(),
            screen_script_name: None,
            screen_scripts: None,
            region: None,
            lights: vec![],
            displacements: FxHashMap::default(),
            characters: vec![],
            loot: FxHashMap::default(),
            messages: vec![],
            audio: vec![],
            multi_choice_data: vec![],
            communication: vec![],
            date: Date::new(),
        }
    }
}
