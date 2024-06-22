use crate::prelude::*;
use theframework::prelude::*;

/// 2D hash, taken from https://www.shadertoy.com/view/4djSRW
#[inline(always)]
pub fn hash21(p: Vec2f) -> f32 {
    let mut p3 = frac(vec3f(p.x * 0.1031, p.y * 0.1031, p.x * 0.1031));
    let dot = dot(p3, vec3f(p3.y + 33.333, p3.z + 33.333, p3.x + 33.333));

    p3.x += dot;
    p3.y += dot;
    p3.z += dot;
    ((p3.x + p3.y) * p3.z).fract()
}

pub fn bricks(coll: &TheCollection, uv: Vec2f, hit: &mut Hit) -> (u8, u8) {
    //let uv = hit.uv / 100.0;

    let ratio = coll.get_f32_default("Ratio", 2.0);
    let round = coll.get_f32_default("Rounding", 0.0);
    let bevel = coll.get_f32_default("Bevel", 0.0);
    let gap = coll.get_f32_default("Gap", 0.1);
    let mode = coll.get_i32_default("Mode", 0);

    let mut u = uv; // + vec2f(10000.0, 10000.0);

    let cell = coll.get_f32_default("Cell", 6.0);

    let w = vec2f(ratio, 1.0);
    u *= vec2f(cell, cell) / w; //u.component_mul(&FP2::new(self.cell, self.cell).component_div(&w));

    if mode == 0 {
        u.x += 0.5 * u.y.floor() % 2.0;
    }

    let new_uv = frac(u);
    hit.uv = new_uv;
    hit.hash = hash21(floor(u));

    let t = new_uv - vec2f(1.0, 1.0) / 2.0;
    let s = w * t;

    let a = w / 2.0 - gap - abs(s);
    let b = a * vec2f(2.0, 2.0) / bevel; ////a.component_mul(&FP2::new(2.0, 2.0)).component_div(&bevel);
    let mut m = b.x.min(b.y);
    if a.x < round && a.y < round {
        m = (round - length(vec2f(round, round) - a)) * 2.0; //
        dot(vec2f(bevel, bevel), normalize(vec2f(round, round) - a));
    }

    //(m.clamp(0.0, 1.0), self.hash21(glm::floor(&u)));

    let m = m.clamp(0.0, 1.0);

    if m == 1.0 {
        (0, 0)
    } else {
        (5, 1)
    }
}

pub fn steepness(coll: &TheCollection, _uv: Vec2f, hit: &mut Hit) -> (u8, u8) {
    let angle1 = coll.get_f32_default("Angle #1", 10.0);
    let angle2 = coll.get_f32_default("Angle #2", 30.0);

    let slope_angle = acos(dot(hit.normal, vec3f(0.0, 1.0, 0.0)));

    if slope_angle < angle1.to_radians() {
        (0, 0)
    } else if slope_angle < angle2.to_radians() {
        (1, 1)
    } else {
        (2, 2)
    }
}

pub fn subdivide(coll: &TheCollection, uv: Vec2f, hit: &mut Hit) -> u8 {
    let mode = coll.get_i32_default("Mode", 0);
    let offset = coll.get_f32_default("Offset", 0.5);

    if mode == 0 {
        if uv.x < offset {
            hit.uv = uv / offset;
            0
        } else {
            hit.uv = (uv - vec2f(offset, 0.0)) / (1.0 - offset);
            1
        }
    } else if uv.y < offset {
        hit.uv = uv / offset;
        0
    } else {
        hit.uv = (uv - vec2f(0.0, offset)) / (1.0 - offset);
        1
    }
}

