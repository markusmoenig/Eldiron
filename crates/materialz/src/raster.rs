//! Bounded embedded raster samples used by host-imported image materials.
//! Pixel data lives in the immutable GPU parameter buffer, never shader code.
use crate::{Error, Result};
#[derive(Clone, Debug)]
pub(crate) struct Raster {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 4]>,
}
impl Raster {
    pub fn parse(source: &str) -> Result<Option<Self>> {
        let mut rows = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("# raster-rgba8 "));
        let Some(line) = rows.next() else {
            return Ok(None);
        };
        if rows.next().is_some() {
            return Err(Error("Only one embedded raster per tile is allowed".into()));
        }
        let mut parts = line.split_whitespace();
        let width = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let height = parts
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let space = parts.next().unwrap_or("");
        let hex = parts.next().unwrap_or("");
        let count = width
            .checked_mul(height)
            .filter(|n| *n > 0 && *n <= 1_048_576)
            .ok_or_else(|| {
                Error("Raster dimensions must contain between 1 and 1048576 pixels".into())
            })?;
        if !matches!(space, "srgb" | "linear")
            || hex.len() != count * 8
            || parts.next().is_some()
            || !hex.is_ascii()
        {
            return Err(Error(
                "Invalid raster dimensions, color space or RGBA8 payload".into(),
            ));
        }
        let mut pixels = Vec::with_capacity(count);
        for pixel in hex.as_bytes().chunks_exact(8) {
            let mut value = [0.; 4];
            for c in 0..4 {
                let byte = std::str::from_utf8(&pixel[c * 2..c * 2 + 2])
                    .ok()
                    .and_then(|v| u8::from_str_radix(v, 16).ok())
                    .ok_or_else(|| Error("Invalid raster hex data".into()))?;
                let x = byte as f32 / 255.;
                value[c] = if space == "srgb" && c < 3 {
                    if x <= 0.04045 {
                        x / 12.92
                    } else {
                        ((x + 0.055) / 1.055).powf(2.4)
                    }
                } else {
                    x
                };
            }
            pixels.push(value);
        }
        Ok(Some(Self {
            width,
            height,
            pixels,
        }))
    }
    pub fn sample(&self, uv: [f32; 2]) -> [f32; 4] {
        let x = ((uv[0] - uv[0].floor()) * self.width as f32) as usize;
        let y = ((uv[1] - uv[1].floor()) * self.height as f32) as usize;
        self.pixels[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}
