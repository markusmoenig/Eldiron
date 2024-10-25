use crate::prelude::*;
//use std::f32::consts::PI;
use theframework::prelude::*;

/*
pub fn cosine_sample_hemisphere2(n: Vec3f, rng: &mut ThreadRng) -> Vec3f {
    let rnd = vec2f(rng.gen(), rng.gen());

    let a = 2.0 * PI * rnd.x;
    let b = 2.0 * rnd.y - 1.0;

    let cos_a = a.cos();
    let sin_a = a.sin();
    let sqrt_term = sqrt(1.0 - b * b);

    let dir = Vec3f::new(sqrt_term * cos_a, sqrt_term * sin_a, b);

    normalize(n + dir)
}

// From Pixar - https://graphics.pixar.com/library/OrthonormalB/paper.pdf
pub fn basis(n: Vec3f) -> (Vec3f, Vec3f) {
    let b1;
    let b2;

    if n.z < 0.0 {
        let a = 1.0 / (1.0 - n.z);
        let b = n.x * n.y * a;

        b1 = vec3f(1.0 - n.x * n.x * a, -b, n.x);

        b2 = vec3f(b, n.y * n.y * a - 1.0, -n.y);
    } else {
        let a = 1.0 / (1.0 + n.z);
        let b = -n.x * n.y * a;

        b1 = vec3f(1.0 - n.x * n.x * a, b, -n.x);

        b2 = vec3f(b, 1.0 - n.y * n.y * a, -n.y);
    }

    (b1, b2)
}

// ---------------------------------------------
// Microfacet
// ---------------------------------------------
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

        voh = sqrt(1.0 - sin_t2);
    }

    let x = 1.0 - voh;
    let ret = r0 + (1.0 - r0) * powf(x, 5.0);

    lerp(f0, f90, ret)
}

// Schlick approximation for Fresnel for a vector3
pub fn f_schlick_vec3(f0: Vec3f, theta: f32) -> Vec3f {
    f0 + (vec3f(1.0, 1.0, 1.0) - f0) * powf(1.0 - theta, 5.0)
}

// Schlick approximation for Fresnel for scalars
pub fn f_schlick_scalar(f0: f32, f90: f32, theta: f32) -> f32 {
    f0 + (f90 - f0) * powf(1.0 - theta, 5.0)
}

// Generalized Trowbridge-Reitz distribution function (GTR)
pub fn d_gtr(roughness: f32, noh: f32, k: f32) -> f32 {
    let a2 = powf(roughness, 2.0);
    let denom = powf((noh * noh) * (a2 * a2 - 1.0) + 1.0, k);

    a2 / (PI * denom)
}

// Smith's masking-shadowing function
pub fn smith_g2(nov: f32, roughness2: f32) -> f32 {
    let a = powf(roughness2, 2.0);
    let b = powf(nov, 2.0);

    (2.0 * nov) / (nov + sqrt(a + b - a * b))
}

// Combined geometry term
pub fn geometry_term(nol: f32, nov: f32, roughness: f32) -> f32 {
    let a2 = roughness * roughness;
    let g1 = smith_g(nov, a2);
    let g2 = smith_g(nol, a2);

    g1 * g2
}

// GGX VNDF sampling function
pub fn sample_ggx_vndf2(v: Vec3f, ax: f32, ay: f32, r1: f32, r2: f32) -> Vec3f {
    let vh = normalize(vec3f(ax * v.x, ay * v.y, v.z));

    let lensq = vh.x * vh.x + vh.y * vh.y;
    let t1 = if lensq > 0.0 {
        vec3f(-vh.y, vh.x, 0.0) * (1.0 / sqrt(lensq))
    } else {
        vec3f(1.0, 0.0, 0.0)
    };
    let t2 = cross(vh, t1);

    let r = sqrt(r1);
    let phi = 2.0 * PI * r2;
    let t1_val = r * cos(phi);
    let mut t2_val = r * sin(phi);
    let s = 0.5 * (1.0 + vh.z);
    t2_val = (1.0 - s) * sqrt(1.0 - t1_val * t1_val) + s * t2_val;

    let nh =
        t1_val * t1 + t2_val * t2 + sqrt(max(0.0, 1.0 - t1_val * t1_val - t2_val * t2_val)) * vh;

    normalize(vec3f(ax * nh.x, ay * nh.y, max(0.0, nh.z)))
}

// GGX VNDF PDF calculation
pub fn ggx_vndf_pdf(noh: f32, nov: f32, roughness: f32) -> f32 {
    let d = d_gtr(roughness, noh, 2.0);
    let g1 = smith_g(nov, roughness * roughness);

    d * g1 / max(0.00001, 4.0 * nov)
}

pub struct State {
    pub is_refracted: bool,
    pub has_been_refracted: bool,
    pub last_ior: f32,
}

impl Default for State {
    fn default() -> Self {
        State {
            is_refracted: false,
            has_been_refracted: false,
            last_ior: 1.0,
        }
    }
}

// Material struct
pub struct Material {
    pub albedo: Vec3f,
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: Vec3f,

    pub spec_trans: f32,
    pub ior: f32,
    pub absorption: f32,
}

// Evaluate Disney Diffuse BSDF
pub fn eval_disney_diffuse2(mat: &Material, nol: f32, nov: f32, loh: f32, roughness: f32) -> Vec3f {
    let fd90 = 0.5 + 2.0 * roughness * powf(loh, 2.0);
    let a = f_schlick_scalar(1.0, fd90, nol);
    let b = f_schlick_scalar(1.0, fd90, nov);

    mat.albedo * (a * b / PI)
}

// Evaluate Disney Specular Reflection
pub fn eval_disney_specular_reflection(
    mat: &Material,
    f: Vec3f,
    noh: f32,
    nov: f32,
    nol: f32,
) -> Vec3f {
    let roughness = powf(mat.roughness, 2.0);
    let d = d_gtr(roughness, noh, 2.0);
    let g = geometry_term(nol, nov, powf(0.5 + mat.roughness * 0.5, 2.0));

    d * f * g / (4.0 * nol * nov)
}

fn luma(color: Vec3f) -> f32 {
    dot(color, vec3f(0.299, 0.587, 0.114))
}

// Evaluate Disney Specular Refraction
#[allow(clippy::too_many_arguments)]
pub fn eval_disney_specular_refraction(
    mat: &Material,
    f: f32,
    noh: f32,
    nov: f32,
    nol: f32,
    voh: f32,
    loh: f32,
    eta: f32,
    pdf: &mut f32,
) -> Vec3f {
    let roughness = powf(mat.roughness, 2.0);
    let d = d_gtr(roughness, noh, 2.0);
    let g = geometry_term(nol, nov, powf(0.5 + mat.roughness * 0.5, 2.0));
    let denom = powf(loh + voh * eta, 2.0);

    let jacobian = (loh.abs()) / denom;
    *pdf = smith_g(nol.abs(), roughness * roughness) * max(0.0, voh) * d * jacobian / nov;

    powf(
        vec3f(1.0 - mat.albedo.x, 1.0 - mat.albedo.y, 1.0 - mat.albedo.z),
        0.5,
    ) * d
        * (1.0 - f)
        * g
        * voh.abs()
        * jacobian
        * powf(eta, 2.0)
        / (nol.abs() * nov.abs())
}

// Sample Disney BSDF
pub fn sample_disney_bsdf(
    v: Vec3f,
    n: Vec3f,
    mat: &Material,
    state: &mut State,
    rng: &mut ThreadRng,
) -> (Vec4f, Vec3f) {
    state.has_been_refracted = state.is_refracted;
    let roughness = powf(mat.roughness, 2.0);

    // Sample microfacet normal
    let (t, b) = basis(n);
    let v_local = to_local(t, b, n, v);
    let mut h = sample_ggx_vndf(v_local, roughness, roughness, rng.gen(), rng.gen());

    if h.z < 0.0 {
        h = -h;
    }
    h = to_world(t, b, n, h);

    // Fresnel
    let voh = dot(v, h);
    let f0 = lerp(vec3f(0.04, 0.04, 0.04), mat.albedo, mat.metallic);
    let f = f_schlick_vec3(f0, voh);
    let diel_f = fresnel(state.last_ior, mat.ior, voh.abs(), 0.0, 1.0);

    // Lobe weight probability
    let mut diff_w = (1.0 - mat.metallic) * (1.0 - mat.spec_trans);
    let reflect_w = luma(f);
    let refract_w = (1.0 - mat.metallic) * mat.spec_trans * (1.0 - diel_f);
    let inv_w = 1.0 / (diff_w + reflect_w + refract_w);
    diff_w *= inv_w;
    let reflect_w = reflect_w * inv_w;
    let refract_w = refract_w * inv_w;

    // CDF
    let cdf = [diff_w, diff_w + reflect_w];

    let mut bsdf; // = vec4f(0.0, 0.0, 0.0, 0.0);
    let rnd: f32 = rng.gen();
    let l; // = vec3f(0.0, 0.0, 0.0);

    if rnd < cdf[0] {
        // Diffuse
        l = cosine_sample_hemisphere2(n, rng);
        h = normalize(l + v);

        let nol = dot(n, l);
        let nov = dot(n, v);
        if nol <= 0.0 || nov <= 0.0 {
            return (vec4f(0.0, 0.0, 0.0, 0.0), l);
        }
        let loh = dot(l, h);
        let pdf = nol / PI;

        let diff = eval_disney_diffuse2(mat, nol, nov, loh, roughness) * (1.0 - f);
        bsdf = vec4f(diff.x, diff.y, diff.z, diff_w * pdf);
    } else if rnd < cdf[1] {
        // Reflection
        l = Vec3f::reflect(-v, h);

        let nol = dot(n, l);
        let nov = dot(n, v);
        if nol <= 0.0 || nov <= 0.0 {
            return (vec4f(0.0, 0.0, 0.0, 0.0), l);
        }
        let noh = min(0.99, dot(n, h));
        let pdf = ggx_vndf_pdf(noh, nov, roughness);

        let spec = eval_disney_specular_reflection(mat, f, noh, nov, nol);
        bsdf = vec4f(spec.x, spec.y, spec.z, reflect_w * pdf);
    } else {
        // Refraction
        state.is_refracted = !state.is_refracted;
        let eta = state.last_ior / mat.ior;
        l = Vec3f::refract(-v, h, eta);
        state.last_ior = mat.ior;

        let nol = dot(n, l);
        if nol <= 0.0 {
            return (vec4f(0.0, 0.0, 0.0, 0.0), l);
        }
        let nov = dot(n, v);
        let noh = min(0.99, dot(n, h));
        let loh = dot(l, h);

        let mut pdf = 0.0;
        let spec =
            eval_disney_specular_refraction(mat, diel_f, noh, nov, nol, voh, loh, eta, &mut pdf);

        bsdf = vec4f(spec.x, spec.y, spec.z, refract_w * pdf);
    }

    let m = dot(n, l).abs();

    bsdf.x *= m;
    bsdf.y *= m;
    bsdf.z *= m;

    (bsdf, l)
}
*/
// ---
/*
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BSDFMedium {
    pub type_: i32,
    pub density: f32,
    pub color: Vec3f,
    pub anisotropy: f32,
}

impl Default for BSDFMedium {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDFMedium {
    pub fn new() -> Self {
        Self {
            type_: 0,
            density: 0.0,
            color: Vec3f::zero(),
            anisotropy: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BSDFMaterial {
    pub base_color: Vec3f,
    pub opacity: f32,
    pub alpha_mode: i32,
    pub alpha_cutoff: f32,
    pub emission: Vec3f,
    pub anisotropic: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub subsurface: f32,
    pub specular_tint: f32,
    pub sheen: f32,
    pub sheen_tint: f32,
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    pub spec_trans: f32,
    pub ior: f32,
    pub ax: f32,
    pub ay: f32,
    pub medium: BSDFMedium,
}

impl Default for BSDFMaterial {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDFMaterial {
    pub fn new() -> Self {
        Self {
            base_color: Vec3f::new(0.5, 0.5, 0.5),
            opacity: 1.0,
            alpha_mode: 0,
            alpha_cutoff: 0.0,
            emission: Vec3f::zero(),
            anisotropic: 0.0,
            metallic: 0.0,
            roughness: 0.5,
            subsurface: 0.0,
            specular_tint: 0.0,
            sheen: 0.0,
            sheen_tint: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            spec_trans: 0.0,
            ior: 1.5,
            ax: 0.0,
            ay: 0.0,
            medium: BSDFMedium::new(),
        }
    }

    /// Mixes two materials.
    pub fn mix(&mut self, mat1: &BSDFMaterial, mat2: &BSDFMaterial, t: f32) {
        self.base_color = lerp(mat1.base_color, mat2.base_color, t);
        self.emission = lerp(mat1.emission, mat2.emission, t);
        self.anisotropic = lerp(mat1.anisotropic, mat2.anisotropic, t);
        self.metallic = lerp(mat1.metallic, mat2.metallic, t);
        self.roughness = lerp(mat1.roughness, mat2.roughness, t);
        self.subsurface = lerp(mat1.subsurface, mat2.subsurface, t);
        self.specular_tint = lerp(mat1.specular_tint, mat2.specular_tint, t);
        self.sheen = lerp(mat1.sheen, mat2.sheen, t);
        self.sheen_tint = lerp(mat1.sheen_tint, mat2.sheen_tint, t);
        self.clearcoat = lerp(mat1.clearcoat, mat2.clearcoat, t);
        self.clearcoat_roughness = lerp(mat1.clearcoat_roughness, mat2.clearcoat_roughness, t);
        self.spec_trans = lerp(mat1.spec_trans, mat2.spec_trans, t);
        self.ior = lerp(mat1.ior, mat2.ior, t);
    }
}
*/
pub struct BSDFLight {
    pub position: Vec3f,
    pub emission: Vec3f,
    pub u: Vec3f,
    pub v: Vec3f,
    pub radius: f32,
    pub area: f32,
    pub type_: f32,
}

