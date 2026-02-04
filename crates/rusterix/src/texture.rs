use crate::IntoDataInput;
use std::io::Cursor;
use theframework::prelude::*;

/// Sample mode for texture sampling.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum SampleMode {
    /// Nearest-neighbor sampling
    Nearest,
    /// Linear interpolation sampling
    Linear,
}

/// The repeat mode for texture sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    /// Clamps UVs to [0, 1] (the default)
    ClampXY,
    /// Repeats texture in both X and Y
    RepeatXY,
    /// Repeats texture only in X
    RepeatX,
    /// Repeats texture only in Y
    RepeatY,
}

/// Textures contain RGBA [u8;4] pixels for color data, plus optional unified material/normal data.
///
/// ## Unified Material/Normal Format
///
/// When `data_ext` is present, each pixel has an additional u32 (4 bytes) packed as follows:
///
/// **Bytes 0-1 (u16): Material Properties**
/// - Bits 0-3:   Roughness (0-15, map to 0.0-1.0)
/// - Bits 4-7:   Metallic  (0-15, map to 0.0-1.0)
/// - Bits 8-11:  Opacity   (0-15, map to 0.0-1.0)
/// - Bits 12-15: Emissive  (0-15, map to 0.0-1.0)
///
/// **Bytes 2-3: Normal Map (2-component, reconstruct Z in shader)**
/// - Byte 2: Normal X component (0-255, map to -1.0 to +1.0)
/// - Byte 3: Normal Y component (0-255, map to -1.0 to +1.0)
/// - Normal Z: Reconstructed in shader as `sqrt(max(0.0, 1.0 - X*X - Y*Y))`
///
/// This compact format saves memory while supporting retro-style PBR rendering.
#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Clone, Debug)]
pub struct Texture {
    /// RGBA8 color data (4 bytes per pixel)
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// Optional unified material+normal data (4 u8 bytes per pixel)
    /// See struct documentation for packed format details
    pub data_ext: Option<Vec<u8>>,
}

impl Default for Texture {
    fn default() -> Self {
        Self::white()
    }
}

impl Texture {
    /// Creates a new texture with the given width, height, and data
    pub fn new(data: Vec<u8>, width: usize, height: usize) -> Self {
        assert_eq!(data.len(), width * height * 4, "Invalid texture data size.");
        Texture {
            data,
            width,
            height,
            data_ext: None,
        }
    }

    /// Creates a new texture with the given width, height, and allocates the data.
    pub fn alloc(width: usize, height: usize) -> Self {
        Texture {
            data: vec![0; width * height * 4],
            width,
            height,
            data_ext: None,
        }
    }

    /// Creates a default 100x100 checkerboard texture
    pub fn checkerboard(size: usize, square_size: usize) -> Self {
        let width = size;
        let height = size;
        let mut data = vec![0; width * height * 4]; // Initialize texture data

        for y in 0..height {
            for x in 0..width {
                let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;
                let color = if is_white {
                    [128, 128, 128, 255]
                } else {
                    [0, 0, 0, 255]
                };

                let idx = (y * width + x) * 4;
                data[idx..idx + 4].copy_from_slice(&color);
            }
        }

        Texture {
            data,
            width,
            height,
            data_ext: None,
        }
    }

    /// Creates a texture filled with a single color (1x1 texture)
    pub fn from_color(color: [u8; 4]) -> Self {
        Texture {
            data: color.to_vec(),
            width: 1,
            height: 1,
            data_ext: None,
        }
    }

    /// Creates a texture filled with a white color (1x1 texture)
    pub fn white() -> Self {
        Texture {
            data: vec![255, 255, 255, 255],
            width: 1,
            height: 1,
            data_ext: None,
        }
    }

    /// Creates a texture filled with a black color (1x1 texture)
    pub fn black() -> Self {
        Texture {
            data: vec![0, 0, 0, 255],
            width: 1,
            height: 1,
            data_ext: None,
        }
    }

    pub fn from_rgbabuffer(buffer: &TheRGBABuffer) -> Self {
        Texture {
            data: buffer.pixels().to_vec(),
            width: buffer.dim().width as usize,
            height: buffer.dim().height as usize,
            data_ext: None,
        }
    }

