use crate::prelude::*;
use theframework::prelude::*;

/// Create the camera setup.
pub fn create_camera_setup(
    mut position: Vec3f,
    region: &Region,
    settings: &mut RegionDrawSettings,
) -> (Vec3f, Vec3f, f32, CameraMode, CameraType) {
    let mut facing = vec3f(0.0, 0.0, -1.0);
    if settings.center_on_character.is_some() {
        position = settings.center_3d + position;
        facing = settings.facing_3d;
    }

    // Get the camera settings

    let mut camera_type = CameraType::TiltedIso;
    let mut first_person_height = 0.5;
    let mut top_down_height = 4.0;
    let mut top_down_x_offset = -5.0;
    let mut top_down_z_offset = 5.0;
    let mut first_person_fov = 70.0;
    let mut top_down_fov = 55.0;
    let tilted_iso_height = 3.0;
    let mut tilted_iso_fov = 74.0;

    if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
        str!("Camera"),
        str!("Camera Type"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if value == 0 {
            camera_type = CameraType::FirstPerson;
        } else if value == 1 {
            camera_type = CameraType::TopDown;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("First Person FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            first_person_fov = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_fov = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("First Person Height"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            first_person_height = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down Height"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_height = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down X Offset"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_x_offset = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down Z Offset"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_z_offset = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Tilted Iso FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            tilted_iso_fov = value;
        }
    }

    // Camera

    let mut ro = vec3f(position.x + 0.5, 0.5, position.z + 0.5);
    let rd;
    let fov;
    let mut camera_mode = CameraMode::Pinhole;

    if camera_type == CameraType::TopDown {
        rd = ro;
        ro.y = top_down_height;
        ro.x += top_down_x_offset;
        ro.z += top_down_z_offset;
        fov = top_down_fov;
        camera_mode = CameraMode::Orthogonal;
    } else if camera_type == CameraType::FirstPerson {
        // First person
        ro.y = first_person_height;
        rd = ro + facing * 2.0;
        fov = first_person_fov;
    } else {
        // Tilted iso
        rd = ro;
        ro.y = tilted_iso_height;
        ro.z += 1.0;
        fov = tilted_iso_fov;
        camera_mode = CameraMode::Orthogonal;
    }

    (ro, rd, fov, camera_mode, camera_type)
}

/// Gets the current time in milliseconds
pub fn get_time() -> u128 {
    let time;
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        time = t.as_millis();
    }
    #[cfg(target_arch = "wasm32")]
    {
        time = web_sys::window().unwrap().performance().unwrap().now() as u128;
    }
    time
}

/*
pub fn ray_models(ray: &Ray, models: &(Vec<ModelFXFloor>, Vec<ModelFXWall>)) -> Option<Hit> {
    let mut hit: Option<Hit> = None;
    let (floors, walls) = models;
    for fx in floors {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    for fx in walls {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    hit
}
*/

fn reflect(i: Vec3f, n: Vec3f) -> Vec3f {
    i - 2.0 * dot(i, n) * n
}

fn refract(i: Vec3f, n: Vec3f, eta: f32) -> Vec3f {
    let k = 1.0 - eta * eta * (1.0 - dot(i, n) * dot(i, n));
    if k < 0.0 {
        Vec3f::new(0.0, 0.0, 0.0) // Total internal reflection
    } else {
        eta * i - (eta * dot(i, n) + k.sqrt()) * n
    }
}

// The following is based on https://www.shadertoy.com/view/Dtl3WS

// Color

fn luma(color: Vec3f) -> f32 {
    dot(color, vec3f(0.299, 0.587, 0.114))
}

// Microfacet

pub fn fresnel(n1: f32, n2: f32, voh: f32, f0: f32, f90: f32) -> f32 {
    let mut r0 = (n1 - n2) / (n1 + n2);
    r0 *= r0;
    let mut voh = voh;
    if n1 > n2 {
        let n = n1 / n2;
        let sin_t2 = n * n * (1.0 - voh * voh);
        if sin_t2 > 1.0 {
            return f90;
        }
        voh = (1.0 - sin_t2).sqrt();
    }
    let x = 1.0 - voh;
    let ret = r0 + (1.0 - r0) * x.powi(5);

    f0 + (f90 - f0) * ret
}

pub fn f_schlick(f0: Vec3f, theta: f32) -> Vec3f {
    f0 + (Vec3f::new(1.0, 1.0, 1.0) - f0) * (1.0 - theta).powi(5)
}

pub fn f_schlick_scalar(f0: f32, f90: f32, theta: f32) -> f32 {
    f0 + (f90 - f0) * (1.0 - theta).powi(5)
}

