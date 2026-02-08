//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct World {
    pub tick_counter: i64,
    pub time: TheTime,
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
            time: TheTime::default(),
        }
    }

    /// Tick.
    pub fn tick(&mut self) {
        self.tick_counter += 1;
        self.time = TheTime::from_ticks(self.tick_counter, 4);
    }

    /// Reset the world.
    pub fn reset(&mut self) {
        self.tick_counter = 0;
    }

    pub fn set_time(&mut self, time: TheTime) {
        self.tick_counter = time.to_ticks(4);
        self.time = time;
    }
}
