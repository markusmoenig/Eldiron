//! Captured from TheFramework at77b3625d before physical data extraction.
mod fixture_api {
    pub use theframework_core::*;
    pub use vek::*;
}
#[path = "support/value_fixture.rs"]
mod fixture;
#[test]
fn core_values_match_pre_extraction_json_and_binary() {
    let original = fixture::fixture();
    let json = include_bytes!("data/legacy-values.json");
    let binary = include_bytes!("data/legacy-values.bin");
    assert_eq!(serde_json::to_vec(&original).unwrap().as_slice(), json);
    assert_eq!(bincode::serialize(&original).unwrap().as_slice(), binary);
    assert_eq!(
        serde_json::from_slice::<fixture::Fixture>(json).unwrap(),
        original
    );
    // The old unit-density bitmap skips a struct field and cannot round-trip
    // through bincode. Preserve its bytes; validate decoding with the readable
    // fractional-density fixture captured from those same original definitions.
    let binary = include_bytes!("data/legacy-readable-values.bin");
    let mut readable = fixture::fixture();
    readable.values.retain(
        |value| !matches!(value, fixture_api::TheValue::Image(image) if image.render_scale() == 1.),
    );
    readable
        .tile
        .buffer
        .retain(|image| image.render_scale() != 1.);
    readable.descriptions = readable
        .values
        .iter()
        .map(fixture_api::TheValue::describe)
        .collect();
    assert_eq!(bincode::serialize(&readable).unwrap().as_slice(), binary);
    assert_eq!(
        bincode::deserialize::<fixture::Fixture>(binary).unwrap(),
        readable
    );
}

#[test]
fn rgb_bitmap_keeps_pre_extraction_data_and_conversion() {
    use theframework_core::{TheDim, TheRGBBuffer};
    let mut rgb = TheRGBBuffer::new(TheDim::new(-3, 7, 3, 2));
    for (i, p) in rgb.pixels_mut().iter_mut().enumerate() {
        *p = (i * 37 % 256) as u8;
    }
    let json = include_bytes!("data/legacy-rgb.json");
    let binary = include_bytes!("data/legacy-rgb.bin");
    assert_eq!(serde_json::to_vec(&rgb).unwrap().as_slice(), json);
    assert_eq!(bincode::serialize(&rgb).unwrap().as_slice(), binary);
    assert_eq!(serde_json::from_slice::<TheRGBBuffer>(json).unwrap(), rgb);
    assert_eq!(bincode::deserialize::<TheRGBBuffer>(binary).unwrap(), rgb);
    let rgba = rgb.to_rgba();
    for (src, dst) in rgb
        .pixels()
        .chunks_exact(3)
        .zip(rgba.pixels().chunks_exact(4))
    {
        assert_eq!(&dst[..3], src);
        assert_eq!(dst[3], 255);
    }
}
