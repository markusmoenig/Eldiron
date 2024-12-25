use theframework::prelude::*;

/// Level holds all necessary data needed to represent a game level.
/// i.e. defining blocking areas, spawn points, portals, tile types at a given position etc.
#[derive(Clone)]
pub struct Level {
    pub time: TheTime,
    pub blocking: TheFlattenedMap<bool>,
    pub lights: FxHashMap<Vec2<i32>, Light>,
}

impl Level {
    pub fn new(width: i32, height: i32, time: TheTime) -> Self {
        Self {
            blocking: TheFlattenedMap::new(width, height),
            time,
            lights: FxHashMap::default(),
        }
    }

    /// Clears the level.
    pub fn clear(&mut self) {
        self.blocking.clear();
        self.lights.clear();
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
    pub fn add_light(&mut self, position: Vec2<i32>, light: TheCollection) {
        let light = Light::from_collection(&light);
        self.lights.insert(position, light);
    }
}

#[derive(Clone)]
pub struct Light {
    pub max_distance: f32,
    pub strength: f32,
    pub sampling_offset: f32,
    pub samples: usize,
    pub color_type: i32,
    pub color: Vec3<f32>,
    pub limiter: i32,
}

impl Light {
    pub fn from_collection(light_coll: &TheCollection) -> Self {
        let max_distance = light_coll.get_i32_default("Max. Distance", 10) as f32;
        let strength = light_coll.get_f32_default("Strength", 1.0);
        let sampling_offset = light_coll.get_f32_default("Sample Offset", 0.5);
        let samples = light_coll.get_i32_default("Samples #", 5) as usize;
        let color_type = light_coll.get_i32_default("Light Color", 0);
        let color = light_coll.get_float3_default("Color", Vec3::new(1.0, 1.0, 1.0));
        let limiter = light_coll.get_i32_default("Limit Direction", 0);

        Light {
            max_distance,
            strength,
            sampling_offset,
            samples,
            color_type,
            color,
            limiter,
        }
    }
}
