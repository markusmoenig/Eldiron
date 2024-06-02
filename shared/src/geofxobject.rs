use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,
    pub material_id: Uuid,

    pub nodes: Vec<GeoFXNode>,

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
            material_id: Uuid::nil(),

            nodes: Vec::new(),
            area: Vec::new(),
        }
    }

    pub fn distance(&self, time: &TheTime, p: Vec2f, scale: f32) -> f32 {
        let mut min_distance = f32::INFINITY;

        for geo in &self.nodes {
            let distance = geo.distance(time, p, scale);
            if distance < min_distance {
                min_distance = distance;
            }
        }

        min_distance
    }

    pub fn distance_3d(&self, time: &TheTime, p: Vec3f) -> f32 {
        let mut min_distance = f32::INFINITY;

        for geo in &self.nodes {
            let distance = geo.distance_3d(time, p);
            if distance < min_distance {
                min_distance = distance;
            }
        }

        min_distance
    }

    pub fn update_area(&mut self) {
        self.area.clear();
        let mut area = AABB2D::zero();
        for geo in &self.nodes {
            if let Some(aabb) = geo.aabb(&TheTime::default()) {
                area.grow(aabb);
            }
        }
        self.area = area.to_tiles();
    }
}
