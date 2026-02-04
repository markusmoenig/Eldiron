use crate::{Assets, BLACK, Map, Pixel, Texture, Tile, ValueContainer};
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

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub enum PixelSource {
    #[default]
    Off,
    TileId(Uuid),
    MaterialId(Uuid),
    Sequence(String),
    EntityTile(u32, u32),
    ItemTile(u32, u32),
    Color(TheColor),
    ShapeFXGraphId(Uuid),
    StaticTileIndex(u16),
    DynamicTileIndex(u16),
    Pixel(Pixel),
    Terrain,
}

use PixelSource::*;

impl PixelSource {
    /// Generate a tile from the given PixelValue
    pub fn to_tile(
        &self,
        assets: &Assets,
        size: usize,
        values: &ValueContainer,
        map: &Map,
    ) -> Option<Tile> {
        match self {
            TileId(id) => assets.tiles.get(id).cloned(),
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
            ShapeFXGraphId(id) => {
                let mut tile = Tile::empty();
                // let mut texture = Texture::alloc(size, size);

                let texture = if let Some(graph) = map.shapefx_graphs.get(id) {
                    //graph.preview(&mut texture, &assets.palette);
                    Texture::from_color(graph.get_dominant_color(&assets.palette))
                } else {
                    Texture::from_color(BLACK)
                };
                tile.append(texture);
                Some(tile)
            }
            _ => None,
        }
    }

    /// Generate a tile from the tile_list indices
    pub fn tile_from_tile_list(&self, assets: &Assets) -> Option<Tile> {
        match self {
            TileId(id) | MaterialId(id) => {
                if let Some(index) = assets.tile_indices.get(id) {
                    assets.tile_list.get(*index as usize).cloned()
                } else {
                    None
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
