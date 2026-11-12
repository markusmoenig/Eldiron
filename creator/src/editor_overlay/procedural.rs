//! Small immutable procedural icon descriptions. CPU materialization is confined
//! to the explicit reference adapter; native consumers receive shader parameters.
use crate::prelude::*;
use rusterix::{Assets, PixelSource, ValueContainer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColorNoise {
    target: i32,
    intensity: f32,
    pixelization: i32,
}
impl Default for ColorNoise {
    fn default() -> Self {
        Self {
            target: 0,
            intensity: 0.,
            pixelization: 1,
        }
    }
}
impl ColorNoise {
    pub(crate) fn from_values(values: &ValueContainer) -> Self {
        Self {
            target: values.get_int_default("noise_target", 0),
            intensity: values.get_float_default("noise_intensity", 0.),
            pixelization: values.get_int_default("pixelization", 1),
        }
    }
    pub(crate) fn values(self) -> ValueContainer {
        let mut values = ValueContainer::default();
        values.set("noise_target", Value::Int(self.target));
        values.set("noise_intensity", Value::Float(self.intensity));
        values.set("pixelization", Value::Int(self.pixelization));
        values
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProceduralIcon {
    pub(super) source: PixelSource,
    pub(super) noise: ColorNoise,
    pub(super) size: u32,
}
impl ProceduralIcon {
    pub(crate) fn new(source: &PixelSource, noise: ColorNoise, size: u32) -> Option<Self> {
        matches!(source, PixelSource::Color(_) | PixelSource::Noise(_)).then(|| Self {
            source: source.clone(),
            noise,
            size,
        })
    }
    pub(crate) fn size(&self) -> u32 {
        if matches!(self.source, PixelSource::Noise(_)) {
            self.size.clamp(2, 256)
        } else {
            self.size
        }
    }
    /// Six vec4 uniform words. Float bits retain source identity, independent of
    /// a color's display name; no per-destination samples or bitmap are prepared.
    pub(crate) fn words(&self) -> [u32; 24] {
        let mut words = [0; 24];
        words[1] = self.size();
        match &self.source {
            PixelSource::Color(color) => {
                words[2] = match self.noise.target {
                    1 => 1,
                    2 => 2,
                    _ => 0,
                };
                words[3] = self.noise.pixelization.max(1) as u32;
                for (out, value) in words[4..8].iter_mut().zip(color.to_u8_array()) {
                    *out = value as u32;
                }
                let hsl = color.as_hsl();
                words[8..12].copy_from_slice(&[
                    hsl.x.to_bits(),
                    hsl.y.to_bits(),
                    hsl.z.to_bits(),
                    self.noise.intensity.to_bits(),
                ]);
            }
            PixelSource::Noise(noise) => {
                words[0] = 1;
                let (low, high) = noise.colors();
                for (out, value) in words[12..16].iter_mut().zip(low) {
                    *out = value as u32;
                }
                for (out, value) in words[16..20].iter_mut().zip(high) {
                    *out = value as u32;
                }
                words[20..24].copy_from_slice(&[
                    noise.scale.clamp(1, 32),
                    noise.seed,
                    noise.voronoi as u32,
                    0,
                ]);
            }
            _ => unreachable!("validated procedural source"),
        }
        words
    }
    /// Pixel generation is an explicit CPU reference/export operation. Never
    /// called by descriptor preparation or the native source producer.
    pub(crate) fn reference_pixels(&self) -> rusterix::Texture {
        self.source
            .to_tile(
                &Assets::default(),
                self.size() as usize,
                &self.noise.values(),
                &Map::default(),
            )
            .expect("procedural source")
            .textures
            .remove(0)
    }
}

impl super::OverlaySources {
    /// Existing authored source pixels, including a single palette source texel.
    /// Unsupported PixelSource variants keep the reference to_tile behavior.
    pub(crate) fn pixel_source(
        &mut self,
        role: &str,
        source: &PixelSource,
        assets: &Assets,
    ) -> Option<super::OverlayImage> {
        let tile = match source {
            PixelSource::TileId(id) => assets.tiles.get(id),
            PixelSource::MaterialId(id) => assets.materials.get(id),
            PixelSource::PaletteIndex(index) => {
                if let Some(tile) = assets.tiles.get(&PixelSource::palette_tile_uuid(*index)) {
                    Some(tile)
                } else {
                    let color = assets.palette.colors.get(*index as usize)?.as_ref()?;
                    return self.image(role, 1, 1, &color.to_u8_array());
                }
            }
            _ => None,
        }?;
        let texture = tile.textures.first()?;
        self.image(role, texture.width, texture.height, &texture.data)
    }
}
