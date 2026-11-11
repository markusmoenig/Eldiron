use serde_json::json;
use theframework_core::*;
#[test]
fn persisted_data_keeps_legacy_shapes() {
    let color = TheColor {
        r: 0.25,
        g: 0.5,
        b: 0.75,
        a: 1.,
        name: "ink".into(),
    };
    assert_eq!(
        serde_json::to_value(&color).unwrap(),
        json!({"r":0.25,"g":0.5,"b":0.75,"a":1.,"name":"ink"})
    );
    let palette = ThePalette::new(vec![Some(color), None]);
    assert_eq!(
        serde_json::to_value(&palette).unwrap()["current_index"],
        json!(0)
    );
    assert_eq!(
        serde_json::to_value(&palette).unwrap()["colors"][1],
        json!(null)
    );
    let restored: ThePalette = serde_json::from_value(json!({"colors":[null]})).unwrap();
    assert_eq!(restored.current_index, 0);
    assert_eq!(
        serde_json::to_value(TheTime::new_time(8, 30).unwrap()).unwrap(),
        json!({"hours":8,"minutes":30,"seconds":0})
    );
    assert_eq!(
        serde_json::to_value(TheDim::new(1, 2, 3, 4)).unwrap(),
        json!({"x":1,"y":2,"width":3,"height":4,"buffer_x":0,"buffer_y":0})
    );
}
#[test]
fn image_data_validates_storage_without_ui_dimensions() {
    assert!(ImageAsset::rgba(2, 2, vec![0; 15]).is_err());
    let mut image = ImageAsset::rgba(2, 1, vec![0; 8]).unwrap();
    image.pixels_mut()[4..].copy_from_slice(&[1, 2, 3, 4]);
    assert_eq!(&image.pixels()[4..], &[1, 2, 3, 4]);
    assert_eq!((image.width(), image.height()), (2, 1));
}