pub fn noise2d(coll: &TheCollection, p: &Vec2f) -> f32 {
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

    let scale = coll.get_f32_default("UV Scale", 1.0);
    let out_scale = coll.get_f32_default("Out Scale", 1.0);
    let octaves = coll.get_i32_default("Octaves", 5);

    let mut x = *p * 8.0 * scale;

    if octaves == 0 {
        return noise(x) * out_scale;
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
    v * out_scale
}

pub fn noise3d(coll: &TheCollection, p: &Vec3f) -> f32 {
    fn hash(mut p: f32) -> f32 {
        p = frac(p * 0.011);
        p *= p + 7.5;
        p *= p + p;
        frac(p)
    }

    fn noise(x: Vec3f) -> f32 {
        let step: Vec3f = vec3f(110.0, 241.0, 171.0);

        let i = floor(x);
        let f = frac(x);

        let n = dot(i, step);

        let u = f * f * (3.0 - 2.0 * f);
        lerp(
            lerp(
                lerp(
                    hash(n + dot(step, vec3f(0.0, 0.0, 0.0))),
                    hash(n + dot(step, vec3f(1.0, 0.0, 0.0))),
                    u.x,
                ),
                lerp(
                    hash(n + dot(step, vec3f(0.0, 1.0, 0.0))),
                    hash(n + dot(step, vec3f(1.0, 1.0, 0.0))),
                    u.x,
                ),
                u.y,
            ),
            lerp(
                lerp(
                    hash(n + dot(step, vec3f(0.0, 0.0, 1.0))),
                    hash(n + dot(step, vec3f(1.0, 0.0, 1.0))),
                    u.x,
                ),
                lerp(
                    hash(n + dot(step, vec3f(0.0, 1.0, 1.0))),
                    hash(n + dot(step, vec3f(1.0, 1.0, 1.0))),
                    u.x,
                ),
                u.y,
            ),
            u.z,
        )
    }

    let scale = coll.get_f32_default("UV Scale", 1.0);
    let out_scale = coll.get_f32_default("Out Scale", 1.0);
    let octaves = coll.get_i32_default("Octaves", 5);

    let mut x = 1240.0 + *p * 8.0 * scale;

    if octaves == 0 {
        return noise(x) * out_scale;
    }

    let mut v = 0.0;
    let mut a = 0.5;
    let shift = vec3f(100.0, 100.0, 100.0);
    for _ in 0..octaves {
        v += a * noise(x);
        x = x * 2.0 + shift;
        a *= 0.5;
    }
    v * out_scale
}

fn rot(a: f32) -> Mat2f {
    Mat2f::new(a.cos(), -a.sin(), a.sin(), a.cos())
}

// Shane's box divide formula from https://www.shadertoy.com/view/XsGyDh
pub fn box_divide(p: Vec2f) -> (f32, f32) {
    fn s_box(p: Vec2f, b: Vec2f, r: f32) -> f32 {
        let d = abs(p) - b + vec2f(r, r);
        d.x.max(d.y).min(0.0) + length(max(d, vec2f(0.0, 0.0))) - r
    }

    let mut p = p;
    let ip = floor(p);
    p -= ip;
    let mut l = vec2f(1.0, 1.0);
    let mut last_l;
    let mut r = hash21(ip);

    for _ in 0..6 {
        r = (dot(l + vec2f(r, r), vec2f(123.71, 439.43)).fract() * 0.4) + (1.0 - 0.4) / 2.0;

        last_l = l;
        if l.x > l.y {
            p = vec2f(p.y, p.x);
            l = vec2f(l.y, l.x);
        }

        if p.x < r {
            l.x /= r;
            p.x /= r;
        } else {
            l.x /= 1.0 - r;
            p.x = (p.x - r) / (1.0 - r);
        }

        if last_l.x > last_l.y {
            p = vec2f(p.y, p.x);
            l = vec2f(l.y, l.x);
        }
    }
    p -= 0.5;

    // Create the id
    let id = hash21(ip + l);

    // Slightly rotate the tile based on its id
    p = rot((id - 0.5) * 0.15) * p;

    // Gap, or mortar, width. Using "l" to keep it roughly constant.
    let th = l * 0.02;
    // Take the subdivided space and turn them into rounded pavers.
    //let c = s_box(p, vec2f(0.5, 0.5) - th, noise(p) * 0.5);
    let c = s_box(p, vec2f(0.5, 0.5) - th, 0.0);
    // Smoothing factor.
    //let sf = 2.0 / res.x * length(l);
    // Individual tile ID.

    // Return distance and id
    (c, id)
}
