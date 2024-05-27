use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,

    pub geos: Vec<GeoFXNode>,

    pub area: Vec<Vec2i>,
}

impl Default for GeoFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl GeoFXObject {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            geos: Vec::new(),
            area: Vec::new(),
        }
    }

    pub fn distance(&self, time: &TheTime, p: Vec2f, scale: f32) -> f32 {
        let mut min_distance = f32::INFINITY;

        for geo in &self.geos {
            let distance = geo.distance(time, p, scale);
            if distance < min_distance {
                min_distance = distance;
            }
        }

        min_distance
    }

    pub fn update_area(&mut self) {
        self.area.clear();
        let mut area = AABB2D::zero();
        for geo in &self.geos {
            if let Some(aabb) = geo.aabb(&TheTime::default()) {
                area.grow(aabb);
            }
        }
        self.area = area.to_tiles();
    }
}
