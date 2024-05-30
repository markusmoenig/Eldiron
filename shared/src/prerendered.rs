//use crate::prelude::*;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PreRenderedData {
    pub location: (f32, f32),
    pub pixel_location: (i32, i32),
}

impl PointDistance for PreRenderedData {
    // Calculate the squared distance to a point
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let dx = self.location.0 - point[0];
        let dy = self.location.1 - point[1];
        dx * dx + dy * dy
    }

    // This optional method improves performance by eliminating objects quickly from consideration
    fn contains_point(&self, point: &[f32; 2]) -> bool {
        self.location.0 == point[0] && self.location.1 == point[1]
    }
}

impl RTreeObject for PreRenderedData {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.location.0, self.location.1])
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRendered {
    pub albedo: TheRGBABuffer,
    pub color: FxHashMap<(i32, i32), RGBA>,
    pub tree: RTree<PreRenderedData>,
}

impl Default for PreRendered {
    fn default() -> Self {
        Self::new()
    }
}

impl PreRendered {
    pub fn new() -> Self {
        Self {
            albedo: TheRGBABuffer::default(),
            color: FxHashMap::default(),
            tree: RTree::new(),
        }
    }
}
