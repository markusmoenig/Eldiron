//! The old module/prelude paths must denote the exact CPU-core types.
mod fixture_api {
    pub use theframework::prelude::*;
}
#[path = "../../theframework-core/tests/support/value_fixture.rs"]
mod fixture;
use theframework::prelude::*;
#[test]
fn legacy_paths_keep_serialized_values_and_core_type_identity() {
    let fixture = fixture::fixture();
    assert_eq!(
        serde_json::to_vec(&fixture).unwrap().as_slice(),
        include_bytes!("../../theframework-core/tests/data/legacy-values.json")
    );
    assert_eq!(
        bincode::serialize(&fixture).unwrap().as_slice(),
        include_bytes!("../../theframework-core/tests/data/legacy-values.bin")
    );
    fn core_value(value: theframework_core::TheValue) -> theframework::thevalue::TheValue {
        value
    }
    assert_eq!(core_value(fixture.values[0].clone()), fixture.values[0]);
    fn core_bitmap(
        value: theframework_core::TheRGBABuffer,
    ) -> theframework::thergbabuffer::TheRGBABuffer {
        value
    }
    assert_eq!(
        core_bitmap(fixture.tile.buffer[0].clone()),
        fixture.tile.buffer[0]
    );
    #[cfg(feature = "ui")]
    {
        fn core_rgb(
            value: theframework_core::TheRGBBuffer,
        ) -> theframework::theui::thergbbuffer::TheRGBBuffer {
            value
        }
        let rgb = theframework_core::TheRGBBuffer::new(TheDim::sized(2, 1));
        assert_eq!(core_rgb(rgb.clone()), rgb);
    }
}
#[test]
fn explicit_reference_text_keeps_pre_extraction_pixels_at_fractional_density() {
    let bytes = theframework::Embedded::get("fonts/Roboto-Bold.ttf").unwrap();
    let font = fontdue::Font::from_bytes(bytes.data, fontdue::FontSettings::default()).unwrap();
    let mut image = TheRGBABuffer::new(TheDim::sized(160, 48));
    image.set_render_scale(1.5);
    image.fill([13, 21, 34, 89]);
    image.draw_text(
        Vec2::new(3, 4),
        &font,
        "Core Åß",
        14.,
        [210, 170, 90, 230],
        TheHorizontalAlign::Left,
        TheVerticalAlign::Top,
    );
    assert_eq!(
        image.to_png().unwrap().as_slice(),
        include_bytes!("../../theframework-core/tests/data/legacy-bitmap-text.png")
    );
}
