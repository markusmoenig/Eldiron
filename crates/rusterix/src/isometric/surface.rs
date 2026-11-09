//! Cached geometry, never illumination. Orthographic pixel coordinates and a
//! height reconstruct a world position; octahedral normals retain both hemispheres.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use vek::Vec3;
const STRIDE: usize = 8;
const MAX_BYTES: usize = 1024 * 1024 * STRIDE;
fn serialize<S: serde::Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&STANDARD.encode(v))
}
fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let s = String::deserialize(d)?;
    if s.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err(serde::de::Error::custom("Iso geometry buffer exceeds 8 MB"));
    }
    let data = STANDARD.decode(s).map_err(serde::de::Error::custom)?;
    if data.len() > MAX_BYTES || data.len() % STRIDE != 0 {
        return Err(serde::de::Error::custom("Invalid iso geometry buffer"));
    }
    Ok(data)
}
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct SurfaceFrame {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<denrim_materials::geometry::effects::ParticleEffect>,
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    pub data: Vec<u8>,
    /// Height intervals from the same alpha-tested mesh, used by live shadow rays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowGrid>,
    /// Actual alpha-tested recipe triangles for GPU shadow maps. Old caches may
    /// still use columns until their editable recipe is synchronized again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_mesh: Option<scenevm::isometric::ShadowGeometry>,
}
fn one() -> usize {
    1
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShadowGrid {
    #[serde(default = "one")]
    pub layers: usize,
    #[serde(default)]
    pub emissive: bool,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub width: usize,
    pub height: usize,
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    pub data: Vec<u8>,
}
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub normal: Vec3<f32>,
    pub height: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub emission: f32,
}
fn decode(x: u8, y: u8) -> Vec3<f32> {
    let a = x as f32 / 255. * 2. - 1.;
    let b = y as f32 / 255. * 2. - 1.;
    let mut n = Vec3::new(a, b, 1. - a.abs() - b.abs());
    if n.z < 0. {
        n.x = (1. - b.abs()) * a.signum();
        n.y = (1. - a.abs()) * b.signum();
    }
    n.normalized()
}
fn normals() -> &'static [Vec3<f32>] {
    static LUT: OnceLock<Vec<Vec3<f32>>> = OnceLock::new();
    LUT.get_or_init(|| {
        (0..65536)
            .map(|i| decode((i >> 8) as u8, i as u8))
            .collect()
    })
}
/// Two words shared with SceneVM: oct normal + R/M/E, then world height.
pub fn packed(
    normal: Vec3<f32>,
    height: f32,
    roughness: f32,
    metallic: f32,
    emission: f32,
) -> [u32; 2] {
    let n = normal / (normal.x.abs() + normal.y.abs() + normal.z.abs()).max(1e-8);
    let (u, v) = if n.z >= 0. {
        (n.x, n.y)
    } else {
        (
            (1. - n.y.abs()) * n.x.signum(),
            (1. - n.x.abs()) * n.y.signum(),
        )
    };
    let x = ((u * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u32;
    let y = ((v * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u32;
    [
        x | (y << 8)
            | (((roughness.clamp(0., 1.) * 15.).round() as u32) << 16)
            | (((metallic.clamp(0., 1.) * 15.).round() as u32) << 20)
            | (((emission.clamp(0., 1.) * 255.).round() as u32) << 24),
        height.to_bits() & 0xffffff00,
    ]
}
impl SurfaceFrame {
    pub fn new(pixels: usize) -> Self {
        Self {
            data: vec![0; pixels * STRIDE],
            shadow: None,
            shadow_mesh: None,
            effects: Vec::new(),
        }
    }
    pub fn set(
        &mut self,
        i: usize,
        normal: Vec3<f32>,
        height: f32,
        roughness: f32,
        metallic: f32,
        emission: f32,
    ) {
        let Some(d) = self.data.get_mut(i * STRIDE..i * STRIDE + STRIDE) else {
            return;
        };
        let n = normal.normalized();
        let n = n / (n.x.abs() + n.y.abs() + n.z.abs()).max(1e-8);
        let (u, v) = if n.z >= 0. {
            (n.x, n.y)
        } else {
            (
                (1. - n.y.abs()) * n.x.signum(),
                (1. - n.x.abs()) * n.y.signum(),
            )
        };
        d[0] = ((u * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u8;
        d[1] = ((v * 0.5 + 0.5) * 255.).round().clamp(0., 255.) as u8;
        d[2..6].copy_from_slice(&height.to_le_bytes());
        d[6] = ((roughness.clamp(0., 1.) * 15.).round() as u8)
            | ((metallic.clamp(0., 1.) * 15.).round() as u8) << 4;
        d[7] = (emission.clamp(0., 1.) * 255.).round() as u8;
    }
    pub fn packed(&self, i: usize, base: f32, scale: f32) -> Option<[u32; 2]> {
        let at = i.checked_mul(8)?;
        let d = self.data.get(at..at + 8)?;
        let height = f32::from_le_bytes(d[2..6].try_into().ok()?) * scale + base;
        Some([
            u32::from_le_bytes([d[0], d[1], d[6], d[7]]),
            height.to_bits() & 0xffffff00,
        ])
    }
    pub fn sample(&self, i: usize) -> Option<Sample> {
        let p = i.checked_mul(STRIDE)?;
        let d = self.data.get(p..p + STRIDE)?;
        let height = f32::from_le_bytes(d[2..6].try_into().ok()?);
        if !height.is_finite() {
            return None;
        }
        Some(Sample {
            normal: normals()[usize::from(d[0]) * 256 + usize::from(d[1])],
            height,
            roughness: (d[6] & 15) as f32 / 15.,
            metallic: (d[6] >> 4) as f32 / 15.,
            emission: d[7] as f32 / 255.,
        })
    }
    pub fn pad(&mut self, old: [usize; 2], new: [usize; 2], offset: [usize; 2]) {
        let mut data = vec![0; new[0] * new[1] * STRIDE];
        for y in 0..old[1] {
            let at = ((y + offset[1]) * new[0] + offset[0]) * STRIDE;
            data[at..at + old[0] * STRIDE]
                .copy_from_slice(&self.data[y * old[0] * STRIDE..(y + 1) * old[0] * STRIDE]);
        }
        self.data = data;
    }
}
impl ShadowGrid {
    pub fn interval(&self, x: usize, z: usize) -> Option<[f32; 2]> {
        self.interval_layer(x, z, 0)
    }
    pub fn interval_layer(&self, x: usize, z: usize, layer: usize) -> Option<[f32; 2]> {
        if x >= self.width || z >= self.height || layer >= self.layers {
            return None;
        }
        let at = z
            .checked_mul(self.width)?
            .checked_add(x)?
            .checked_mul(self.layers)?
            .checked_add(layer)?
            .checked_mul(8)?;
        let d = self.data.get(at..at + 8)?;
        let lo = f32::from_le_bytes(d[..4].try_into().ok()?);
        let hi = f32::from_le_bytes(d[4..].try_into().ok()?);
        (lo.is_finite() && hi.is_finite() && lo <= hi).then_some([lo, hi])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normals_heights_materials_and_shadow_intervals_roundtrip() {
        let mut f = SurfaceFrame::new(6);
        for (i, n) in [
            Vec3::unit_x(),
            -Vec3::unit_x(),
            Vec3::unit_y(),
            -Vec3::unit_y(),
            Vec3::unit_z(),
            -Vec3::unit_z(),
        ]
        .into_iter()
        .enumerate()
        {
            f.set(i, n, 2.75, 0.8, 0.2, 0.4);
            assert!(f.sample(i).unwrap().normal.dot(n) > 0.999);
        }
        let d: SurfaceFrame = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(f, d);
        assert_eq!(d.sample(0).unwrap().height, 2.75);
        assert!(serde_json::from_str::<SurfaceFrame>(r#"{"data":"AA=="}"#).is_err());
    }
}
