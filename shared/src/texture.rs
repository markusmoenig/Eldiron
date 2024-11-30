use euc::Texture;
use vek::Rgba;

#[derive(Debug, Clone)]
pub struct RgbaTexture {
    data: Vec<u8>,
    width: usize,
    height: usize,
}

impl RgbaTexture {
    /// Create a new `RgbaTexture` from an RGBA array.
    ///
    /// # Arguments
    /// - `data`: A `Vec<u8>` containing the RGBA data. Must have a length of `width * height * 4`.
    /// - `width`: The width of the image.
    /// - `height`: The height of the image.
    ///
    /// # Panics
    /// Panics if the `data` length does not match `width * height * 4`.
    pub fn new(data: Vec<u8>, width: usize, height: usize) -> Self {
        assert_eq!(
            data.len(),
            width * height * 4,
            "Data size does not match image dimensions"
        );
        Self {
            data,
            width,
            height,
        }
    }
}

impl Texture<2> for RgbaTexture {
    type Index = usize;
    type Texel = Rgba<f32>; // Directly return `Rgba<f32>` as the texel type.

    fn size(&self) -> [Self::Index; 2] {
        [self.width, self.height]
    }

    fn preferred_axes(&self) -> Option<[usize; 2]> {
        Some([0, 1])
    }

    fn read(&self, index: [Self::Index; 2]) -> Self::Texel {
        let x = index[0];
        let y = index[1];
        assert!(x < self.width && y < self.height, "Index out of bounds");

        let idx = (y * self.width + x) * 4;
        Rgba::new(
            self.data[idx] as f32 / 255.0,
            self.data[idx + 1] as f32 / 255.0,
            self.data[idx + 2] as f32 / 255.0,
            self.data[idx + 3] as f32 / 255.0,
        )
    }

    unsafe fn read_unchecked(&self, index: [Self::Index; 2]) -> Self::Texel {
        let idx = (index[1] * self.width + index[0]) * 4;
        Rgba::new(
            self.data[idx] as f32 / 255.0,
            self.data[idx + 1] as f32 / 255.0,
            self.data[idx + 2] as f32 / 255.0,
            self.data[idx + 3] as f32 / 255.0,
        )
    }
}
