//! Clipboard images are editable CPU assets. PNG transport never uses a canvas,
//! scene capture, presentation buffer or GPU device.
use image::{ImageEncoder, ImageFormat, ImageReader, codecs::png::PngEncoder};
use std::io::Cursor;
use theframework_core::ImageAsset;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 8192;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageClipboardRequest {
    Read { request: u64 },
    Write { request: u64, image: ImageAsset },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageClipboardResult {
    Written(bool),
    Image(Option<ImageAsset>),
}
fn dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) * 4 > MAX_BYTES as u64
    {
        return Err("Clipboard image exceeds the dimension or byte limit".into());
    }
    Ok(())
}
pub fn validate(image: &ImageAsset) -> Result<(), String> {
    if image.color_space != theframework_core::ColorSpace::Srgb
        || image.alpha_mode != theframework_core::AlphaMode::Straight
    {
        return Err("Clipboard images require straight sRGB pixels".into());
    }
    dimensions(image.width(), image.height())
}
pub fn encode_png(image: &ImageAsset) -> Result<Vec<u8>, String> {
    validate(image)?;
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            image.pixels(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Clipboard PNG exceeds the byte limit".into());
    }
    Ok(bytes)
}
pub fn decode_png(bytes: &[u8]) -> Result<ImageAsset, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Clipboard PNG exceeds the byte limit".into());
    }
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png)
        .into_dimensions()
        .map_err(|e| e.to_string())?;
    dimensions(width, height)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_BYTES as u64);
    reader.limits(limits);
    let pixels = reader
        .decode()
        .map_err(|e| e.to_string())?
        .into_rgba8()
        .into_raw();
    ImageAsset::rgba(width, height, pixels).map_err(|e| e.to_string())
}
