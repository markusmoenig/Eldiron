//! Bounded original subrectangle decoding. No original PNG/depth byte buffer or
//! whole decoded image is constructed. Current encoded strings remain unchanged.
//! Source-page owners can advance rows/chunks cooperatively and retire the
//! decoder at any boundary without exposing incomplete pixels.
mod base64_source;
mod png;
pub use png::PngRegionDecoder;

/// Top-left pixel origin and extent in one original square bake tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OriginalRegion {
    pub origin: [u32; 2],
    pub size: [u32; 2],
}
impl OriginalRegion {
    pub const MAX_SIDE: u32 = 512;
    pub fn validate(self, original_size: u32) -> Result<usize, String> {
        if !(0..2).all(|axis| {
            self.size[axis] > 0
                && self.size[axis] <= Self::MAX_SIDE
                && self.origin[axis]
                    .checked_add(self.size[axis])
                    .is_some_and(|end| end <= original_size)
        }) {
            return Err(
                "Original source rectangle is empty, out of bounds or exceeds 512px".into(),
            );
        }
        Ok(self.size[0] as usize * self.size[1] as usize)
    }
}

/// Synchronous reference/convenience wrapper; asynchronous owners use the row
/// decoder directly. This still allocates only original rows plus the region.
pub fn decode_png_region(
    encoded: &str,
    original_size: u32,
    region: OriginalRegion,
) -> Result<Vec<u8>, String> {
    let mut work = PngRegionDecoder::new(encoded, original_size, region)?;
    while !work.advance(64)? {}
    work.finish()
}

mod depth;
pub use depth::DepthRegionDecoder;

/// Synchronous convenience wrapper over the same cooperative depth owner.
pub fn decode_depth_region(
    encoded: &str,
    original_size: u32,
    region: OriginalRegion,
) -> Result<Vec<f32>, String> {
    let mut work = DepthRegionDecoder::new(encoded, original_size, region)?;
    while !work.advance(64)? {}
    work.finish()
}

#[cfg(test)]
mod tests;
