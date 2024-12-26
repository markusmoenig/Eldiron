use crate::prelude::*;
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

pub fn noise2d(p: &Vec2<f32>, scale: Vec2<f32>, octaves: i32) -> f32 {
    fn hash(p: Vec2<f32>) -> f32 {
        let mut p3 = Vec3::new(p.x, p.y, p.x).map(|v| (v * 0.13).fract());
        p3 += p3.dot(Vec3::new(p3.y, p3.z, p3.x) + 3.333);
        ((p3.x + p3.y) * p3.z).fract()
    }

    fn noise(x: Vec2<f32>) -> f32 {
        let i = x.map(|v| v.floor());
        let f = x.map(|v| v.fract());

        let a = hash(i);
        let b = hash(i + Vec2::new(1.0, 0.0));
        let c = hash(i + Vec2::new(0.0, 1.0));
        let d = hash(i + Vec2::new(1.0, 1.0));

        let u = f * f * f.map(|v| 3.0 - 2.0 * v);
        f32::lerp(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y
    }

    let mut x = *p * 8.0 * scale;

    if octaves == 0 {
        return noise(x);
    }

    let mut v = 0.0;
    let mut a = 0.5;
    let shift = Vec2::new(100.0, 100.0);
    let rot = Mat2::new(0.5f32.cos(), 0.5f32.sin(), -0.5f32.sin(), 0.5f32.cos());
    for _ in 0..octaves {
        v += a * noise(x);
        x = rot * x * 2.0 + shift;
        a *= 0.5;
    }
    v
}

pub fn noise3d(coll: &TheCollection, p: &Vec3<f32>) -> f32 {
    fn hash(mut p: f32) -> f32 {
        p = (p * 0.011).fract();
        p *= p + 7.5;
        p *= p + p;
        p.fract()
    }

    fn noise(x: Vec3<f32>) -> f32 {
        let step = Vec3::new(110.0, 241.0, 171.0);

        let i = x.map(|v| v.floor());
        let f = x.map(|v| v.fract());

        let n = i.dot(step);

        let u = f * f * f.map(|v| 3.0 - 2.0 * v);

        f32::lerp(
            f32::lerp(
                f32::lerp(
                    hash(n + step.dot(Vec3::new(0.0, 0.0, 0.0))),
                    hash(n + step.dot(Vec3::new(1.0, 0.0, 0.0))),
                    u.x,
                ),
                f32::lerp(
                    hash(n + step.dot(Vec3::new(0.0, 1.0, 0.0))),
                    hash(n + step.dot(Vec3::new(1.0, 1.0, 0.0))),
                    u.x,
                ),
                u.y,
            ),
            f32::lerp(
                f32::lerp(
                    hash(n + step.dot(Vec3::new(0.0, 0.0, 1.0))),
                    hash(n + step.dot(Vec3::new(1.0, 0.0, 1.0))),
                    u.x,
                ),
                f32::lerp(
                    hash(n + step.dot(Vec3::new(0.0, 1.0, 1.0))),
                    hash(n + step.dot(Vec3::new(1.0, 1.0, 1.0))),
                    u.x,
                ),
                u.y,
            ),
            u.z,
        )
    }

    let scale = Vec3::new(
        coll.get_f32_default("UV Scale X", 1.0),
        coll.get_f32_default("UV Scale Y", 1.0),
        coll.get_f32_default("UV Scale Z", 1.0),
    );

    let octaves = coll.get_i32_default("Octaves", 5);

    let mut x = Vec3::new(1240.0, 1240.0, 1240.0) + *p * 8.0 * scale;

    if octaves == 0 {
        return noise(x);
    }

    let mut v = 0.0;
    let mut a = 0.5;
    let shift = Vec3::new(100.0, 100.0, 100.0);
    for _ in 0..octaves {
        v += a * noise(x);
        x = x * 2.0 + shift;
        a *= 0.5;
    }
    v
}

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

pub fn bricks(uv: Vec2<f32>, hit: &mut Hit, params: &[f32]) -> f32 {
    fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
        let d = p.map(|v| v.abs()) - b + Vec2::new(r, r);
        d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
    }

    let ratio = params[0];
    let round = params[1];
    let rotation = params[2];
    let gap = params[3];
    let cell = params[4];
    let mode = params[5] as i32;

    let mut u = uv;

    let w = Vec2::new(ratio, 1.0);
    u *= Vec2::new(cell, cell) / w;

    if mode == 0 {
        u.x += 0.5 * (u.y.floor() % 2.0);
    }

    let id = hash21(u.map(|v| v.floor()));

    let mut p = u.map(|v| v.fract());
    p = rot((id - 0.5) * rotation) * (p - 0.5);

    hit.hash = id;
    hit.uv = p;

    s_box(p, Vec2::new(0.5, 0.5) - gap, round)
}
