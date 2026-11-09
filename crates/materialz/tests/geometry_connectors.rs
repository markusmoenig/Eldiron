#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
fn control(d: &mut Document, id: &str, value: f32) {
    let Operation::Group { controls, .. } = &mut d.rows[0].tiles[0].op else {
        panic!()
    };
    controls.iter_mut().find(|c| c.id == id).unwrap().value = value;
}
#[test]
fn portable_connectors_follow_actual_recipe_parameters_without_adding_geometry() {
    let mut d = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/wall-segment.mzgeometry.json"
    ))
    .unwrap();
    let base = d.evaluate().unwrap().mesh;
    assert_eq!(base.connectors.len(), 3);
    control(&mut d, "width", 2.);
    control(&mut d, "height", 3.);
    let m = Document::load(&d.save().unwrap())
        .unwrap()
        .evaluate()
        .unwrap()
        .mesh;
    assert_eq!(m.connectors[0].position, [-1., 0., 0.]);
    assert_eq!(m.connectors[1].position, [1., 0., 0.]);
    assert_eq!(m.connectors[2].position, [0., 3., 0.]);
    assert_eq!(base.vertex_count(), m.vertex_count());
    let mut transformed = m.clone();
    transformed.transform([5., 1., 7.], [0., 90., 0.], [2., 1., 3.]);
    assert!((transformed.connectors[1].position[2] - 5.).abs() < 1e-5);
    assert!((transformed.connectors[1].direction[2] + 1.).abs() < 1e-5);
}
#[test]
fn repeat_and_assemble_preserve_connectors_with_stable_distinct_ids() {
    let mut d = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/box.mzgeometry.json"
    ))
    .unwrap();
    d.rows[0].tiles.push(Tile {
        id: "copies".into(),
        op: Operation::Repeat {
            count: 3,
            translation: [2., 0., 0.],
            rotation: [0.; 3],
        },
    });
    let a = d.evaluate().unwrap().mesh;
    assert_eq!(a.connectors.len(), 12);
    assert_ne!(a.connectors[0].id, a.connectors[4].id);
    assert!((a.connectors[4].position[0] - a.connectors[0].position[0] - 2.).abs() < 1e-5);
    d.rows.push(Row {
        id: "all".into(),
        label: "All".into(),
        tiles: vec![Tile {
            id: "combine".into(),
            op: Operation::Assemble {
                rows: vec!["recipe".into()],
            },
        }],
    });
    d.output = "all".into();
    let b = d.evaluate().unwrap().mesh;
    assert_eq!(b.connectors.len(), 12);
    assert!(b.connectors[0].id.starts_with("recipe/"));
}
#[test]
fn derived_fence_end_follows_count_spacing_and_invalid_families_fail() {
    let mut d = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/fence.mzgeometry.json"
    ))
    .unwrap();
    control(&mut d, "count", 8.);
    control(&mut d, "spacing", 0.5);
    assert_eq!(
        d.evaluate()
            .unwrap()
            .mesh
            .connectors
            .last()
            .unwrap()
            .position,
        [4., 0., 0.]
    );
    let Operation::Group { document, .. } = &mut d.rows[0].tiles[0].op else {
        panic!()
    };
    let Operation::Connector { family, .. } = &mut document
        .rows
        .last_mut()
        .unwrap()
        .tiles
        .last_mut()
        .unwrap()
        .op
    else {
        panic!()
    };
    *family = "bad family".into();
    assert!(d.evaluate().is_err());
}
