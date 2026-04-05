use vek::Vec3;

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
