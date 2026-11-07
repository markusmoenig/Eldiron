#![cfg(feature = "integration")]
use denrim_materials::geometry::{Document, Operation};
use std::collections::BTreeMap;

fn frame(size: [f32; 3], position: [f32; 3]) -> denrim_materials::geometry::Mesh {
    let mut doc = Document::load(denrim_materials::geometry::PITCHED_ROOF_RECIPE).unwrap();
    let Operation::Input { preview, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    let Operation::Box { size: target } = &mut preview.rows[0].tiles[0].op else {
        panic!()
    };
    *target = size;
    let mut mesh = preview.evaluate().unwrap().mesh;
    mesh.transform(position, [0.; 3], [1.; 3]);
    mesh
}

#[test]
fn roof_is_portable_and_fits_host_geometry_without_including_it_in_output() {
    let doc = Document::load(denrim_materials::geometry::PITCHED_ROOF_RECIPE).unwrap();
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
    let input = frame([8., 3., 6.], [10., 1.5, -2.]);
    let result = doc
        .evaluate_with_inputs(&BTreeMap::from([("selection".into(), input)]))
        .unwrap()
        .mesh;
    let (lo, hi) = result.bounds();
    for (a, b) in lo
        .into_iter()
        .chain(hi)
        .zip([5.75, 3., -5.25, 14.25, 4.14, 1.25])
    {
        assert!((a - b).abs() < 0.0001, "{a} != {b}");
    }
    assert_eq!(result.parts.len(), 2);
    assert!(result.parts.iter().all(|p| p.id.ends_with("slope")));
}

#[test]
fn controls_preserve_thickness_and_reject_invalid_fits() {
    let mut doc = Document::load(denrim_materials::geometry::PITCHED_ROOF_RECIPE).unwrap();
    let Operation::Group { controls, .. } = &mut doc.rows[1].tiles[0].op else {
        panic!()
    };
    controls[0].value = 2.;
    let mesh = doc.evaluate().unwrap().mesh;
    assert!((mesh.bounds().1[1] - 3.14).abs() < 0.0001);
    let Operation::FitTo { source_anchor, .. } = &mut doc.rows[1].tiles[1].op else {
        panic!()
    };
    source_anchor[0] = f32::NAN;
    assert!(doc.evaluate().is_err());
}