    /// Loads a texture from an image file at the given path.
    pub fn from_image(input: impl IntoDataInput) -> Self {
        // Load the image from the input source
        let data = input.load_data().expect("Failed to load data");
        let img = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .expect("Failed to read image format")
            .decode()
            .expect("Failed to decode the image");

        // Convert to RGBA8 format
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        // Flatten the image data into a Vec<u8>
        let data = rgba_img.into_raw();

        Texture {
            data,
            width: width as usize,
            height: height as usize,
            data_ext: None,
        }
    }

    /// Loads a texture from an image file at the given path (if available).
    pub fn from_image_safe(input: impl IntoDataInput) -> Option<Self> {
        // Try to load the image from the input source
        let data = input.load_data().ok()?;
        let img = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .ok()? // Early return on format guessing failure
            .decode()
            .ok()?; // Early return on decoding failure

        // Convert to RGBA8 format
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        // Flatten the image data into a Vec<u8>
        let data = rgba_img.into_raw();

        Some(Texture {
            data,
            width: width as usize,
            height: height as usize,
            data_ext: None,
        })
    }

    /// Samples the texture using the specified sampling and repeat mode
    #[inline(always)]
    pub fn sample(
        &self,
        mut u: f32,
        mut v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
    ) -> [u8; 4] {
        match repeat_mode {
            RepeatMode::ClampXY => {
                u = u.clamp(0.0, 1.0);
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatXY => {
                u = u - u.floor(); // Wraps in both X and Y
                v = v - v.floor();
            }
            RepeatMode::RepeatX => {
                u = u - u.floor(); // Wraps only in X
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatY => {
                u = u.clamp(0.0, 1.0);
                v = v - v.floor(); // Wraps only in Y
            }
        }
        match sample_mode {
            SampleMode::Nearest => self.sample_nearest(u, v),
            SampleMode::Linear => self.sample_linear(u, v),
        }
    }

    /// Samples the texture using the specified sampling and repeat mode
    pub fn sample_blur(
        &self,
        mut u: f32,
        mut v: f32,
        sample_mode: SampleMode,
        repeat_mode: RepeatMode,
        blur_strength: f32,
    ) -> [u8; 4] {
        match repeat_mode {
            RepeatMode::ClampXY => {
                u = u.clamp(0.0, 1.0);
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatXY => {
                u = u - u.floor(); // Wraps in both X and Y
                v = v - v.floor();
            }
            RepeatMode::RepeatX => {
                u = u - u.floor(); // Wraps only in X
                v = v.clamp(0.0, 1.0);
            }
            RepeatMode::RepeatY => {
                u = u.clamp(0.0, 1.0);
                v = v - v.floor(); // Wraps only in Y
            }
        }
        match sample_mode {
            SampleMode::Nearest => {
                if blur_strength == 0.0 {
                    self.sample_nearest(u, v)
                } else {
                    self.sample_nearest_blur(u, v, blur_strength)
                }
            }

            SampleMode::Linear => self.sample_linear(u, v),
        }
    }

    // Samples the texture at given UV coordinates.
    // pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
    //     // Map UV coordinates to pixel indices
    //     let tex_x = (u * (self.width as f32 - 1.0)).round() as usize;
    //     let tex_y = (v * (self.height as f32 - 1.0)).round() as usize;

    //     // Retrieve the color from the texture
    //     let idx = (tex_y * self.width + tex_x) * 4;
    //     [
    //         self.data[idx],
    //         self.data[idx + 1],
    //         self.data[idx + 2],
    //         self.data[idx + 3],
    //     ]
    // }
    // #[inline(always)]
    // pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
    //     let mut tx = (u * self.width as f32 + 0.5).floor() as usize;
    //     let mut ty = (v * self.height as f32 + 0.5).floor() as usize;

    //     tx = tx.clamp(0, self.width - 1);
    //     ty = ty.clamp(0, self.height - 1);

    //     let idx = (ty * self.width + tx) * 4;
    //     [
    //         self.data[idx],
    //         self.data[idx + 1],
    //         self.data[idx + 2],
    //         self.data[idx + 3],
    //     ]
    // }
    //
    #[inline(always)]
    pub fn sample_nearest(&self, u: f32, v: f32) -> [u8; 4] {
        // Properly map [0.0, 1.0] to texel centers
        let mut tx = (u * (self.width as f32 - 1.0)).round() as usize;
        let mut ty = (v * (self.height as f32 - 1.0)).round() as usize;

        // Clamp to prevent out-of-bounds
        tx = tx.clamp(0, self.width - 1);
        ty = ty.clamp(0, self.height - 1);

        let idx = (ty * self.width + tx) * 4;
        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Samples the texture at given UV coordinates.
    #[inline(always)]
    pub fn sample_nearest_blur(&self, u: f32, v: f32, blur_strength: f32) -> [u8; 4] {
        // Clamp blur_strength to [0, 1]
        let blur_strength = blur_strength.clamp(0.0, 1.0);

        // Map UV coordinates to pixel indices
        let mut tx = (u * self.width as f32 + 0.5).floor() as i32;
        let mut ty = (v * self.height as f32 + 0.5).floor() as i32;

        // Clamp texel coordinates to texture bounds
        if tx < 0 {
            tx = 0;
        } else if tx >= self.width as i32 {
            tx = self.width as i32 - 1;
        }
        if ty < 0 {
            ty = 0;
        } else if ty >= self.height as i32 {
            ty = self.height as i32 - 1;
        }

        // If blur_strength is 0, fall back to pure nearest sampling
        if blur_strength == 0.0 {
            let idx = (ty as usize * self.width + tx as usize) * 4;
            return [
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
                self.data[idx + 3],
            ];
        }

        // Define a 3x3 kernel for blurring
        let offsets = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        // Accumulate color values from neighboring texels
        let mut result = [0.0f32; 4];
        let mut total_weight = 0.0f32;

        for &(dx, dy) in &offsets {
            let nx = (tx + dx).clamp(0, self.width as i32 - 1) as usize;
            let ny = (ty + dy).clamp(0, self.height as i32 - 1) as usize;

            // Calculate weight based on distance from center
            let distance = ((dx.abs() + dy.abs()) as f32).max(1.0); // Avoid division by zero
            let weight = (1.0 / distance) * blur_strength;

            // Retrieve the color from the texture
            let idx = (ny * self.width + nx) * 4;
            let color = [
                self.data[idx] as f32,
                self.data[idx + 1] as f32,
                self.data[idx + 2] as f32,
                self.data[idx + 3] as f32,
            ];

            // Accumulate weighted color
            for i in 0..4 {
                result[i] += color[i] * weight;
            }
            total_weight += weight;
        }

        // Normalize the result by total weight
        for item in &mut result {
            *item /= total_weight;
        }

        // Convert back to u8
        [
            result[0].round() as u8,
            result[1].round() as u8,
            result[2].round() as u8,
            result[3].round() as u8,
        ]
    }

    /// Samples the texture using linear interpolation at the given UV coordinates
    pub fn sample_linear(&self, u: f32, v: f32) -> [u8; 4] {
        // Clamp UV coordinates to [0, 1]
        // let u = u.clamp(0.0, 1.0);
        // let v = v.clamp(0.0, 1.0);

        // Map UV coordinates to floating-point pixel coordinates
        let x = u * (self.width as f32 - 1.0);
        let y = v * (self.height as f32 - 1.0);

        // Calculate integer pixel indices and fractional offsets
        let x0 = x.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1); // Clamp to texture bounds
        let y0 = y.floor() as usize;
        let y1 = (y0 + 1).min(self.height - 1); // Clamp to texture bounds

        let dx = x - x.floor(); // Fractional part of x
        let dy = y - y.floor(); // Fractional part of y

        // Sample the four texels
        let idx00 = (y0 * self.width + x0) * 4;
        let idx10 = (y0 * self.width + x1) * 4;
        let idx01 = (y1 * self.width + x0) * 4;
        let idx11 = (y1 * self.width + x1) * 4;

        let c00 = &self.data[idx00..idx00 + 4];
        let c10 = &self.data[idx10..idx10 + 4];
        let c01 = &self.data[idx01..idx01 + 4];
        let c11 = &self.data[idx11..idx11 + 4];

        // Interpolate the colors
        let mut result = [0u8; 4];
        for i in 0..4 {
            let v00 = c00[i] as f32;
            let v10 = c10[i] as f32;
            let v01 = c01[i] as f32;
            let v11 = c11[i] as f32;

            // Bilinear interpolation formula
            let v0 = v00 + dx * (v10 - v00); // Interpolate along x at y0
            let v1 = v01 + dx * (v11 - v01); // Interpolate along x at y1
            let v = v0 + dy * (v1 - v0); // Interpolate along y

            result[i] = v.round() as u8;
        }

        result
    }

    /// Returns a new Texture resized to the specified width and height using nearest-neighbor sampling.
    pub fn resized(&self, new_width: usize, new_height: usize) -> Self {
        let mut new_data = vec![0; new_width * new_height * 4];
        let scale_x = self.width as f32 / new_width as f32;
        let scale_y = self.height as f32 / new_height as f32;

        for y in 0..new_height {
            for x in 0..new_width {
                let mut src_x = (x as f32 * scale_x) as usize;
                if src_x >= self.width {
                    src_x = self.width - 1;
                }

                let mut src_y = (y as f32 * scale_y) as usize;
                if src_y >= self.height {
                    src_y = self.height - 1;
                }

                let src_idx = (src_y * self.width + src_x) * 4;
                let dst_idx = (y * new_width + x) * 4;

                new_data[dst_idx..dst_idx + 4].copy_from_slice(&self.data[src_idx..src_idx + 4]);
            }
        }

        // Resize data_ext if present (nearest-neighbor for material/normal data)
        let resized_data_ext = self.data_ext.as_ref().map(|ext| {
            let mut new_ext = vec![0u8; new_width * new_height * 4];
            for y in 0..new_height {
                for x in 0..new_width {
                    let mut src_x = (x as f32 * scale_x) as usize;
                    if src_x >= self.width {
                        src_x = self.width - 1;
                    }
                    let mut src_y = (y as f32 * scale_y) as usize;
                    if src_y >= self.height {
                        src_y = self.height - 1;
                    }
                    let src_idx = (src_y * self.width + src_x) * 4;
                    let dst_idx = (y * new_width + x) * 4;
                    new_ext[dst_idx..dst_idx + 4].copy_from_slice(&ext[src_idx..src_idx + 4]);
                }
            }
            new_ext
        });

        Texture {
            data: new_data,
            width: new_width,
            height: new_height,
            data_ext: resized_data_ext,
        }
    }

    /// Fills the entire texture with the specified color
    pub fn fill(&mut self, color: [u8; 4]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) * 4;
                self.data[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }

    /// Gets the pixel at the specified (x, y) position. Clamps to bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Sets the pixel at the specified (x, y) position. Clamps to bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        self.data[idx..idx + 4].copy_from_slice(&color);
    }

    /// Convert to an TheRGBABuffer
    pub fn to_rgba(&self) -> TheRGBABuffer {
        TheRGBABuffer::from(self.data.clone(), self.width as u32, self.height as u32)
    }

    /// Generates normals from this texture's color data using Sobel filter on luma,
    /// and stores them in the unified data_ext format (preserves any existing material data).
    ///
    /// `wrap`: if true, samples wrap at edges (tiles nicely); if false, clamps at borders.
    pub fn generate_normals(&mut self, wrap: bool) {
        self.ensure_data_ext();
        let w = self.width as i32;
        let h = self.height as i32;
        let width = self.width;

        // Precompute luma (height) as f32 in [0,1]
        let mut height = vec![0.0f32; (w as usize) * (h as usize)];
        for y in 0..h {
            for x in 0..w {
                let idx = ((y as usize) * width + (x as usize)) * 4;
                let r = self.data[idx] as f32 / 255.0;
                let g = self.data[idx + 1] as f32 / 255.0;
                let b = self.data[idx + 2] as f32 / 255.0;
                // Perceptual luma
                let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                height[(y as usize) * width + (x as usize)] = l;
            }
        }

        // Helper to read height with wrap/clamp
        let sample_h = |height: &[f32], xx: i32, yy: i32| -> f32 {
            let (mut sx, mut sy) = (xx, yy);
            if wrap {
                sx = ((sx % w) + w) % w;
                sy = ((sy % h) + h) % h;
            } else {
                sx = sx.clamp(0, w - 1);
                sy = sy.clamp(0, h - 1);
            }
            height[(sy as usize) * width + (sx as usize)]
        };

        // Compute normals and store them
        for y in 0..h {
            for x in 0..w {
                let tl = sample_h(&height, x - 1, y - 1);
                let tc = sample_h(&height, x + 0, y - 1);
                let tr = sample_h(&height, x + 1, y - 1);
                let cl = sample_h(&height, x - 1, y + 0);
                let cr = sample_h(&height, x + 1, y + 0);
                let bl = sample_h(&height, x - 1, y + 1);
                let bc = sample_h(&height, x + 0, y + 1);
                let br = sample_h(&height, x + 1, y + 1);

                let gx = (-1.0 * tl)
                    + (0.0 * tc)
                    + (1.0 * tr)
                    + (-2.0 * cl)
                    + (0.0 * 0.0)
                    + (2.0 * cr)
                    + (-1.0 * bl)
                    + (0.0 * bc)
                    + (1.0 * br);

                let gy = (-1.0 * tl)
                    + (-2.0 * tc)
                    + (-1.0 * tr)
                    + (0.0 * cl)
                    + (0.0 * 0.0)
                    + (0.0 * cr)
                    + (1.0 * bl)
                    + (2.0 * bc)
                    + (1.0 * br);

                // Build normal; Z up
                let nx = -gx;
                let ny = -gy;
                let nz = 1.0;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();
                let (nx, ny) = if len > 0.0 {
                    (nx / len, ny / len)
                } else {
                    (0.0, 0.0)
                };

                // Store normal in data_ext, preserving material data
                self.set_normal(x as u32, y as u32, nx, ny);
            }
        }
    }

    // ===== Unified Material/Normal Format Functions =====

    /// Ensures data_ext is allocated for this texture
    fn ensure_data_ext(&mut self) {
        if self.data_ext.is_none() {
            self.data_ext = Some(vec![0u8; self.width * self.height * 4]);
        }
    }

    /// Pack material properties into u16 (lower 2 bytes of u32)
    /// Each property: 4 bits (0-15)
    #[inline(always)]
    fn pack_materials(roughness: f32, metallic: f32, opacity: f32, emissive: f32) -> u16 {
        let r = (roughness.clamp(0.0, 1.0) * 15.0).round() as u16;
        let m = (metallic.clamp(0.0, 1.0) * 15.0).round() as u16;
        let o = (opacity.clamp(0.0, 1.0) * 15.0).round() as u16;
        let e = (emissive.clamp(0.0, 1.0) * 15.0).round() as u16;

        r | (m << 4) | (o << 8) | (e << 12)
    }

    /// Unpack material properties from u16
    /// Returns (roughness, metallic, opacity, emissive) in 0.0-1.0
    #[inline(always)]
    fn unpack_materials(packed: u16) -> (f32, f32, f32, f32) {
        let r = (packed & 0xF) as f32 / 15.0;
        let m = ((packed >> 4) & 0xF) as f32 / 15.0;
        let o = ((packed >> 8) & 0xF) as f32 / 15.0;
        let e = ((packed >> 12) & 0xF) as f32 / 15.0;
        (r, m, o, e)
    }

    /// Pack normal X,Y into upper 2 bytes of u32
    #[inline(always)]
    fn pack_normal(nx: f32, ny: f32) -> u16 {
        let x = ((nx.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u16;
        let y = ((ny.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u16;
        x | (y << 8)
    }

    /// Unpack normal X,Y from upper 2 bytes
    /// Returns (nx, ny) in -1.0 to 1.0 range (reconstruct Z in shader)
    #[inline(always)]
    fn unpack_normal(packed: u16) -> (f32, f32) {
        let x = (packed & 0xFF) as f32 / 255.0 * 2.0 - 1.0;
        let y = ((packed >> 8) & 0xFF) as f32 / 255.0 * 2.0 - 1.0;
        (x, y)
    }

    /// Set all material properties for a pixel
    pub fn set_materials(
        &mut self,
        x: u32,
        y: u32,
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    ) {
        self.ensure_data_ext();
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        if let Some(ext) = self.data_ext.as_mut() {
            let mat_packed = Self::pack_materials(roughness, metallic, opacity, emissive);
            let mat_bytes = mat_packed.to_le_bytes();
            ext[idx] = mat_bytes[0];
            ext[idx + 1] = mat_bytes[1];
            // Preserve bytes 2-3 (normal data)
        }
    }

    /// Set material properties for all pixels in the texture
    pub fn set_materials_all(
        &mut self,
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    ) {
        self.ensure_data_ext();

        if let Some(ext) = self.data_ext.as_mut() {
            let mat_packed = Self::pack_materials(roughness, metallic, opacity, emissive);
            let mat_bytes = mat_packed.to_le_bytes();

            // Set material bytes for all pixels
            for pixel_idx in 0..(self.width * self.height) {
                let idx = pixel_idx * 4;
                ext[idx] = mat_bytes[0];
                ext[idx + 1] = mat_bytes[1];
                // Preserve bytes 2-3 (normal data)
            }
        }
    }

    /// Get all material properties for a pixel
    /// Returns (roughness, metallic, opacity, emissive) or defaults if data_ext not present
    pub fn get_materials(&self, x: u32, y: u32) -> (f32, f32, f32, f32) {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        if let Some(ext) = self.data_ext.as_ref() {
            let mat_packed = u16::from_le_bytes([ext[idx], ext[idx + 1]]);
            Self::unpack_materials(mat_packed)
        } else {
            (0.5, 0.0, 1.0, 0.0) // Defaults: half rough, no metal, opaque, no emissive
        }
    }

    /// Set individual roughness value (0.0-1.0), preserving other materials
    pub fn set_roughness(&mut self, x: u32, y: u32, roughness: f32) {
        let (_, m, o, e) = self.get_materials(x, y);
        self.set_materials(x, y, roughness, m, o, e);
    }

    /// Set individual metallic value (0.0-1.0), preserving other materials
    pub fn set_metallic(&mut self, x: u32, y: u32, metallic: f32) {
        let (r, _, o, e) = self.get_materials(x, y);
        self.set_materials(x, y, r, metallic, o, e);
    }

    /// Set individual opacity value (0.0-1.0), preserving other materials
    pub fn set_opacity(&mut self, x: u32, y: u32, opacity: f32) {
        let (r, m, _, e) = self.get_materials(x, y);
        self.set_materials(x, y, r, m, opacity, e);
    }

    /// Set individual emissive value (0.0-1.0), preserving other materials
    pub fn set_emissive(&mut self, x: u32, y: u32, emissive: f32) {
        let (r, m, o, _) = self.get_materials(x, y);
        self.set_materials(x, y, r, m, o, emissive);
    }

    /// Initialize all materials in data_ext to default values (0.5, 0.0, 1.0, 0.0)
    /// This sets roughness=0.5, metallic=0.0, opacity=1.0, emissive=0.0 for all pixels
    /// Preserves any existing normal data
    pub fn set_default_materials(&mut self) {
        self.ensure_data_ext();

        let default_packed = Self::pack_materials(0.5, 0.0, 1.0, 0.0);
        let mat_bytes = default_packed.to_le_bytes();

        if let Some(ext) = self.data_ext.as_mut() {
            for i in (0..ext.len()).step_by(4) {
                ext[i] = mat_bytes[0];
                ext[i + 1] = mat_bytes[1];
                // Preserve bytes i+2 and i+3 (normal data)
            }
        }
    }

    /// Set normal for a pixel, preserving material data
    pub fn set_normal(&mut self, x: u32, y: u32, nx: f32, ny: f32) {
        self.ensure_data_ext();
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        if let Some(ext) = self.data_ext.as_mut() {
            let normal_packed = Self::pack_normal(nx, ny);
            let normal_bytes = normal_packed.to_le_bytes();
            ext[idx + 2] = normal_bytes[0];
            ext[idx + 3] = normal_bytes[1];
            // Preserve bytes 0-1 (material data)
        }
    }

    /// Get normal for a pixel (returns X,Y; Z should be reconstructed in shader)
    /// Returns (0,0) if data_ext not present (flat normal pointing up)
    pub fn get_normal(&self, x: u32, y: u32) -> (f32, f32) {
        let x = x.min((self.width - 1) as u32) as usize;
        let y = y.min((self.height - 1) as u32) as usize;
        let idx = (y * self.width + x) * 4;

        if let Some(ext) = self.data_ext.as_ref() {
            let normal_packed = u16::from_le_bytes([ext[idx + 2], ext[idx + 3]]);
            Self::unpack_normal(normal_packed)
        } else {
            (0.0, 0.0) // Flat normal
        }
    }
}