pub struct BSDFState {
    pub depth: i32,
    pub eta: f32,
    pub hit_dist: f32,

    pub fhp: Vec3f,
    pub normal: Vec3f,
    pub ffnormal: Vec3f,
    pub tangent: Vec3f,
    pub bitangent: Vec3f,

    pub is_emitter: bool,

    pub tex_coord: Vec2f,
    pub mat: BSDFMaterial,
    pub medium: BSDFMedium,
}

impl Default for BSDFState {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDFState {
    pub fn new() -> Self {
        Self {
            depth: 0,
            eta: 0.0,
            hit_dist: 0.0,
            fhp: Vec3f::zero(),
            normal: Vec3f::zero(),
            ffnormal: Vec3f::zero(),
            tangent: Vec3f::zero(),
            bitangent: Vec3f::zero(),
            is_emitter: false,
            tex_coord: Vec2f::zero(),
            mat: BSDFMaterial::new(),
            medium: BSDFMedium::new(),
        }
    }
}

pub struct BSDFScatterSampleRec {
    pub l: Vec3f,
    pub f: Vec3f,
    pub pdf: f32,
}

impl Default for BSDFScatterSampleRec {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDFScatterSampleRec {
    pub fn new() -> Self {
        Self {
            l: Vec3f::zero(),
            f: Vec3f::zero(),
            pdf: 0.0,
        }
    }
}

pub struct BSDFLightSampleRec {
    pub normal: Vec3f,
    pub emission: Vec3f,
    pub direction: Vec3f,
    pub dist: f32,
    pub pdf: f32,
}

impl Default for BSDFLightSampleRec {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDFLightSampleRec {
    pub fn new() -> Self {
        Self {
            normal: Vec3f::zero(),
            emission: Vec3f::zero(),
            direction: Vec3f::zero(),
            dist: 0.0,
            pdf: 0.0,
        }
    }
}

pub fn face_forward(a: Vec3f, b: Vec3f) -> Vec3f {
    if dot(a, b) < 0.0 {
        -b
    } else {
        b
    }
}

pub fn luminance(c: Vec3f) -> f32 {
    0.212671 * c.x + 0.715160 * c.y + 0.072169 * c.z
}

pub fn gtr1(ndoth: f32, a: f32) -> f32 {
    if a >= 1.0 {
        return f32::inv_pi();
    }
    let a2 = a * a;
    let t = 1.0 + (a2 - 1.0) * ndoth * ndoth;
    (a2 - 1.0) / (f32::pi() * a2.ln() * t)
}

pub fn sample_gtr1(rgh: f32, r1: f32, r2: f32) -> Vec3f {
    let a = rgh.max(0.001);
    let a2 = a * a;

    let phi = r1 * f32::two_pi();

    let cos_theta = ((1.0 - a2.powf(1.0 - r2)) / (1.0 - a2)).sqrt();
    let sin_theta = (1.0 - (cos_theta * cos_theta)).sqrt().clamp(0.0, 1.0);
    let sin_phi = phi.sin();
    let cos_phi = phi.cos();

    Vec3f::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta)
}

pub fn gtr2(ndoth: f32, a: f32) -> f32 {
    let a2 = a * a;
    let t = 1.0 + (a2 - 1.0) * ndoth * ndoth;
    a2 / (f32::pi() * t * t)
}

pub fn sample_gtr2(rgh: f32, r1: f32, r2: f32) -> Vec3f {
    let a = rgh.max(0.001);

    let phi = r1 * f32::two_pi();

    let cos_theta = ((1.0 - r2) / (1.0 + (a * a - 1.0) * r2)).sqrt();
    let sin_theta = (1.0 - (cos_theta * cos_theta)).sqrt().clamp(0.0, 1.0);
    let sin_phi = phi.sin();
    let cos_phi = phi.cos();

    Vec3f::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta)
}

pub fn sample_ggx_vndf(v: Vec3f, ax: f32, ay: f32, r1: f32, r2: f32) -> Vec3f {
    let vh = normalize(Vec3f::new(ax * v.x, ay * v.y, v.z));

    let lensq = vh.x * vh.x + vh.y * vh.y;
    let tt1 = if lensq > 0.0 {
        Vec3f::new(-vh.y, vh.x, 0.0) * rsqrt(lensq)
    } else {
        Vec3f::new(1.0, 0.0, 0.0)
    };
    let tt2 = cross(vh, tt1);

    let r = r1.sqrt();
    let phi = 2.0 * f32::pi() * r2;
    let t1 = r * phi.cos();
    let t2 = r * phi.sin();
    let s = 0.5 * (1.0 + vh.z);
    let t2 = (1.0 - s) * (1.0 - t1 * t1).sqrt() + s * t2;

    let nh = t1 * tt1 + t2 * tt2 + (1.0 - t1 * t1 - t2 * t2).max(0.0).sqrt() * vh;

    normalize(Vec3f::new(ax * nh.x, ay * nh.y, nh.z.max(0.0)))
}

pub fn gtr2_aniso(ndoth: f32, hdox: f32, hdoy: f32, ax: f32, ay: f32) -> f32 {
    let a = hdox / ax;
    let b = hdoy / ay;
    let c = a * a + b * b + ndoth * ndoth;
    1.0 / (f32::pi() * ax * ay * c * c)
}

pub fn sample_gtr2_aniso(ax: f32, ay: f32, r1: f32, r2: f32) -> Vec3f {
    let phi = r1 * f32::two_pi();

    let sin_phi = ay * phi.sin();
    let cos_phi = ax * phi.cos();
    let tan_theta = (r2 / (1.0 - r2)).sqrt();

    Vec3f::new(tan_theta * cos_phi, tan_theta * sin_phi, 1.0)
}

pub fn smith_g(ndotv: f32, alpha_g: f32) -> f32 {
    let a = alpha_g * alpha_g;
    let b = ndotv * ndotv;
    (2.0 * ndotv) / (ndotv + (a + b - a * b).sqrt())
}

pub fn smith_g_aniso(ndotv: f32, vdotx: f32, vdoty: f32, ax: f32, ay: f32) -> f32 {
    let a = vdotx * ax;
    let b = vdoty * ay;
    let c = ndotv;
    (2.0 * ndotv) / (ndotv + (a * a + b * b + c * c).sqrt())
}

pub fn schlick_weight(u: f32) -> f32 {
    let m = (1.0 - u).clamp(0.0, 1.0);
    let m2 = m * m;
    m2 * m2 * m
}

pub fn dielectric_fresnel(cos_theta_i: f32, eta: f32) -> f32 {
    let sin_theta_t_sq = eta * eta * (1.0 - cos_theta_i * cos_theta_i);

    if sin_theta_t_sq > 1.0 {
        return 1.0;
    }

    let cos_theta_t = (1.0 - sin_theta_t_sq).max(0.0).sqrt();

    let rs = (eta * cos_theta_t - cos_theta_i) / (eta * cos_theta_t + cos_theta_i);
    let rp = (eta * cos_theta_i - cos_theta_t) / (eta * cos_theta_i + cos_theta_t);

    0.5 * (rs * rs + rp * rp)
}

pub fn cosine_sample_hemisphere(r1: f32, r2: f32) -> Vec3f {
    let r = r1.sqrt();
    let phi = f32::two_pi() * r2;
    let x = r * phi.cos();
    let y = r * phi.sin();
    let z = (1.0 - x * x - y * y).max(0.0).sqrt();
    Vec3f::new(x, y, z)
}

pub fn uniform_sample_hemisphere(r1: f32, r2: f32) -> Vec3f {
    let r = (1.0 - r1 * r1).max(0.0).sqrt();
    let phi = f32::two_pi() * r2;
    Vec3f::new(r * phi.cos(), r * phi.sin(), r1)
}

pub fn uniform_sample_sphere(r1: f32, r2: f32) -> Vec3f {
    let z = 1.0 - 2.0 * r1;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = f32::two_pi() * r2;
    Vec3f::new(r * phi.cos(), r * phi.sin(), z)
}

pub fn power_heuristic(a: f32, b: f32) -> f32 {
    let t = a * a;
    t / (b * b + t)
}

pub fn onb(n: Vec3f, t: &mut Vec3f, b: &mut Vec3f) {
    let up = if n.z.abs() < 0.9999999 {
        Vec3f::new(0.0, 0.0, 1.0)
    } else {
        Vec3f::new(1.0, 0.0, 0.0)
    };
    *t = normalize(cross(n, up));
    *b = cross(n, *t);
}

pub fn sample_sphere_light(
    light: &BSDFLight,
    scatter_pos: Vec3f,
    light_sample: &mut BSDFLightSampleRec,
    num_of_lights: i32,
    rng: &mut ThreadRng,
    max_distance: f32,
) {
    let r1 = rng.gen();
    let r2 = rng.gen();

    let mut sphere_center_to_surface = scatter_pos - light.position;
    let dist_to_sphere_center = length(sphere_center_to_surface);

    sphere_center_to_surface /= dist_to_sphere_center;
    let sampled_dir = uniform_sample_hemisphere(r1, r2);
    let mut t = Vec3f::zero();
    let mut b = Vec3f::zero();
    onb(sphere_center_to_surface, &mut t, &mut b);
    let sampled_dir =
        t * sampled_dir.x + b * sampled_dir.y + sphere_center_to_surface * sampled_dir.z;

    let light_surface_pos = light.position + sampled_dir * light.radius;

    light_sample.direction = light_surface_pos - scatter_pos;
    light_sample.dist = length(light_sample.direction);
    let dist_sq = light_sample.dist * light_sample.dist;

    light_sample.direction /= light_sample.dist;
    light_sample.normal = normalize(light_surface_pos - light.position);
    light_sample.emission = light.emission * num_of_lights as f32;
    light_sample.pdf = dist_sq
        / (light.area * /*0.5*/ max_distance * dot(light_sample.normal, light_sample.direction).abs());
}

pub fn sample_rect_light(
    light: &BSDFLight,
    scatter_pos: Vec3f,
    light_sample: &mut BSDFLightSampleRec,
    num_of_lights: i32,
    rng: &mut ThreadRng,
) {
    let r1: f32 = rng.gen();
    let r2: f32 = rng.gen();

    let light_surface_pos = light.position + light.u * r1 + light.v * r2;
    light_sample.direction = light_surface_pos - scatter_pos;
    light_sample.dist = length(light_sample.direction);
    let dist_sq = light_sample.dist * light_sample.dist;
    light_sample.direction /= light_sample.dist;
    light_sample.normal = normalize(cross(light.u, light.v));
    light_sample.emission = light.emission * num_of_lights as f32;
    light_sample.pdf =
        dist_sq / (light.area * dot(light_sample.normal, light_sample.direction).abs());
}

pub fn sample_distant_light(
    light: &BSDFLight,
    scatter_pos: Vec3f,
    light_sample: &mut BSDFLightSampleRec,
    num_of_lights: i32,
) {
    light_sample.direction = light.position;
    light_sample.normal = normalize(scatter_pos - light.position);
    light_sample.emission = light.emission * num_of_lights as f32;
    light_sample.dist = f32::INFINITY;
    light_sample.pdf = 1.0;
}

// fn sample_one_light(light: &Light, scatter_pos: Vec3f, light_sample: &mut LightSampleRec) {
//     let light_type = light.type_ as i32;

//     if light_type == QUAD_LIGHT {
//         sample_rect_light(light, scatter_pos, light_sample);
//     } else if light_type == SPHERE_LIGHT {
//         sample_sphere_light(light, scatter_pos, light_sample);
//     } else {
//         sample_distant_light(light, scatter_pos, light_sample);
//     }
// }

pub fn sample_hg(v: Vec3f, g: f32, r1: f32, r2: f32) -> Vec3f {
    let cos_theta = if g.abs() < 0.001 {
        1.0 - 2.0 * r2
    } else {
        let sqr_term = (1.0 - g * g) / (1.0 + g - 2.0 * g * r2);
        -(1.0 + g * g - sqr_term * sqr_term) / (2.0 * g)
    };

    let phi = r1 * f32::two_pi();
    let sin_theta = (1.0 - (cos_theta * cos_theta)).sqrt().clamp(0.0, 1.0);
    let sin_phi = phi.sin();
    let cos_phi = phi.cos();

    let mut v1 = Vec3f::zero();
    let mut v2 = Vec3f::zero();
    onb(v, &mut v1, &mut v2);

    sin_theta * cos_phi * v1 + sin_theta * sin_phi * v2 + cos_theta * v
}

#[allow(clippy::excessive_precision)]
pub fn phase_hg(cos_theta: f32, g: f32) -> f32 {
    let denom = 1.0 + g * g + 2.0 * g * cos_theta;
    /*INV_4_PI*/
    0.07957747154594766 * (1.0 - g * g) / (denom * denom.sqrt())
}

pub fn to_world(x: Vec3f, y: Vec3f, z: Vec3f, v: Vec3f) -> Vec3f {
    v.x * x + v.y * y + v.z * z
}

pub fn to_local(x: Vec3f, y: Vec3f, z: Vec3f, v: Vec3f) -> Vec3f {
    Vec3f::new(dot(v, x), dot(v, y), dot(v, z))
}

pub fn tint_colors(
    mat: &BSDFMaterial,
    eta: f32,
    f0: &mut f32,
    csheen: &mut Vec3f,
    cspec0: &mut Vec3f,
) {
    let lum = luminance(mat.base_color);
    let ctint = if lum > 0.0 {
        mat.base_color / lum
    } else {
        Vec3f::new(1.0, 1.0, 1.0)
    };

    *f0 = (1.0 - eta) / (1.0 + eta);
    *f0 *= *f0;

    *cspec0 = *f0 * lerp(Vec3f::new(1.0, 1.0, 1.0), ctint, mat.specular_tint);
    *csheen = lerp(Vec3f::new(1.0, 1.0, 1.0), ctint, mat.sheen_tint);
}

pub fn eval_disney_diffuse(
    mat: &BSDFMaterial,
    csheen: Vec3f,
    v: Vec3f,
    l: Vec3f,
    h: Vec3f,
    pdf: &mut f32,
) -> Vec3f {
    *pdf = 0.0;
    if l.z <= 0.0 {
        return Vec3f::zero();
    }

    let l_dot_h = dot(l, h);

    let rr = 2.0 * mat.roughness * l_dot_h * l_dot_h;

    // Diffuse
    let fl = schlick_weight(l.z);
    let fv = schlick_weight(v.z);
    let fretro = rr * (fl + fv + fl * fv * (rr - 1.0));
    let fd = (1.0 - 0.5 * fl) * (1.0 - 0.5 * fv);

    // Fake subsurface
    let fss90 = 0.5 * rr;
    let fss = lerp(1.0, fss90, fl) * lerp(1.0, fss90, fv);
    let ss = 1.25 * (fss * (1.0 / (l.z + v.z) - 0.5) + 0.5);

    // Sheen
    let fh = schlick_weight(l_dot_h);
    let fsheen = fh * mat.sheen * csheen;

    *pdf = l.z * f32::inv_pi();
    f32::inv_pi() * mat.base_color * lerp(fd + fretro, ss, mat.subsurface) + fsheen
}

pub fn eval_microfacet_reflection(
    mat: &BSDFMaterial,
    v: Vec3f,
    l: Vec3f,
    h: Vec3f,
    f: Vec3f,
    pdf: &mut f32,
) -> Vec3f {
    *pdf = 0.0;
    if l.z <= 0.0 {
        return Vec3f::zero();
    }

    let d = gtr2_aniso(h.z, h.x, h.y, mat.ax, mat.ay);
    let g1 = smith_g_aniso(v.z.abs(), v.x, v.y, mat.ax, mat.ay);
    let g2 = g1 * smith_g_aniso(l.z.abs(), l.x, l.y, mat.ax, mat.ay);

    *pdf = g1 * d / (4.0 * v.z);
    f * d * g2 / (4.0 * l.z * v.z)
}

pub fn eval_microfacet_refraction(
    mat: &BSDFMaterial,
    eta: f32,
    v: Vec3f,
    l: Vec3f,
    h: Vec3f,
    f: Vec3f,
    pdf: &mut f32,
) -> Vec3f {
    *pdf = 0.0;
    if l.z >= 0.0 {
        return Vec3f::zero();
    }

    let l_dot_h = dot(l, h);
    let v_dot_h = dot(v, h);

    let d = gtr2_aniso(h.z, h.x, h.y, mat.ax, mat.ay);
    let g1 = smith_g_aniso(v.z.abs(), v.x, v.y, mat.ax, mat.ay);
    let g2 = g1 * smith_g_aniso(l.z.abs(), l.x, l.y, mat.ax, mat.ay);
    let denom = l_dot_h + v_dot_h * eta;
    let denom = denom * denom;
    let eta2 = eta * eta;
    let jacobian = l_dot_h.abs() / denom;

    *pdf = g1 * v_dot_h.max(0.0) * d * jacobian / v.z;
    powf(mat.base_color, 0.5) * (1.0 - f) * d * g2 * v_dot_h.abs() * jacobian * eta2
        / (l.z * v.z).abs()
}

pub fn eval_clearcoat(mat: &BSDFMaterial, v: Vec3f, l: Vec3f, h: Vec3f, pdf: &mut f32) -> Vec3f {
    *pdf = 0.0;
    if l.z <= 0.0 {
        return Vec3f::zero();
    }

    let v_dot_h = dot(v, h);

    let f = lerp(0.04, 1.0, schlick_weight(v_dot_h));
    let d = gtr1(h.z, mat.clearcoat_roughness);
    let g = smith_g(l.z, 0.25) * smith_g(v.z, 0.25);
    let jacobian = 1.0 / (4.0 * v_dot_h);

    *pdf = d * h.z * jacobian;
    Vec3f::new(f, f, f) * d * g
}

pub fn disney_sample(
    state: &BSDFState,
    v: Vec3f,
    n: Vec3f,
    ll: &mut Vec3f,
    pdf: &mut f32,
    rng: &mut ThreadRng,
) -> Vec3f {
    *pdf = 0.0;

    let r1 = rng.gen();
    let r2 = rng.gen();

    // TODO: Tangent and bitangent should be calculated from mesh (provided, the mesh has proper uvs)
    let mut t = Vec3f::zero();
    let mut b = Vec3f::zero();
    onb(n, &mut t, &mut b);

    // Transform to shading space to simplify operations (NDotL = L.z; NDotV = V.z; NDotH = H.z)
    let v = to_local(t, b, n, v);

    // Tint colors
    let mut csheen = Vec3f::zero();
    let mut cspec0 = Vec3f::zero();
    let mut f0 = 0.0;
    tint_colors(&state.mat, state.eta, &mut f0, &mut csheen, &mut cspec0);

    // Model weights
    let dielectric_wt = (1.0 - state.mat.metallic) * (1.0 - state.mat.spec_trans);
    let metal_wt = state.mat.metallic;
    let glass_wt = (1.0 - state.mat.metallic) * state.mat.spec_trans;

    // Lobe probabilities
    let schlick_wt = schlick_weight(v.z);

    let diff_pr = dielectric_wt * luminance(state.mat.base_color);
    let dielectric_pr =
        dielectric_wt * luminance(lerp(cspec0, Vec3f::new(1.0, 1.0, 1.0), schlick_wt));
    let metal_pr = metal_wt
        * luminance(lerp(
            state.mat.base_color,
            Vec3f::new(1.0, 1.0, 1.0),
            schlick_wt,
        ));
    let glass_pr = glass_wt;
    let clear_ct_pr = 0.25 * state.mat.clearcoat;

    // Normalize probabilities
    let inv_total_wt = 1.0 / (diff_pr + dielectric_pr + metal_pr + glass_pr + clear_ct_pr);
    let diff_pr = diff_pr * inv_total_wt;
    let dielectric_pr = dielectric_pr * inv_total_wt;
    let metal_pr = metal_pr * inv_total_wt;
    let glass_pr = glass_pr * inv_total_wt;
    let clear_ct_pr = clear_ct_pr * inv_total_wt;

    // CDF of the sampling probabilities
    let cdf = [
        diff_pr,
        diff_pr + dielectric_pr,
        diff_pr + dielectric_pr + metal_pr,
        diff_pr + dielectric_pr + metal_pr + glass_pr,
        diff_pr + dielectric_pr + metal_pr + glass_pr + clear_ct_pr,
    ];

    // Sample a lobe based on its importance
    let r3: f32 = rng.gen();

    let l = if r3 < cdf[0] {
        cosine_sample_hemisphere(r1, r2)
    } else if r3 < cdf[2] {
        let mut h = sample_ggx_vndf(v, state.mat.ax, state.mat.ay, r1, r2);

        if h.z < 0.0 {
            h = -h;
        }

        normalize(Vec3f::reflect(-v, h))
    } else if r3 < cdf[3] {
        let mut h = sample_ggx_vndf(v, state.mat.ax, state.mat.ay, r1, r2);
        let f = dielectric_fresnel(dot(v, h).abs(), state.eta);

        if h.z < 0.0 {
            h = -h;
        }

        let r3 = (r3 - cdf[2]) / (cdf[3] - cdf[2]);

        if r3 < f {
            normalize(Vec3f::reflect(-v, h))
        } else {
            normalize(Vec3f::refract(-v, h, state.eta))
        }
    } else {
        let mut h = sample_gtr1(state.mat.clearcoat_roughness, r1, r2);

        if h.z < 0.0 {
            h = -h;
        }

        normalize(Vec3f::reflect(-v, h))
    };

    let l = to_world(t, b, n, l);
    let v = to_world(t, b, n, v);

    *ll = l;

    disney_eval(state, v, n, l, pdf)
}

pub fn disney_eval(state: &BSDFState, vv: Vec3f, nn: Vec3f, ll: Vec3f, pdf: &mut f32) -> Vec3f {
    *pdf = 0.0;
    let mut f = Vec3f::zero();

    // TODO: Tangent and bitangent should be calculated from mesh (provided, the mesh has proper uvs)
    let mut t = Vec3f::zero();
    let mut b = Vec3f::zero();
    onb(nn, &mut t, &mut b);

    // Transform to shading space to simplify operations (NDotL = L.z; NDotV = V.z; NDotH = H.z)
    let v = to_local(t, b, nn, vv);
    let l = to_local(t, b, nn, ll);

    let mut h = if l.z > 0.0 {
        normalize(l + v)
    } else {
        normalize(l + v * state.eta)
    };

    if h.z < 0.0 {
        h = -h;
    }

    // Tint colors
    let mut csheen = Vec3f::zero();
    let mut cspec0 = Vec3f::zero();
    let mut f0 = 0.0;
    tint_colors(&state.mat, state.eta, &mut f0, &mut csheen, &mut cspec0);

    // Model weights
    let dielectric_wt = (1.0 - state.mat.metallic) * (1.0 - state.mat.spec_trans);
    let metal_wt = state.mat.metallic;
    let glass_wt = (1.0 - state.mat.metallic) * state.mat.spec_trans;

    // Lobe probabilities
    let schlick_wt = schlick_weight(v.z);

    let diff_pr = dielectric_wt * luminance(state.mat.base_color);
    let dielectric_pr =
        dielectric_wt * luminance(lerp(cspec0, Vec3f::new(1.0, 1.0, 1.0), schlick_wt));
    let metal_pr = metal_wt
        * luminance(lerp(
            state.mat.base_color,
            Vec3f::new(1.0, 1.0, 1.0),
            schlick_wt,
        ));
    let glass_pr = glass_wt;
    let clear_ct_pr = 0.25 * state.mat.clearcoat;

    // Normalize probabilities
    let inv_total_wt = 1.0 / (diff_pr + dielectric_pr + metal_pr + glass_pr + clear_ct_pr);
    let diff_pr = diff_pr * inv_total_wt;
    let dielectric_pr = dielectric_pr * inv_total_wt;
    let metal_pr = metal_pr * inv_total_wt;
    let glass_pr = glass_pr * inv_total_wt;
    let clear_ct_pr = clear_ct_pr * inv_total_wt;

    let reflect = l.z * v.z > 0.0;

    let mut tmp_pdf = 0.0;
    let v_dot_h = dot(v, h).abs();

    if diff_pr > 0.0 && reflect {
        f += eval_disney_diffuse(&state.mat, csheen, v, l, h, &mut tmp_pdf) * dielectric_wt;
        *pdf += tmp_pdf * diff_pr;
    }

    if dielectric_pr > 0.0 && reflect {
        let ff = (dielectric_fresnel(v_dot_h, 1.0 / state.mat.ior) - f0) / (1.0 - f0);

        f += eval_microfacet_reflection(
            &state.mat,
            v,
            l,
            h,
            lerp(cspec0, Vec3f::new(1.0, 1.0, 1.0), ff),
            &mut tmp_pdf,
        ) * dielectric_wt;
        *pdf += tmp_pdf * dielectric_pr;
    }

    if metal_pr > 0.0 && reflect {
        let ff = lerp(
            state.mat.base_color,
            Vec3f::new(1.0, 1.0, 1.0),
            schlick_weight(v_dot_h),
        );

        f += eval_microfacet_reflection(&state.mat, v, l, h, ff, &mut tmp_pdf) * metal_wt;
        *pdf += tmp_pdf * metal_pr;
    }

    if glass_pr > 0.0 {
        let ff = dielectric_fresnel(v_dot_h, state.eta);

        if reflect {
            f += eval_microfacet_reflection(
                &state.mat,
                v,
                l,
                h,
                Vec3f::new(ff, ff, ff),
                &mut tmp_pdf,
            ) * glass_wt;
            *pdf += tmp_pdf * glass_pr * ff;
        } else {
            f += eval_microfacet_refraction(
                &state.mat,
                state.eta,
                v,
                l,
                h,
                Vec3f::new(ff, ff, ff),
                &mut tmp_pdf,
            ) * glass_wt;
            *pdf += tmp_pdf * glass_pr * (1.0 - ff);
        }
    }

    if clear_ct_pr > 0.0 && reflect {
        f += eval_clearcoat(&state.mat, v, l, h, &mut tmp_pdf) * 0.25 * state.mat.clearcoat;
        *pdf += tmp_pdf * clear_ct_pr;
    }

    f * l.z.abs()
}
