use crate::{Assets, Map, Pixel, Texture, Tile, ValueContainer};
use theframework::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseTarget {
    RGB,
    Hue,
    Luminance,
}

impl From<i32> for NoiseTarget {
    fn from(value: i32) -> Self {
        match value {
            0 => NoiseTarget::RGB,
            1 => NoiseTarget::Hue,
            2 => NoiseTarget::Luminance,
            _ => NoiseTarget::RGB, // Default to RGB if value is invalid
        }
    }
}

/// Serializable procedural material, generated once into a seamless atlas tile.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct NoiseMaterial {
    pub voronoi: bool,
    pub scale: u32,
    pub seed: u32,
    pub low: u8,
    pub high: u8,
    #[serde(default)]
    pub low_color: Option<[u8; 4]>,
    #[serde(default)]
    pub high_color: Option<[u8; 4]>,
}

impl Default for NoiseMaterial {
    fn default() -> Self {
        Self {
            voronoi: false,
            scale: 8,
            seed: 0,
            low: 65,
            high: 115,
            low_color: None,
            high_color: None,
        }
    }
}

impl NoiseMaterial {
    pub fn colors(&self) -> ([u8; 4], [u8; 4]) {
        (
            self.low_color
                .unwrap_or([self.low, self.low, self.low, 255]),
            self.high_color
                .unwrap_or([self.high, self.high, self.high, 255]),
        )
    }

    pub fn tile_id(&self) -> Uuid {
        if self.low_color.is_some() || self.high_color.is_some() {
            let (low, high) = self.colors();
            let packed = self.seed as u128
                | ((self.scale.clamp(1, 32) as u128) << 32)
                | ((self.voronoi as u128) << 38)
                | ((u32::from_be_bytes(low) as u128) << 40)
                | ((u32::from_be_bytes(high) as u128) << 72);
            return Uuid::from_u128((0x4e4f49u128 << 104) | packed);
        }
        let packed = self.seed as u128
            | ((self.scale.clamp(1, 32) as u128) << 32)
            | ((self.low as u128) << 40)
            | ((self.high as u128) << 48)
            | ((self.voronoi as u128) << 56);
        Uuid::from_u128(0x4e4f_4953_4500_0000_0000_0000_0000_0000u128 | packed)
    }

