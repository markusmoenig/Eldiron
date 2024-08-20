use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,
    pub material_id: Uuid,

    pub nodes: Vec<GeoFXNode>,
    pub area: Vec<Vec2i>,

    #[serde(default)]
    pub height: i32,
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

            height: 0,
            level: 0,
        }
    }

    /// Gives a chance to each node to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        for n in &mut self.nodes {
            n.update_parameters();
        }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, time: &TheTime) -> Vec<Vec<f32>> {
        let mut data = vec![];

        for n in &self.nodes {
            data.push(n.load_parameters(time));
        }
        data
    }

    /// Returns the distance to the object nodes, the distance and the index of the closes node is returned.
    pub fn distance(
        &self,
        time: &TheTime,
        p: Vec2f,
        scale: f32,
        hit: &mut Option<&mut Hit>,
    ) -> (f32, usize) {
        let mut min_distance = f32::INFINITY;
        let mut index = 10000;

        for (i, geo) in self.nodes.iter().enumerate() {
            let distance = geo.distance(time, p, scale, hit);
            if distance < min_distance {
                min_distance = distance;
                index = i;
            }
        }

        (min_distance, index)
    }

    pub fn distance_3d(
        &self,
        time: &TheTime,
        p: Vec3f,
        hit: &mut Option<&mut Hit>,
        params: &[Vec<f32>],
    ) -> (f32, usize) {
        let mut min_distance = f32::INFINITY;
        let mut index = 10000;

        for (i, geo) in self.nodes.iter().enumerate() {
            let distance = geo.distance_3d(time, p, hit, &params[i]);
            if distance < min_distance {
                min_distance = distance;
                index = i;
            }
        }

        (min_distance, index)
    }

    pub fn normal(&self, time: &TheTime, p: Vec3f, params: &[Vec<f32>]) -> Vec3f {
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let n = e1 * self.distance_3d(time, p + e1, &mut None, params).0
            + e2 * self.distance_3d(time, p + e2, &mut None, params).0
            + e3 * self.distance_3d(time, p + e3, &mut None, params).0
            + e4 * self.distance_3d(time, p + e4, &mut None, params).0;
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
        // self.area = area.to_tiles();

        // for geo in &self.nodes {
        //     let p = geo.position();
        //     let pp = vec2i(p.x as i32, p.y as i32);
        //     if !self.area.contains(&pp) {
        //         self.area.push(pp);
        //     }
        // }

        for geo in &self.nodes {
            let area = geo.area();
            self.height = geo.height();
            for p in area {
                if !self.area.contains(&p) {
                    self.area.push(p);
                }
            }
        }
    }

    /// Checks if this tile is blocking
    pub fn is_blocking(&self) -> bool {
        for node in &self.nodes {
            if node.is_blocking() {
                return true;
            }
        }
        false
    }

    /// Returns the layer role (Ground, Wall etc) for this object.
    pub fn get_layer_role(&self) -> Option<Layer2DRole> {
        if let Some(geo) = self.nodes.first() {
            return Some(geo.get_layer_role());
        }

        None
    }

    /// Get the tile position of the node.
    pub fn get_position(&self) -> Vec2f {
        if let Some(geo) = self.nodes.first() {
            let collection = geo.collection();
            geo.position(&collection)
        } else {
            Vec2f::zero()
        }
    }

    /// Set the tile position of the node.
    pub fn set_position(&mut self, pos: Vec2f) {
        if let Some(geo) = self.nodes.first_mut() {
            geo.set_position(pos);
        }
    }
}
