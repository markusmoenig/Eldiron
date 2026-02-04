use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// A color buffer holding an array of float pixels.
#[derive(PartialEq, Debug, Clone)]
pub struct RenderBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<f32>,
    pub accum: u32,

    pub file_path: Option<std::path::PathBuf>,
}

impl RenderBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0.0; width * height * 4],
            accum: 1,
            file_path: None,
        }
    }

    /// Get the color of a pixel
    #[inline(always)]
    pub fn at(&self, x: usize, y: usize) -> [f32; 4] {
        let i = y * self.width * 4 + x * 4;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }

    /// Set the color of a pixel
    pub fn set(&mut self, x: usize, y: usize, color: [f32; 4]) {
        let i = y * self.width * 4 + x * 4;
        self.pixels[i..i + 4].copy_from_slice(&color);
    }

    /// Copy the pixels from another buffer to this buffer
    pub fn copy_from(&mut self, x: usize, y: usize, other: &RenderBuffer) {
        for local_y in 0..other.height {
            for local_x in 0..other.width {
                let global_x = x + local_x;
                let global_y = y + local_y;

                if global_x >= self.width || global_y >= self.height {
                    continue;
                }

                let index = (global_y * self.width + global_x) * 4;
                let local_index = (local_y * other.width + local_x) * 4;
                self.pixels[index..index + 4]
                    .copy_from_slice(&other.pixels[local_index..local_index + 4]);
            }
        }
    }

    /// Copy and accumulate pixels from another buffer to this buffer
    pub fn accum_from(&mut self, x: usize, y: usize, other: &RenderBuffer) {
        for local_y in 0..other.height {
            for local_x in 0..other.width {
                let global_x = x + local_x;
                let global_y = y + local_y;

                if global_x >= self.width || global_y >= self.height {
                    continue;
                }

                let index = (global_y * self.width + global_x) * 4;
                let local_index = (local_y * other.width + local_x) * 4;

                for i in 0..4 {
                    let old = self.pixels[index + i];
                    let new = other.pixels[local_index + i];
                    let factor = 1.0 / self.accum as f32;
                    self.pixels[index + i] = old * (1.0 - factor) + new * factor;
                }
            }
        }
    }

    /// Convert the frame to an u8 vec, applying gamma correction
    pub fn to_u8_vec_gamma(&self) -> Vec<u8> {
        let source = &self.pixels[..];
        let mut out: Vec<u8> = vec![0; self.width * self.height * 4];
        let gamma_correction = 0.4545;

        for y in 0..self.height {
            for x in 0..self.width {
                let d = x * 4 + y * self.width * 4;
                let c = [
                    (source[d].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 1].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 2].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 3] * 255.0) as u8,
                ];
                out[d..d + 4].copy_from_slice(&c);
            }
        }

        out
    }

    /// Convert the frame to an u8 vec, applying gamma correction
    pub fn to_u8_vec_gamma_buffer(&self, buffer: &mut [u8]) {
        let source = &self.pixels[..];
        let gamma_correction = 0.4545;

        for y in 0..self.height {
            for x in 0..self.width {
                let d = x * 4 + y * self.width * 4;
                let c = [
                    (source[d].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 1].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 2].powf(gamma_correction) * 255.0) as u8,
                    (source[d + 3] * 255.0) as u8,
                ];
                buffer[d..d + 4].copy_from_slice(&c);
            }
        }
    }

    /// Convert the frame to an u8 vecc.
    pub fn to_u8_vec(&self) -> Vec<u8> {
        let source = &self.pixels[..];
        let mut out: Vec<u8> = vec![0; self.width * self.height * 4];

        for y in 0..self.height {
            for x in 0..self.width {
                let d = x * 4 + y * self.width * 4;
                let c = [
                    (source[d] * 255.0) as u8,
                    (source[d + 1] * 255.0) as u8,
                    (source[d + 2] * 255.0) as u8,
                    (source[d + 3] * 255.0) as u8,
                ];
                out[d..d + 4].copy_from_slice(&c);
            }
        }

        out
    }

    /// Return raw **RGBA8** bytes suitable for JS `ImageData` (sRGB gamma corrected).
    /// Length is `width * height * 4`.
    #[inline]
    pub fn as_rgba_bytes(&self) -> Vec<u8> {
        // This is exactly what the canvas expects; reuse our gamma path.
        self.to_u8_vec_gamma()
    }

    /// Fill the provided buffer with raw **RGBA8** bytes (sRGB gamma corrected).
    /// `out` must be length `width * height * 4`.
    #[inline]
    pub fn write_rgba_to_buffer(&self, out: &mut [u8]) {
        debug_assert_eq!(out.len(), self.width * self.height * 4);
        self.to_u8_vec_gamma_buffer(out);
    }

    /// Save the buffer to a file as PNG
    pub fn save(&self, path: std::path::PathBuf) {
        let mut image = image::ImageBuffer::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width * 4 + x * 4;
                let c = image::Rgb([
                    (self.pixels[i] * 255.0) as u8,
                    (self.pixels[i + 1] * 255.0) as u8,
                    (self.pixels[i + 2] * 255.0) as u8,
                ]);
                image.put_pixel(x as u32, y as u32, c);
            }
        }

        image.save(path).unwrap();
    }

    pub fn save_srgb(&self, path: std::path::PathBuf) {
        let gamma = 1.0 / 2.2;
        let mut image = image::ImageBuffer::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width * 4 + x * 4;

                let r = self.pixels[i].max(0.0).powf(gamma);
                let g = self.pixels[i + 1].max(0.0).powf(gamma);
                let b = self.pixels[i + 2].max(0.0).powf(gamma);

                let rgb = image::Rgb([
                    (r * 255.0).min(255.0) as u8,
                    (g * 255.0).min(255.0) as u8,
                    (b * 255.0).min(255.0) as u8,
                ]);
                image.put_pixel(x as u32, y as u32, rgb);
            }
        }

        image.save(path).unwrap();
    }

    pub fn save_film(&self, path: std::path::PathBuf) {
        fn tonemap_film(x: f32) -> f32 {
            let a = 2.51;
            let b = 0.03;
            let c = 2.43;
            let d = 0.59;
            let e = 0.14;
            ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
        }

        let mut image = image::ImageBuffer::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width * 4 + x * 4;

                let r: f32 = tonemap_film(self.pixels[i]);
                let g: f32 = tonemap_film(self.pixels[i + 1]);
                let b: f32 = tonemap_film(self.pixels[i + 2]);

                let rgb = image::Rgb([
                    (r * 255.0).min(255.0) as u8,
                    (g * 255.0).min(255.0) as u8,
                    (b * 255.0).min(255.0) as u8,
                ]);
                image.put_pixel(x as u32, y as u32, rgb);
            }
        }

        image.save(path).unwrap();
    }

    // Save to an png in memory
    pub fn as_png_bytes(&self) -> Vec<u8> {
        let gamma = 1.0 / 2.2;
        let mut img = image::ImageBuffer::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width * 4 + x * 4;

                let r = self.pixels[i].max(0.0).powf(gamma);
                let g = self.pixels[i + 1].max(0.0).powf(gamma);
                let b = self.pixels[i + 2].max(0.0).powf(gamma);

                img.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgb([
                        (r * 255.0).min(255.0) as u8,
                        (g * 255.0).min(255.0) as u8,
                        (b * 255.0).min(255.0) as u8,
                    ]),
                );
            }
        }

        let dyn_img = DynamicImage::ImageRgb8(img);
        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);

        dyn_img
            .write_to(&mut cursor, ImageFormat::Png)
            .expect("failed to write PNG");

        buf
    }
}
