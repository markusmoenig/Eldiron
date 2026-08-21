use std::error::Error;
use std::fmt::{Display, Formatter};

/// A signed pixel rectangle. Negative origins are valid; non-positive sizes are empty.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ThePixelRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ThePixelRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub fn intersection(self, other: Self) -> Self {
        let left = i64::from(self.x).max(i64::from(other.x));
        let top = i64::from(self.y).max(i64::from(other.y));
        let right = (i64::from(self.x) + i64::from(self.width).max(0))
            .min(i64::from(other.x) + i64::from(other.width).max(0));
        let bottom = (i64::from(self.y) + i64::from(self.height).max(0))
            .min(i64::from(other.y) + i64::from(other.height).max(0));

        if right <= left || bottom <= top {
            return Self::new(
                left.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                top.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                0,
                0,
            );
        }

        Self::new(
            left as i32,
            top as i32,
            (right - left) as i32,
            (bottom - top) as i32,
        )
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        if self.is_empty() {
            return false;
        }
        let x = i64::from(x);
        let y = i64::from(y);
        x >= i64::from(self.x)
            && y >= i64::from(self.y)
            && x < i64::from(self.x) + i64::from(self.width)
            && y < i64::from(self.y) + i64::from(self.height)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TheSurfaceError {
    DimensionTooLarge { width: usize, height: usize },
    BufferTooSmall { required: usize, actual: usize },
}

impl Display for TheSurfaceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionTooLarge { width, height } => {
                write!(
                    formatter,
                    "surface dimensions {width}x{height} are too large"
                )
            }
            Self::BufferTooSmall { required, actual } => write!(
                formatter,
                "surface needs {required} RGBA bytes, but the buffer contains {actual}"
            ),
        }
    }
}

impl Error for TheSurfaceError {}

/// A validated RGBA8 drawing target with a mandatory clip rectangle.
///
/// Drawing through this type cannot address the guard bytes after the declared surface and cannot
/// write outside its current clip. It is intended to become the boundary between widget code and
/// raster backends while legacy `TheDraw2D` calls are migrated incrementally.
pub struct TheSurfaceMut<'a> {
    pixels: &'a mut [u8],
    width: usize,
    height: usize,
    clip: ThePixelRect,
}

impl<'a> TheSurfaceMut<'a> {
    pub fn new(pixels: &'a mut [u8], width: usize, height: usize) -> Result<Self, TheSurfaceError> {
        if width > i32::MAX as usize || height > i32::MAX as usize {
            return Err(TheSurfaceError::DimensionTooLarge { width, height });
        }

        let required = width
            .checked_mul(height)
            .and_then(|count| count.checked_mul(4))
            .ok_or(TheSurfaceError::DimensionTooLarge { width, height })?;
        if pixels.len() < required {
            return Err(TheSurfaceError::BufferTooSmall {
                required,
                actual: pixels.len(),
            });
        }

        Ok(Self {
            pixels,
            width,
            height,
            clip: ThePixelRect::new(0, 0, width as i32, height as i32),
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn bounds(&self) -> ThePixelRect {
        ThePixelRect::new(0, 0, self.width as i32, self.height as i32)
    }

    pub fn clip(&self) -> ThePixelRect {
        self.clip
    }

    /// Sets the clip and returns its clamped value.
    pub fn set_clip(&mut self, clip: ThePixelRect) -> ThePixelRect {
        self.clip = clip.intersection(self.bounds());
        self.clip
    }

    pub fn reset_clip(&mut self) {
        self.clip = self.bounds();
    }

    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        if !self.bounds().contains(x, y) {
            return None;
        }
        let index = self.pixel_index(x, y);
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ])
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if !self.clip.contains(x, y) {
            return;
        }
        let index = self.pixel_index(x, y);
        self.pixels[index..index + 4].copy_from_slice(&color);
    }

