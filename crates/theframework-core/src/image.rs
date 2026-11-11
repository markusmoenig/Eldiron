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
