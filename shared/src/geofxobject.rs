use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,
    pub material_id: Uuid,

    pub nodes: Vec<GeoFXNode>,

    pub area: Vec<Vec2i>,

    pub level: i32,
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

            level: 0,
        }
    }

    /// Returns the distance to the object nodes, the distance and the index of the closes node is returned.
    pub fn distance(&self, time: &TheTime, p: Vec2f, scale: f32) -> (f32, usize) {
        let mut min_distance = f32::INFINITY;
        let mut index = 10000;

        for (i, geo) in self.nodes.iter().enumerate() {
            let distance = geo.distance(time, p, scale);
            if distance < min_distance {
                min_distance = distance;
                index = i;
            }
        }

        (min_distance, index)
    }

    pub fn distance_3d(&self, time: &TheTime, p: Vec3f, hit: &mut Option<&mut Hit>) -> f32 {
        let mut min_distance = f32::INFINITY;

        for geo in &self.nodes {
            let distance = geo.distance_3d(time, p, hit);
            if distance < min_distance {
                min_distance = distance;
            }
        }

        min_distance
    }

    pub fn normal(&self, time: &TheTime, p: Vec3f) -> Vec3f {
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let n = e1 * self.distance_3d(time, p + e1, &mut None)
            + e2 * self.distance_3d(time, p + e2, &mut None)
            + e3 * self.distance_3d(time, p + e3, &mut None)
            + e4 * self.distance_3d(time, p + e4, &mut None);
        normalize(n)
    }

    pub fn update_area(&mut self) {
        self.area.clear();
        // let mut area = AABB2D::zero();
        // for geo in &self.nodes {
        //     if let Some(aabb) = geo.aabb(&TheTime::default()) {
        //         area.grow(aabb);
        //     }
        // }
        //self.area = area.to_tiles();

        for geo in &self.nodes {
            let p = geo.position();
            let pp = vec2i(p.x as i32, p.y as i32);
            if !self.area.contains(&pp) {
                self.area.push(pp);
            }
        }
    }
}
