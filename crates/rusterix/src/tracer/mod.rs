pub mod buffer;
pub mod trace;

use vek::{Vec2, Vec3};

use crate::GeometrySource;

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3<f32>,
    pub dir: Vec3<f32>,
}

impl Default for Ray {
    fn default() -> Self {
        Ray::empty()
    }
}

impl Ray {
    pub fn new(o: Vec3<f32>, d: Vec3<f32>) -> Self {
        Self { origin: o, dir: d }
    }
    pub fn empty() -> Self {
        Self {
            origin: Vec3::zero(),
            dir: Vec3::zero(),
        }
    }
    pub fn at(&self, t: f32) -> Vec3<f32> {
        self.origin + self.dir * t
    }
}

#[derive(Debug)]
pub struct HitInfo {
    pub t: f32,
    pub uv: Vec2<f32>,
    pub normal: Option<Vec3<f32>>,
    pub triangle_index: usize,
    pub hitpoint: Vec3<f32>,

    pub albedo: Vec3<f32>,
    pub emissive: Vec3<f32>,
    pub specular_weight: f32,

    pub profile_id: Option<u32>,
    pub geometry_source: GeometrySource,
}

impl Default for HitInfo {
    fn default() -> Self {
        HitInfo::new()
    }
}

impl HitInfo {
    pub fn new() -> Self {
        Self {
            t: f32::MAX,
            uv: Vec2::zero(),
            normal: None,
            triangle_index: 0,
            hitpoint: Vec3::zero(),

            albedo: Vec3::zero(),
            emissive: Vec3::zero(),
            specular_weight: 0.0,

            profile_id: None,
            geometry_source: GeometrySource::Unknown,
        }
    }

    pub fn has_hit(&self) -> bool {
        self.t < f32::MAX
    }
}
