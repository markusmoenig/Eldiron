use theframework::prelude::*;

// From IQs SDF3D page at https://iquilezles.org/articles/distfunctions/

pub fn sd_box(p: Vec3f, b: Vec3f) -> f32 {
    let q = abs(p) - b;
    length(max(q, Vec3f::zero())) + min(max(q.x, max(q.y, q.z)), 0.0)
}
