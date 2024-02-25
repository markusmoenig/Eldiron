use theframework::prelude::*;

/// Level holds all necessary data needed to represent a game level.
/// i.e. defining blocking areas, spawn points, portals, tile types at a given position etc.
#[derive(Clone)]
pub struct Level {
    pub time: TheTime,
    pub blocking: TheFlattenedMap<bool>,
    pub lights: FxHashMap<Vec2i, TheCollection>,
}

impl Level {
    pub fn new(width: i32, height: i32, time: TheTime) -> Self {
        Self {
            blocking: TheFlattenedMap::new(width, height),
            time,
            lights: FxHashMap::default(),
        }
    }

    /// Clears the blocking positions of the level.
    pub fn clear_blocking(&mut self) {
        self.blocking.clear();
    }

    /// Marks the given position as blocking.
    #[inline(always)]
    pub fn set_blocking(&mut self, position: (i32, i32)) {
        self.blocking.set(position, true);
    }

    /// Checks if the given position is blocking.
    #[inline(always)]
    pub fn is_blocking(&self, position: (i32, i32)) -> bool {
        if let Some(blocking) = self.blocking.get(position) {
            *blocking
        } else {
            false
        }
    }

    /// Adds a light to the level.
    #[inline(always)]
    pub fn add_light(&mut self, position: Vec2i, light: TheCollection) {
        self.lights.insert(position, light);
    }
}