pub fn d_gtr(roughness: f32, noh: f32, k: f32) -> f32 {
    let a2 = roughness.powi(2);
    a2 / (std::f32::consts::PI * ((noh * noh) * (a2 * a2 - 1.0) + 1.0).powf(k))
}

pub fn smith_g(no_v: f32, roughness2: f32) -> f32 {
    let a = roughness2.powi(2);
    let b = no_v.powi(2);
    (2.0 * no_v) / (no_v + (a + b - a * b).sqrt())
}

pub fn geometry_term(no_l: f32, no_v: f32, roughness: f32) -> f32 {
    let a2 = roughness * roughness;
    let g1 = smith_g(no_v, a2);
    let g2 = smith_g(no_l, a2);
    g1 * g2
}

pub fn sample_ggx_vndf(v: Vec3f, ax: f32, ay: f32, r1: f32, r2: f32) -> Vec3f {
    let vh = normalize(vec3f(ax * v.x, ay * v.y, v.z));

    let lensq = vh.x * vh.x + vh.y * vh.y;
    let tt1 = if lensq > 0.0 {
        vec3f(-vh.y, vh.x, 0.0) * lensq.sqrt().recip()
    } else {
        vec3f(1.0, 0.0, 0.0)
    };
    let tt2 = cross(vh, tt1);

    let r = r1.sqrt();
    let phi = 2.0 * std::f32::consts::PI * r2;
    let t1 = r * phi.cos();
    let mut t2 = r * phi.sin();
    let s = 0.5 * (1.0 + vh.z);
    t2 = (1.0 - s) * (1.0 - t1 * t1).sqrt().max(0.0) + s * t2;

    let nh = t1 * tt1 + t2 * tt2 + (1.0 - t1 * t1 - t2 * t2).max(0.0).sqrt() * vh;

    normalize(vec3f(ax * nh.x, ay * nh.y, nh.z.max(0.0)))
}

pub fn ggx_vndf_pdf(noh: f32, nov: f32, roughness: f32) -> f32 {
    let d = d_gtr(roughness, noh, 2.0);
    let g1 = smith_g(nov, roughness * roughness);
    (d * g1) / (4.0 * nov).max(0.00001)
}

// BSDF

pub fn eval_disney_diffuse(mat: &Hit, no_l: f32, no_v: f32, lo_h: f32, roughness: f32) -> Vec3f {
    let fd90 = 0.5 + 2.0 * roughness * lo_h.powi(2);
    let a = f_schlick_scalar(1.0, fd90, no_l);
    let b = f_schlick_scalar(1.0, fd90, no_v);

    mat.albedo * (a * b / std::f32::consts::PI)
}

pub fn eval_disney_specular_reflection(mat: &Hit, f: Vec3f, noh: f32, nov: f32, nol: f32) -> Vec3f {
    let roughness = mat.roughness.powi(2);
    let d = d_gtr(roughness, noh, 2.0);
    let g = geometry_term(nol, nov, (0.5 + mat.roughness * 0.5).powi(2));

    d * f * g / (4.0 * nol * nov)
}

#[allow(clippy::too_many_arguments)]
pub fn eval_disney_specular_refraction(
    mat: &Hit,
    f: f32,
    noh: f32,
    nov: f32,
    nol: f32,
    voh: f32,
    loh: f32,
    eta: f32,
    pdf: &mut f32,
) -> Vec3f {
    let roughness = mat.roughness.powi(2);
    let d = d_gtr(roughness, noh, 2.0);
    let g = geometry_term(nol, nov, (0.5 + mat.roughness * 0.5).powi(2));
    let denom = (loh + voh * eta).powi(2);

    let jacobian = (loh.abs()) / denom;
    *pdf = smith_g(nol.abs(), roughness * roughness) * voh.max(0.0) * d * jacobian / nov;

    powf(Vec3f::new(1.0, 1.0, 1.0) - mat.albedo, 0.5)
        * d
        * (1.0 - f)
        * g
        * voh.abs()
        * jacobian
        * eta.powi(2)
        / (nol * nov).abs()
}

fn basis(n: Vec3f) -> (Vec3f, Vec3f) {
    if n.z < 0.0 {
        let a = 1.0 / (1.0 - n.z);
        let b = n.x * n.y * a;
        (
            vec3f(1.0 - n.x * n.x * a, -b, n.x),
            vec3f(b, n.y * n.y * a - 1.0, -n.y),
        )
    } else {
        let a = 1.0 / (1.0 + n.z);
        let b = -n.x * n.y * a;
        (
            vec3f(1.0 - n.x * n.x * a, b, -n.x),
            vec3f(b, 1.0 - n.y * n.y * a, -n.y),
        )
    }
}

