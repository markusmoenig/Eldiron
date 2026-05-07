use crate::GeoId;
use vek::Vec3;

#[derive(Debug, Clone)]
pub struct Line3D {
    pub id: GeoId,
    pub a: Vec3<f32>,
    pub b: Vec3<f32>,
    pub color: [f32; 4],
    pub layer: i32,
    pub visible: bool,
}
