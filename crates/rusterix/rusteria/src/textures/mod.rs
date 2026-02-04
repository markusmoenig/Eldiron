pub use crate::Value;
use image::{ImageBuffer, Rgb, RgbImage};
use std::path::PathBuf;
pub mod patterns;

// Patterns taken from https://github.com/tuxalin/procedural-tileable-shaders

use rayon::prelude::*;

#[derive(Clone)]
pub struct TexStorage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Value>, // length = width * height
}

impl TexStorage {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![Value::zero(); width * height],
        }
    }

    /// Nearest-neighbor sample of a normalized UV carried in a Value (Vec3<f32>), using x/y as uv.
    #[inline]
    pub fn sample(&self, uv: Value) -> Value {
        let (x, y) = self.sample_index(uv);
        unsafe { *self.data.get_unchecked(y * self.width + x) }
    }

    /// Directly set a pixel by integer coordinates (no bounds check).
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, value: Value) {
        let idx = y * self.width + x;
        self.data[idx] = value;
    }

    /// Parallel iterate over all pixels with a per-row state initializer.
    /// `init` creates state once per processed row (Rayon work item).
    /// `f` shades one pixel using that reusable state and returns the pixel Value.
    pub fn par_iterate_with<Init, F, S>(&mut self, init: Init, f: F)
    where
        Init: Fn() -> S + Sync,
        F: Fn(&mut S, usize, usize, Value) -> Value + Sync,
        S: Send,
    {
        let inv_w = 1.0f32 / self.width as f32;
        let inv_h = 1.0f32 / self.height as f32;
        self.data
            .par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, row)| {
                let mut state = init(); // once per row
                let v = y as f32 * inv_h;
                for x in 0..self.width {
                    let u = x as f32 * inv_w;
                    let uv = Value::new(u, v, 0.0);
                    row[x] = f(&mut state, x, y, uv);
                }
            });
    }

    /// Construct a TexStorage by decoding a PNG from bytes.
    pub fn from_png_bytes(bytes: &[u8]) -> image::ImageResult<Self> {
        let img = image::load_from_memory(bytes)?.to_rgb8();
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut tex = TexStorage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let p = img.get_pixel(x as u32, y as u32);
                tex.data[y * width + x] = Value::new(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
            }
        }
        Ok(tex)
    }

    /// Save the texture to a PNG file using only 3 channels (RGB).
    pub fn save_png(&self, path: &PathBuf) -> image::ImageResult<()> {
        let mut img: RgbImage = ImageBuffer::new(self.width as u32, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let v = self.data[idx];
                // Clamp and convert Value (Vec3<f32>) to RGB u8
                let r = (v.x.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (v.y.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (v.z.clamp(0.0, 1.0) * 255.0) as u8;
                img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
            }
        }
        img.save(path)
    }

    /// Load the texture from a 3-channel (RGB) PNG file.
    pub fn load_png(&mut self, path: &PathBuf) -> image::ImageResult<()> {
        let img = image::open(path)?.to_rgb8();
        assert_eq!(img.width() as usize, self.width);
        assert_eq!(img.height() as usize, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let px = img.get_pixel(x as u32, y as u32);
                self.data[idx] = Value::new(
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                );
            }
        }
        Ok(())
    }

    #[inline]
    fn rem_i32(&self, a: i32, m: i32) -> i32 {
        let r = a % m;
        if r < 0 { r + m } else { r }
    }

    #[inline]
    fn sample_index(&self, uv: Value) -> (usize, usize) {
        let mut u = uv.x;
        let mut v = uv.y;

        u = u - u.floor();
        v = v - v.floor();

        let mut x = (u * self.width as f32).floor() as i32;
        let mut y = (v * self.height as f32).floor() as i32;

        x = self.rem_i32(x, self.width as i32);
        y = self.rem_i32(y, self.height as i32);

        (x as usize, y as usize)
    }

    /// Get pixel at integer coordinates with wrap-around (tiled addressing).
    #[inline]
    fn get_pixel_wrapped(&self, x: i32, y: i32) -> Value {
        let xx = self.rem_i32(x, self.width as i32) as usize;
        let yy = self.rem_i32(y, self.height as i32) as usize;
        // Safe because we wrap into bounds
        self.data[yy * self.width + xx]
    }

    /// Perceptual luminance from RGB Value in [0,1].
    #[inline]
    fn luminance(v: Value) -> f32 {
        // Use Rec. 709 weights
        0.2126 * v.x + 0.7152 * v.y + 0.0722 * v.z
    }

    /// Generate a tangent-space normal map from this texture treated as a height field.
    /// The result is tile-safe (wraps neighbors) and encoded as RGB in [0,1], Z-up.
    /// `strength` scales the slope sensitivity (try 1.0..5.0).
    pub fn to_normal_map(&self, strength: f32) -> Self {
        let mut out = TexStorage::new(self.width, self.height);

        // Parallelize per row
        out.data
            .par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, row)| {
                let y_i = y as i32;
                for x in 0..self.width {
                    let x_i = x as i32;

                    // Central differences on luminance with wrap-around
                    let h_l = Self::luminance(self.get_pixel_wrapped(x_i - 1, y_i));
                    let h_r = Self::luminance(self.get_pixel_wrapped(x_i + 1, y_i));
                    let h_u = Self::luminance(self.get_pixel_wrapped(x_i, y_i - 1));
                    let h_d = Self::luminance(self.get_pixel_wrapped(x_i, y_i + 1));

                    let dx = (h_r - h_l) * 0.5 * strength; // dH/du
                    let dy = (h_d - h_u) * 0.5 * strength; // dH/dv

                    // Tangent-space normal, Z-up. Negate to follow standard convention.
                    let mut n = Value::new(-dx, -dy, 1.0);
                    let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
                    if len > 0.0 {
                        n = Value::new(n.x / len, n.y / len, n.z / len);
                    }

                    // Pack to [0,1]
                    row[x] = Value::new(0.5 * (n.x + 1.0), 0.5 * (n.y + 1.0), 0.5 * (n.z + 1.0));
                }
            });

        out
    }

    /// Convenience: generate and save a normal map PNG derived from this texture.
    /// Returns the image::ImageResult from saving.
    pub fn save_normal_map_png(&self, path: &PathBuf, strength: f32) -> image::ImageResult<()> {
        let nm = self.to_normal_map(strength);
        nm.save_png(path)
    }
}
