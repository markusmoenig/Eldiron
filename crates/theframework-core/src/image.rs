use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// A backend-neutral resource identity. Registries validate owner and generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageHandle {
    pub owner: u64,
    pub slot: u32,
    pub generation: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    Linear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlphaMode {
    Straight,
    Premultiplied,
    Opaque,
}

/// Authored/decoded RGBA data, independent of widget position or display scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageAsset {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pub color_space: ColorSpace,
    pub alpha_mode: AlphaMode,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageError {
    DimensionsOverflow,
    ByteCount { expected: usize, actual: usize },
}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid RGBA image: {self:?}")
    }
}
impl Error for ImageError {}
impl ImageAsset {
    pub fn rgba(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, ImageError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(ImageError::DimensionsOverflow)?;
        if expected != pixels.len() {
            return Err(ImageError::ByteCount {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }
}

/// Generic image color adjustment in straight encoded sRGB. Coverage remains
/// independent; GPU sources are sampled in linear premultiplied color first.
#[derive(Clone, Copy, Debug)]
pub struct ImageColorFilter {
    pub scale: [f32; 3],
    pub bias: [f32; 3],
    pub saturation: f32,
    pub minimum: f32,
    /// Opacity attenuation in the inclusive range 0..=1.
    pub alpha_scale: f32,
}
impl Default for ImageColorFilter {
    fn default() -> Self {
        Self {
            scale: [1.; 3],
            bias: [0.; 3],
            saturation: 1.,
            minimum: 0.,
            alpha_scale: 1.,
        }
    }
}
impl ImageColorFilter {
    /// Reference/legacy source-asset adjustment; command UI executes it on the GPU.
    pub fn apply_encoded_srgb(self, pixel: [u8; 4]) -> [u8; 4] {
        let source = [
            pixel[0] as f32 / 255.,
            pixel[1] as f32 / 255.,
            pixel[2] as f32 / 255.,
        ];
        let gray = source[0] * 0.299 + source[1] * 0.587 + source[2] * 0.114;
        let mut result = [0; 4];
        for i in 0..3 {
            let value =
                (gray + (source[i] - gray) * self.saturation) * self.scale[i] + self.bias[i];
            result[i] = (value.max(self.minimum).clamp(0., 1.) * 255.).round() as u8;
        }
        result[3] = (pixel[3] as f32 * self.alpha_scale).round().clamp(0., 255.) as u8;
        result
    }
}
