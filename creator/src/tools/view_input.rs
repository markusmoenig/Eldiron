//! Immutable CPU query results supplied by the viewport owner for one event.
//! Tools retain authoring behavior without borrowing a renderer or querying an
//! unrelated global VM. The parent validates the displayed output before supply.
use super::*;
use scenevm::{Camera3D, GeoId};

#[derive(Clone)]
pub struct ToolViewInput {
    pub camera: Camera3D,
    pub ray: (Vec3<f32>, Vec3<f32>),
    pub base: Option<(GeoId, Vec3<f32>, f32)>,
    pub dynamic: Option<(GeoId, Vec3<f32>, f32)>,
    pub paint_geo: Option<[u32; 4]>,
    pub rectangle: Option<(Vec2<f32>, Vec2<f32>, Vec<GeoId>)>,
}
impl ToolViewInput {
    pub fn ray(&self) -> rusterix::Ray {
        rusterix::Ray::new(self.ray.0, self.ray.1)
    }
    pub fn rectangle(&self, top: Vec2<f32>, bottom: Vec2<f32>) -> Vec<GeoId> {
        self.rectangle
            .as_ref()
            .filter(|rect| rect.0 == top && rect.1 == bottom)
            .map(|rect| rect.2.clone())
            .unwrap_or_default()
    }
}