    fn hash(&self, x: i32, y: i32, channel: u32) -> f32 {
        let period = self.scale.clamp(1, 32) as i32;
        let mut h = self.seed
            ^ (x.rem_euclid(period) as u32).wrapping_mul(0x9e3779b9)
            ^ (y.rem_euclid(period) as u32).wrapping_mul(0x85ebca6b)
            ^ channel;
        h ^= h >> 16;
        h = h.wrapping_mul(0x7feb352d);
        h ^= h >> 15;
        h = h.wrapping_mul(0x846ca68b);
        h ^= h >> 16;
        h as f32 / u32::MAX as f32
    }

    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let scale = self.scale.clamp(1, 32) as f32;
        let x = u * scale;
        let y = v * scale;
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        if self.voronoi {
            let mut distance = f32::INFINITY;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let px = dx as f32 + self.hash(ix + dx, iy + dy, 0);
                    let py = dy as f32 + self.hash(ix + dx, iy + dy, 0xa511e9b3);
                    distance = distance.min((px - fx).powi(2) + (py - fy).powi(2));
                }
            }
            distance.sqrt().clamp(0.0, 1.0)
        } else {
            let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
            let sx = smooth(fx);
            let sy = smooth(fy);
            let a = self.hash(ix, iy, 0) * (1.0 - sx) + self.hash(ix + 1, iy, 0) * sx;
            let b = self.hash(ix, iy + 1, 0) * (1.0 - sx) + self.hash(ix + 1, iy + 1, 0) * sx;
            a * (1.0 - sy) + b * sy
        }
    }

    pub fn to_tile(&self, size: usize) -> Tile {
        let size = size.clamp(2, 256);
        let mut data = Vec::with_capacity(size * size * 4);
        for y in 0..size {
            for x in 0..size {
                let value = self.sample(x as f32 / size as f32, y as f32 / size as f32);
                let (low, high) = self.colors();
                for channel in 0..4 {
                    data.push(
                        (low[channel] as f32 + (high[channel] as f32 - low[channel] as f32) * value)
                            .round() as u8,
                    );
                }
            }
        }
        let mut tile = Tile::from_texture(Texture::new(data, size, size));
        tile.id = self.tile_id();
        tile
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub enum PixelSource {
    #[default]
    Off,
    TileId(Uuid),
    TileGroup(Uuid),
    TileGroupMember {
        group_id: Uuid,
        member_index: u16,
    },
    ProceduralTile(Uuid),
    PaletteIndex(u16),
    MaterialId(Uuid),
    Sequence(String),
    EntityTile(u32, u32),
    ItemTile(u32, u32),
    Color(TheColor),
    Noise(NoiseMaterial),
    #[serde(rename = "ShapeFXGraphId")]
    LegacyShapeFXGraphId(Uuid),
    StaticTileIndex(u16),
    DynamicTileIndex(u16),
    Pixel(Pixel),
}

use PixelSource::*;

impl PixelSource {
    pub fn palette_tile_uuid(index: u16) -> Uuid {
        Uuid::from_u128(0x50414C455454455F0000000000000000u128 | index as u128)
    }

    pub fn color_tile_uuid(color: &TheColor) -> Uuid {
        let rgba = color.to_u8_array();
        let packed = u32::from_be_bytes(rgba);
        Uuid::from_u128(0x434F_4C4F_525F_0000_0000_0000_0000_0000u128 | packed as u128)
    }

    pub fn render_tile_id(&self, assets: &Assets) -> Option<Uuid> {
        match self {
            TileId(id) | MaterialId(id) => Some(*id),
            PaletteIndex(index) => Some(Self::palette_tile_uuid(*index)),
            Color(color) => Some(Self::color_tile_uuid(color)),
            Noise(noise) => Some(noise.tile_id()),
            _ => self.tile_from_tile_list(assets).map(|tile| tile.id),
        }
    }

    fn synthetic_palette_tile(assets: &Assets, index: u16) -> Option<Tile> {
        let col = assets.palette.colors.get(index as usize)?.as_ref()?;
        let mut tile = Tile::from_texture(Texture::from_color(col.to_u8_array()));
        tile.id = Self::palette_tile_uuid(index);
        let material_id = assets
            .palette_material_ids
            .get(index as usize)
            .copied()
            .unwrap_or(0);
        for texture in &mut tile.textures {
            texture.set_material_id_all(material_id);
        }
        Some(tile)
    }

    /// Generate a tile from the given PixelValue
    pub fn to_tile(
        &self,
        assets: &Assets,
        size: usize,
        values: &ValueContainer,
        _map: &Map,
    ) -> Option<Tile> {
        match self {
            Noise(noise) => Some(noise.to_tile(size)),
            TileId(id) => assets.tiles.get(id).cloned(),
            TileGroup(_) | TileGroupMember { .. } | ProceduralTile(_) => None,
            PaletteIndex(index) => {
                let tile_id = Self::palette_tile_uuid(*index);
                assets
                    .tiles
                    .get(&tile_id)
                    .cloned()
                    .or_else(|| Self::synthetic_palette_tile(assets, *index))
            }
            MaterialId(id) => assets.materials.get(id).cloned(),
            Color(color) => {
                let apply_to: NoiseTarget = values.get_int_default("noise_target", 0).into();
                let noise_intensity = values.get_float_default("noise_intensity", 0.0);
                let pixelization = values.get_int_default("pixelization", 1).max(1) as usize;

                let mut tile = Tile::empty();

                let mut buffer = vec![0u8; size * size * 4];
                for y in (0..size).step_by(pixelization) {
                    for x in (0..size).step_by(pixelization) {
                        // Normalized coordinates
                        let p = Vec2::new(x as f32 / size as f32, y as f32 / size as f32);
                        let noise = self.noise2d(&p, Vec2::new(1.0, 1.0), 4) * noise_intensity;
                        let uniform_noise = (noise * 2.0 - 1.0) * noise_intensity;

                        let mut color = color.clone();
                        match apply_to {
                            NoiseTarget::RGB => {
                                let mut rgb = color.to_u8_array();
                                for channel in rgb.iter_mut() {
                                    *channel = ((*channel as f32 * (1.0 + uniform_noise))
                                        .clamp(0.0, 255.0))
                                        as u8;
                                }
                                color = TheColor::from_u8_array(rgb);
                                color.a = 1.0;
                            }
                            NoiseTarget::Hue => {
                                let hsl = color.as_hsl();
                                let new_h = (hsl.x + uniform_noise).fract();
                                color = TheColor::from_hsl(new_h, hsl.y, hsl.z);
                            }
                            NoiseTarget::Luminance => {
                                let hsl = color.as_hsl();
                                let new_l = (hsl.z + uniform_noise).clamp(0.0, 1.0);
                                color = TheColor::from_hsl(hsl.x, hsl.y, new_l);
                            }
                        }

                        // Write the modified color to the buffer
                        let rgba = color.to_u8_array();
                        for block_y in y..(y + pixelization).min(size) {
                            for block_x in x..(x + pixelization).min(size) {
                                let index = (block_y * size + block_x) * 4;
                                buffer[index..index + 4].copy_from_slice(&rgba);
                            }
                        }
                    }
                }
                tile.append(Texture::new(buffer, size, size));
                Some(tile)
            }
            _ => None,
        }
    }

    /// Generate a tile from the tile_list indices
    pub fn tile_from_tile_list(&self, assets: &Assets) -> Option<Tile> {
        match self {
            Noise(noise) => assets
                .tiles
                .get(&noise.tile_id())
                .cloned()
                .or_else(|| Some(noise.to_tile(64))),
            TileId(id) | MaterialId(id) => {
                if let Some(index) = assets.tile_indices.get(id) {
                    assets.tile_list.get(*index as usize).cloned()
                } else {
                    assets
                        .tiles
                        .get(id)
                        .cloned()
                        .or_else(|| assets.materials.get(id).cloned())
                }
            }
            TileGroup(_) | TileGroupMember { .. } | ProceduralTile(_) => None,
            PaletteIndex(index) => {
                let tile_id = Self::palette_tile_uuid(*index);
                if let Some(index) = assets.tile_indices.get(&tile_id) {
                    assets.tile_list.get(*index as usize).cloned()
                } else {
                    assets
                        .tiles
                        .get(&tile_id)
                        .cloned()
                        .or_else(|| Self::synthetic_palette_tile(assets, *index))
                }
            }
            _ => None,
        }
    }

    /// Generate a tile from the entities sequence
    pub fn entity_tile_id(&self, id: u32, assets: &Assets) -> Option<PixelSource> {
        match self {
            Sequence(name) => {
                if let Some(sequences) = assets.entity_tiles.get(&id) {
                    sequences
                        .get_index_of(name)
                        .map(|index| PixelSource::EntityTile(id, index as u32))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Generate a tile from the items sequence
    pub fn item_tile_id(&self, id: u32, assets: &Assets) -> Option<PixelSource> {
        match self {
            Sequence(name) => {
                if let Some(sequences) = assets.item_tiles.get(&id) {
                    sequences
                        .get_index_of(name)
                        .map(|index| PixelSource::ItemTile(id, index as u32))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn noise2d(&self, p: &Vec2<f32>, scale: Vec2<f32>, octaves: i32) -> f32 {
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
}

#[cfg(test)]
mod noise_tests {
    use super::*;
    #[test]
    fn noise_gradient_interpolates_colors_and_has_distinct_atlas_ids() {
        let noise = NoiseMaterial {
            low_color: Some([200, 20, 40, 255]),
            high_color: Some([20, 180, 220, 255]),
            ..Default::default()
        };
        let tile = noise.to_tile(8);
        for pixel in tile.textures[0].data.chunks_exact(4) {
            assert!((20..=200).contains(&pixel[0]));
            assert!((20..=180).contains(&pixel[1]));
            assert!((40..=220).contains(&pixel[2]));
            assert_eq!(pixel[3], 255);
        }
        let mut changed = noise.clone();
        changed.high_color = Some([20, 180, 221, 255]);
        assert_ne!(noise.tile_id(), changed.tile_id());
        assert_ne!(noise.tile_id(), NoiseMaterial::default().tile_id());
    }

    #[test]
    fn noise_is_repeatable_periodic_and_varies_with_seed_and_mode() {
        for voronoi in [false, true] {
            let noise = NoiseMaterial {
                voronoi,
                ..Default::default()
            };
            for (u, v) in [(0.13, 0.27), (0.52, 0.79), (0.0, 0.0)] {
                assert!((noise.sample(u, v) - noise.sample(u + 1.0, v - 1.0)).abs() < 1e-5);
                assert!((0.0..=1.0).contains(&noise.sample(u, v)));
            }
            let mut seeded = noise.clone();
            seeded.seed = 42;
            assert_ne!(noise.tile_id(), seeded.tile_id());
            assert_ne!(noise.sample(0.13, 0.27), seeded.sample(0.13, 0.27));
            assert_eq!(noise.to_tile(16).id, noise.tile_id());
        }
        assert_ne!(
            NoiseMaterial::default().sample(0.13, 0.27),
            NoiseMaterial {
                voronoi: true,
                ..Default::default()
            }
            .sample(0.13, 0.27)
        );
    }
}
