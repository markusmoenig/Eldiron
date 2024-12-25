#[inline(always)]
pub fn sdf_box2d(p: Vec2f, pos: Vec2f, dim1: f32, dim2: f32) -> f32 {
    let d = abs(p - pos) - vec2f(dim1, dim2);
    length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0)
}

#[inline(always)]
pub fn sdf_rounded_box2d(p: Vec2f, size: f32, thick: f32, rounding: (f32, f32, f32, f32)) -> f32 {
    let mut r: (f32, f32);

    if p.x > 0.0 {
        r = (rounding.0, rounding.1);
    } else {
        r = (rounding.2, rounding.3);
    }

    if p.y <= 0.0 {
        r.0 = r.1;
    }
    let hb = thick / 2.0;
    let q: (f32, f32) = (
        p.x.abs() - size + hb + rounding.0,
        p.y.abs() - size + hb + rounding.0,
    );
    f32::min(f32::max(q.0, q.1), 0.0) + length(vec2f(f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
        - rounding.0
}

// Create multiple copies of an object - https://iquilezles.org/articles/sdfrepetition/
pub fn op_rep_lim(p: Vec2f, s: f32, lima: Vec2f, limb: Vec2f) -> Vec2f {
    p - s * clamp(round(p / s), lima, limb)
}
