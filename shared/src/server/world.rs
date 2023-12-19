//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct World {
    pub tick_counter: i64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            tick_counter: 0,
        }
    }

    /// Tick.
    pub fn tick(& mut self) {
        self.tick_counter += 1;
    }

    /// Reset the world.
    pub fn reset(& mut self) {
        self.tick_counter = 0;
    }
}