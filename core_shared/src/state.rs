use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]

/// State of an item.
pub struct State {
    /// On / off switch
    pub state: bool,
    pub use_counter: usize,
    pub use_ticks: usize,

    pub light: Option<LightData>,
    pub tile: Option<TileData>,
}

impl State {
    pub fn new() -> Self {
        Self {
            state: false,
            use_counter: 0,
            use_ticks: 0,

            light: None,
            tile: None,
        }
    }
}