fn to_world(x: Vec3f, y: Vec3f, z: Vec3f, v: Vec3f) -> Vec3f {
    v.x * x + v.y * y + v.z * z
}

fn to_local(x: Vec3f, y: Vec3f, z: Vec3f, v: Vec3f) -> Vec3f {
    vec3f(dot(v, x), dot(v, y), dot(v, z))
}

fn cosine_sample_hemisphere(n: Vec3f, rng: &mut ThreadRng) -> Vec3f {
    let rnd = vec2f(rng.gen(), rng.gen());

    let a = std::f32::consts::PI * 2.0 * rnd.x;
    let b = 2.0 * rnd.y - 1.0;

    let t = sqrt(1.0 - b * b) * vec2f(a.cos(), a.sin());
    let dir = vec3f(t.x, t.y, b);
    normalize(n + dir)
}

pub fn sample_disney_bsdf(
    v: Vec3f,
    n: Vec3f,
    mat: &Hit,
    out_dir: &mut Vec3f,
    state: &mut TracerState,
    rng: &mut ThreadRng,
) -> (Vec3f, f32) {
    state.has_been_refracted = state.is_refracted;

    let roughness = mat.roughness.powi(2);

    // sample microfacet normal
    let (t, b) = basis(n);
    let v_local = to_local(t, b, n, v);
    let mut h = sample_ggx_vndf(v_local, roughness, roughness, rng.gen(), rng.gen());
    if h.z < 0.0 {
        h = -h;
    }
    h = to_world(t, b, n, h);

    // fresnel
    let voh = dot(v, h);
    let f0 = lerp(Vec3f::new(0.04, 0.04, 0.04), mat.albedo, mat.metallic);
    let f = f_schlick(f0, voh);
    let diel_f = fresnel(state.last_ior, mat.ior, voh.abs(), 0.0, 1.0);

    // lobe weight probability
    let diff_w = (1.0 - mat.metallic) * (1.0 - mat.spec_trans);
    let reflect_w = luma(f);
    let refract_w = (1.0 - mat.metallic) * mat.spec_trans * (1.0 - diel_f);
    let inv_w = 1.0 / (diff_w + reflect_w + refract_w);
    let diff_w = diff_w * inv_w;
    let reflect_w = reflect_w * inv_w;
    let refract_w = refract_w * inv_w;

    // cdf
    let cdf = [diff_w, diff_w + reflect_w, diff_w + reflect_w + refract_w];

    let mut bsdf; // = Vec3f::new(0.0, 0.0, 0.0);
    let mut pdf; // = 0.0;
    let l; // = Vec3f::new(0.0, 0.0, 0.0);
    let rnd: f32 = rng.gen();
    if rnd < cdf[0] {
        // diffuse
        l = cosine_sample_hemisphere(n, rng);
        h = normalize(l + v);

        let nol = dot(n, l);
        let nov = dot(n, v);
        if nol <= 0.0 || nov <= 0.0 {
            return (Vec3f::new(0.0, 0.0, 0.0), 0.0);
        }
        let loh = dot(l, h);
        pdf = nol / std::f32::consts::PI;

        let diff =
            eval_disney_diffuse(mat, nol, nov, loh, roughness) * (Vec3f::new(1.0, 1.0, 1.0) - f);
        bsdf = diff;
        pdf *= diff_w;
    } else if rnd < cdf[1] {
        // reflection
        l = reflect(-v, h);

        let nol = dot(n, l);
        let nov = dot(n, v);
        if nol <= 0.0 || nov <= 0.0 {
            return (Vec3f::new(0.0, 0.0, 0.0), 0.0);
        }
        let noh = dot(n, h).min(0.99);
        pdf = ggx_vndf_pdf(noh, nov, roughness);

        let spec = eval_disney_specular_reflection(mat, f, noh, nov, nol);
        bsdf = spec;
        pdf *= reflect_w;
    } else {
        // refraction
        state.is_refracted = !state.is_refracted;
        let eta = state.last_ior / mat.ior;
        l = refract(-v, h, eta);
        state.last_ior = mat.ior;

        let nol = dot(n, l);
        if nol <= 0.0 {
            return (Vec3f::new(0.0, 0.0, 0.0), 0.0);
        }
        let nov = dot(n, v);
        let noh = dot(n, h).min(0.99);
        let loh = dot(l, h);

        let mut refract_pdf = 0.0;
        let spec = eval_disney_specular_refraction(
            mat,
            diel_f,
            noh,
            nov,
            nol,
            voh,
            loh,
            eta,
            &mut refract_pdf,
        );

        bsdf = spec;
        pdf = refract_w * refract_pdf;
    }

    bsdf *= dot(n, l).abs();
    *out_dir = l;

    (bsdf, pdf)
}
