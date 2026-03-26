use theframework::prelude::*;

/// 2D hash, taken from https://www.shadertoy.com/view/4djSRW
#[inline(always)]
pub fn hash21(p: Vec2<f32>) -> f32 {
    let mut p3 = Vec3::new(
        (p.x * 0.1031).fract(),
        (p.y * 0.1031).fract(),
        (p.x * 0.1031).fract(),
    );
    let dot = p3.dot(Vec3::new(p3.y + 33.333, p3.z + 33.333, p3.x + 33.333));

    p3.x += dot;
    p3.y += dot;
    p3.z += dot;

    ((p3.x + p3.y) * p3.z).fract()
}

/*
pub fn noise2d(p: &Vec2f, scale: Vec2f, octaves: i32) -> f32 {
    fn hash(p: Vec2f) -> f32 {
        let mut p3 = frac(vec3f(p.x, p.y, p.x) * 0.13);
        p3 += dot(p3, vec3f(p3.y, p3.z, p3.x) + 3.333);
        frac((p3.x + p3.y) * p3.z)
    }

    fn noise(x: Vec2f) -> f32 {
        let i = floor(x);
        let f = frac(x);

        let a = hash(i);
        let b = hash(i + vec2f(1.0, 0.0));
        let c = hash(i + vec2f(0.0, 1.0));
        let d = hash(i + vec2f(1.0, 1.0));

        let u = f * f * (3.0 - 2.0 * f);
        lerp(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y
    }

    let mut x = *p * 8.0 * scale;

    if octaves == 0 {
        return noise(x);
    }

    let mut v = 0.0;
    let mut a = 0.5;
    let shift = vec2f(100.0, 100.0);
    // Rotate to reduce axial bias
    let rot = Mat2::new(cos(0.5), sin(0.5), -sin(0.5), cos(0.50));
    for _ in 0..octaves {
        v += a * noise(x);
        x = rot * x * 2.0 + shift;
        a *= 0.5;
    }
    v
}*/

fn rot(a: f32) -> Mat2<f32> {
    Mat2::new(a.cos(), -a.sin(), a.sin(), a.cos())
}

// Shane's box divide formula from https://www.shadertoy.com/view/XsGyDh and https://www.shadertoy.com/view/Ws3GRs
pub fn box_divide(p: Vec2<f32>, gap: f32, rotation: f32, rounding: f32) -> (f32, f32) {
    fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
        let d = p.map(|v| v.abs()) - b + Vec2::new(r, r);
        d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
    }

    let mut p = p;
    let ip = p.map(|v| v.floor());
    p -= ip;

    let mut l = Vec2::new(1.0, 1.0);
    let mut last_l;
    let mut r = hash21(ip);

    for _ in 0..6 {
        r = (l + Vec2::new(r, r)).dot(Vec2::new(123.71, 439.43)).fract() * 0.4 + (1.0 - 0.4) / 2.0;

        last_l = l;
        if l.x > l.y {
            p = Vec2::new(p.y, p.x);
            l = Vec2::new(l.y, l.x);
        }

        if p.x < r {
            l.x /= r;
            p.x /= r;
        } else {
            l.x /= 1.0 - r;
            p.x = (p.x - r) / (1.0 - r);
        }

        if last_l.x > last_l.y {
            p = Vec2::new(p.y, p.x);
            l = Vec2::new(l.y, l.x);
        }
    }
    p -= 0.5;

    // Create the id
    let id = hash21(ip + l);

    // Slightly rotate the tile based on its id
    p = rot((id - 0.5) * rotation) * p;

    // Gap, or mortar, width. Using "l" to keep it roughly constant.
    let th = l * 0.02 * gap;

    // Take the subdivided space and turn them into rounded pavers.
    let c = s_box(p, Vec2::new(0.5, 0.5) - th, rounding);

    // Return distance and id
    (c, id)
}
