use rayon::prelude::*;
use vek::Vec4;

#[derive(PartialEq, Debug, Clone)]
pub struct AccumBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<f32>,
    pub frame: usize,
}

impl AccumBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0.0; width * height * 4],
            frame: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: vec![],
            frame: 0,
        }
    }

    pub fn reset(&mut self) {
        self.frame = 0;
    }

    #[inline(always)]
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width * 4 + x * 4
    }

    /// RGBA → flat-index helper
    #[inline(always)]
    fn idx(&self, x: usize, y: usize) -> usize {
        (y * self.width + x) * 4
    }

    /// Fetch one pixel
    #[inline(always)]
    pub fn get_pixel(&self, x: usize, y: usize) -> Vec4<f32> {
        let i = self.idx(x, y);
        Vec4::new(
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        )
    }

    /// Store one pixel
    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, c: Vec4<f32>) {
        let i = self.idx(x, y);
        self.pixels[i] = c.x;
        self.pixels[i + 1] = c.y;
        self.pixels[i + 2] = c.z;
        self.pixels[i + 3] = c.w;
    }

    #[inline(always)]
    fn linear_to_srgb_u8(x: f32) -> u8 {
        let srgb = if x <= 0.003_130_8 {
            x * 12.92
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (srgb.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 // +0.5 for proper rounding
    }

    pub fn to_u8_vec(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height * 4];
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.index(x, y);
                let o = i;
                out[o] = Self::linear_to_srgb_u8(self.pixels[i]);
                out[o + 1] = Self::linear_to_srgb_u8(self.pixels[i + 1]);
                out[o + 2] = Self::linear_to_srgb_u8(self.pixels[i + 2]);
                out[o + 3] = (self.pixels[i + 3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
        out
    }

    pub fn convert_to_u8(&self, frame: &mut [u8]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.index(x, y);
                let o = i;
                frame[o] = Self::linear_to_srgb_u8(self.pixels[i]);
                frame[o + 1] = Self::linear_to_srgb_u8(self.pixels[i + 1]);
                frame[o + 2] = Self::linear_to_srgb_u8(self.pixels[i + 2]);
                frame[o + 3] = (self.pixels[i + 3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    }

    pub fn convert_to_u8_at(&self, frame: &mut [u8], at: (usize, usize, usize, usize)) {
        let (ox, oy, w, h) = at;

        frame
            .par_rchunks_exact_mut(w * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let x = i;
                    let y = h - j;

                    if x >= ox && x < ox + self.width && y >= oy && y < oy + self.height {
                        let sx = x - ox;
                        let sy = y - oy;
                        let si = self.index(sx, sy);

                        pixel[0] = Self::linear_to_srgb_u8(self.pixels[si]);
                        pixel[1] = Self::linear_to_srgb_u8(self.pixels[si + 1]);
                        pixel[2] = Self::linear_to_srgb_u8(self.pixels[si + 2]);
                        pixel[3] = (self.pixels[si + 3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    }
                }
            });
    }
}
