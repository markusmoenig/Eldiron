//! Derived terrain paint, shared by editor and game. Coordinates are world X/Z;
//! height and camera changes never move the authored texels.
use rustc_hash::FxHashMap;
use std::sync::Arc;

pub struct StampSprite {
    pub surface: Option<super::surface::SurfaceFrame>,
    pub texture: crate::Texture,
    pub anchor: [f32; 2],
}

#[derive(Clone)]
pub struct TerrainStamp {
    pub id: uuid::Uuid,
    pub order: u64,
    pub position: [f32; 2],
    pub raster_key: u64,
    pub sprite: Arc<StampSprite>,
}

pub const TERRAIN_PAINT_GEO: [u32; 4] = [10, 0, 0, 0];
pub const CHUNK_SIZE: i32 = 64;
pub const PIXELS_PER_UNIT: f32 = 32.;

#[derive(Clone, Default)]
pub struct TerrainPaintChunk {
    pub color: Vec<u8>,
    pub material: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct TerrainPaint {
    pub key: u64,
    pub ground_key: u64,
    pub chunks: FxHashMap<[i32; 2], TerrainPaintChunk>,
    pub stamps: Vec<TerrainStamp>,
}

impl TerrainPaint {
    pub fn sample(&self, world: [f32; 2]) -> Option<([u8; 4], [u8; 4])> {
        let pixel = world.map(|v| (v * PIXELS_PER_UNIT).round() as i32);
        let origin = pixel.map(|v| v.div_euclid(CHUNK_SIZE) * CHUNK_SIZE);
        let chunk = self.chunks.get(&origin)?;
        let local = [pixel[0] - origin[0], pixel[1] - origin[1]];
        let at = (local[1] * CHUNK_SIZE + local[0]) as usize * 4;
        Some((
            chunk.color.get(at..at + 4)?.try_into().ok()?,
            chunk.material.get(at..at + 4)?.try_into().ok()?,
        ))
    }

    /// Blend the painted library material with the terrain's MZ surface.
    pub fn surface_material(&self, base: [f32; 3], world: [f32; 2]) -> [f32; 3] {
        let Some((_, material)) = self.sample(world) else {
            return base;
        };
        let (r,m,e,replace) = match material[0] {
            254 => { let [r,m,_,e] = crate::material_library::MaterialDefinition::from_id(material[1]).rmoe_values(); (r,m,e,material[2]!=0) }
            64..=123 | 128..=187 | 252 | 253 => { let (r,m,_,e) = crate::Texture::decode_material_bytes([material[1],material[2]]); (r,m,e,material[0]==252 || (128..=187).contains(&material[0])) }
            _ => return base,
        };
        let amount = if replace && material[3] > 0 {
            1.
        } else {
            material[3] as f32 / 255.
        };
        let top = [r, m, e];
        std::array::from_fn(|i| base[i] * (1. - amount) + top[i] * amount)
    }

    pub fn composite(&self, base: [u8; 4], world: [f32; 2]) -> [u8; 4] {
        let Some((paint, material)) = self.sample(world) else {
            return base;
        };
        let alpha = paint[3] as f32 / 255.;
        let coverage = if material[0] == 252 || (128..=187).contains(&material[0]) || material[0] == 254 && material[2] != 0 {
            material[3] as f32 / 255.
        } else {
            alpha
        };
        let under = base[3] as f32 / 255. * (1. - coverage);
        let total = under + alpha;
        if total <= 0. {
            return [0; 4];
        }
        std::array::from_fn(|i| {
            if i == 3 {
                (total * 255.).round().min(255.) as u8
            } else {
                ((base[i] as f32 * under + paint[i] as f32 * alpha) / total).round() as u8
            }
        })
    }
}