    /// Blends a straight-alpha RGBA color through an 8-bit coverage mask.
    pub fn blend_pixel_coverage(&mut self, x: i32, y: i32, color: [u8; 4], coverage: u8) {
        if coverage == 0 || !self.clip.contains(x, y) {
            return;
        }

        let index = self.pixel_index(x, y);
        let source_alpha = (color[3] as f32 / 255.0) * (coverage as f32 / 255.0);
        let destination_alpha = self.pixels[index + 3] as f32 / 255.0;
        let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

        if output_alpha <= f32::EPSILON {
            self.pixels[index..index + 4].fill(0);
            return;
        }

        for channel in 0..3 {
            let source = color[channel] as f32 / 255.0;
            let destination = self.pixels[index + channel] as f32 / 255.0;
            let output = (source * source_alpha
                + destination * destination_alpha * (1.0 - source_alpha))
                / output_alpha;
            self.pixels[index + channel] = (output * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        self.pixels[index + 3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    pub fn fill_rect(&mut self, rect: ThePixelRect, color: [u8; 4]) {
        let clipped = rect.intersection(self.clip);
        if clipped.is_empty() {
            return;
        }

        let end_x = clipped.x + clipped.width;
        let end_y = clipped.y + clipped.height;
        for y in clipped.y..end_y {
            for x in clipped.x..end_x {
                self.set_pixel(x, y, color);
            }
        }
    }

    fn pixel_index(&self, x: i32, y: i32) -> usize {
        (y as usize * self.width + x as usize) * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_dimensions_and_buffer_size() {
        let mut too_small = [0; 15];
        assert_eq!(
            TheSurfaceMut::new(&mut too_small, 2, 2).err(),
            Some(TheSurfaceError::BufferTooSmall {
                required: 16,
                actual: 15,
            })
        );

        let mut empty = [];
        assert!(matches!(
            TheSurfaceMut::new(&mut empty, i32::MAX as usize + 1, 0),
            Err(TheSurfaceError::DimensionTooLarge { .. })
        ));
    }

    #[test]
    fn clipping_preserves_pixels_and_guard_bytes() {
        const GUARD: u8 = 0xa7;
        let mut storage = vec![GUARD; 4 * 4 * 4 + 32];
        {
            let mut surface = TheSurfaceMut::new(&mut storage, 4, 4).unwrap();
            surface.set_clip(ThePixelRect::new(1, 1, 2, 2));
            surface.fill_rect(ThePixelRect::new(-20, -20, 50, 50), [4, 8, 15, 255]);
        }

        for y in 0..4 {
            for x in 0..4 {
                let index = (y * 4 + x) * 4;
                if (1..3).contains(&x) && (1..3).contains(&y) {
                    assert_eq!(&storage[index..index + 4], &[4, 8, 15, 255]);
                } else {
                    assert_eq!(&storage[index..index + 4], &[GUARD; 4]);
                }
            }
        }
        assert!(storage[4 * 4 * 4..].iter().all(|byte| *byte == GUARD));
    }

    #[test]
    fn arbitrary_rectangles_never_escape_the_clip() {
        for width in 0..9usize {
            for height in 0..9usize {
                let data_len = width * height * 4;
                let mut storage = vec![0x39; data_len + 17];
                {
                    let mut surface = TheSurfaceMut::new(&mut storage, width, height).unwrap();
                    let clip = surface.set_clip(ThePixelRect::new(1, 1, 3, 4));
                    for seed in 0..100i32 {
                        let x = seed.wrapping_mul(37) % 31 - 15;
                        let y = seed.wrapping_mul(53) % 29 - 14;
                        let w = seed.wrapping_mul(17) % 24 - 4;
                        let h = seed.wrapping_mul(23) % 24 - 4;
                        surface.fill_rect(ThePixelRect::new(x, y, w, h), [seed as u8, 2, 3, 255]);
                        surface.blend_pixel_coverage(x, y, [255, 0, 0, 192], seed as u8);
                    }

                    for y in 0..height as i32 {
                        for x in 0..width as i32 {
                            if !clip.contains(x, y) {
                                assert_eq!(surface.pixel(x, y), Some([0x39; 4]));
                            }
                        }
                    }
                }
                assert!(storage[data_len..].iter().all(|byte| *byte == 0x39));
            }
        }
    }

    #[test]
    fn blends_straight_alpha_with_coverage() {
        let mut storage = vec![0; 4];
        let mut surface = TheSurfaceMut::new(&mut storage, 1, 1).unwrap();
        surface.set_pixel(0, 0, [20, 40, 60, 255]);
        surface.blend_pixel_coverage(0, 0, [220, 140, 40, 255], 128);
        assert_eq!(surface.pixel(0, 0), Some([120, 90, 50, 255]));
    }
}
