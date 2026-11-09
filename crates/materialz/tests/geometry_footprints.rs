#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
fn set(d: &mut Document, id: &str, value: f32) {
    let Operation::Group { controls, .. } = &mut d.rows[0].tiles[0].op else {
        panic!()
    };
    controls.iter_mut().find(|c| c.id == id).unwrap().value = value;
}
fn close(a: [f32; 3], b: [f32; 3]) {
    assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-5), "{a:?} != {b:?}");
}
#[test]
fn concave_footprint_is_portable_and_does_not_add_visible_geometry() {
    let mut d = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/wall-corner.mzgeometry.json"
    ))
    .unwrap();
    for (id, value) in [("width", 2.), ("depth", 1.4), ("thickness", 0.3)] {
        set(&mut d, id, value);
    }
    let result = d.evaluate().unwrap();
    assert_eq!(result.mesh.parts.len(), 2);
    let expected = [
        [0., 0., 0.],
        [2., 0., 0.],
        [2., 0., 0.3],
        [0.3, 0., 0.3],
        [0.3, 0., 1.4],
        [0., 0., 1.4],
    ];
    for (a, b) in result.mesh.footprint.as_ref().unwrap().regions[0]
        .outer
        .iter()
        .zip(expected)
    {
        close(*a, b);
    }
    let source = d.save().unwrap();
    assert_eq!(
        Document::load(&source).unwrap().evaluate().unwrap().mesh,
        result.mesh
    );
    let Operation::Group { document, .. } = &mut d.rows[0].tiles[0].op else {
        panic!()
    };
    document
        .rows
        .last_mut()
        .unwrap()
        .tiles
        .retain(|t| !matches!(t.op, Operation::Footprint { .. }));
    let mesh = d.evaluate().unwrap().mesh;
    assert!(mesh.footprint.is_none());
    assert_eq!(mesh.parts, result.mesh.parts);
}
#[test]
fn tree_footprint_matches_trunk_vertices_and_follows_published_dimensions() {
    let mut d = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/tree.mzgeometry.json"
    ))
    .unwrap();
    for (w, depth, trunk_w, trunk_d) in [
        (1.5, 1.2, 0.32, 0.28),
        (3., 2., 0.6, 0.4),
        (0.5, 0.8, 0.06, 0.06),
    ] {
        for (id, value) in [
            ("width", w),
            ("depth", depth),
            ("trunk-width", trunk_w),
            ("trunk-depth", trunk_d),
        ] {
            set(&mut d, id, value);
        }
        let m = d.evaluate().unwrap().mesh;
        let points = &m.footprint.as_ref().unwrap().regions[0].outer;
        assert_eq!(points.len(), 8);
        let trunk = m.parts.iter().find(|p| p.id == "trunk").unwrap();
        for p in points {
            assert!(
                trunk
                    .vertices
                    .iter()
                    .any(|v| (0..3).all(|k| (v.position[k] - p[k]).abs() < 1e-5)),
                "Footprint must follow actual trunk geometry: {p:?}"
            );
        }
        let (lo, hi) = m.bounds();
        let min_x = points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_x - min_x < (hi[0] - lo[0]) * 0.6,
            "Canopy must not become collision"
        );
    }
}
#[test]
fn footprint_transforms_with_mesh_and_invalid_bindings_or_polygons_are_rejected() {
    let source = r#"{"version":1,"id":"shape","label":"Shape","rows":[{"id":"outline","label":"Outline","tiles":[{"id":"contour","op":{"type":"contour","points":[[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]]}}]},{"id":"mesh","label":"Mesh","tiles":[{"id":"box","op":{"type":"box","size":[1,1,1]}},{"id":"footprint","op":{"type":"footprint","row":"outline"}},{"id":"place","op":{"type":"transform","translation":[4,0,7],"rotation":[0,90,0],"scale":[2,1,3]}}]}],"output":"mesh"}"#;
    let d = Document::load(source).unwrap();
    let m = d.evaluate().unwrap().mesh;
    let expected = [[2.5, 0., 8.], [2.5, 0., 6.], [5.5, 0., 6.], [5.5, 0., 8.]];
    for (a, b) in m.footprint.unwrap().regions[0].outer.iter().zip(expected) {
        close(*a, b);
    }
    assert!(Document::load(&source.replace("\"row\":\"outline\"", "\"row\":\"missing\"")).is_err());
    assert!(
        Document::load(&source.replace("[0.5,0.5],[-0.5,0.5]", "[-0.5,0.5],[0.5,0.5]")).is_err()
    );
    let mut d = d;
    d.rows[1].tiles.push(Tile {
        id: "repeat".into(),
        op: Operation::Repeat {
            count: 2,
            translation: [2., 0., 0.],
            rotation: [0.; 3],
        },
    });
    assert_eq!(
        d.evaluate().unwrap().mesh.footprint.unwrap().regions.len(),
        2
    );
}
#[test]
fn holes_and_repeated_disconnected_footprints_stay_portable_and_parameter_driven() {
    let mut ring = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/ring-walkway.mzgeometry.json"
    ))
    .unwrap();
    set(&mut ring, "opening_width", 1.0);
    let mesh = ring.evaluate().unwrap().mesh;
    let shape = mesh.footprint.unwrap().map(|p| [p[0], p[2]]);
    assert_eq!(shape.regions.len(), 1);
    assert_eq!(shape.regions[0].holes.len(), 1);
    assert!(!footprint::contains(&shape, [0., 0.]));
    assert!(footprint::contains(&shape, [1., 0.]));
    assert!(
        footprint::contains(&shape, [0.6, 0.]),
        "Narrowing the opening must update the footprint"
    );
    assert_eq!(mesh.parts.len(), 4);
    assert_eq!(Document::load(&ring.save().unwrap()).unwrap(), ring);
    let doorway = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/doorway.mzgeometry.json"
    ))
    .unwrap();
    let mesh = doorway.evaluate().unwrap().mesh;
    assert_eq!(mesh.parts.len(), 3);
    let shape = mesh.footprint.unwrap().map(|p| [p[0], p[2]]);
    assert_eq!(shape.regions.len(), 2);
    assert!(!footprint::contains(&shape, [0., 0.]));
    assert!(footprint::contains(&shape, [0.6, 0.]));
    let mut posts = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/repeated-posts.mzgeometry.json"
    ))
    .unwrap();
    set(&mut posts, "count", 3.);
    set(&mut posts, "spacing", 2.);
    let shape = posts
        .evaluate()
        .unwrap()
        .mesh
        .footprint
        .unwrap()
        .map(|p| [p[0], p[2]]);
    assert_eq!(shape.regions.len(), 3);
    for x in [0., 2., 4.] {
        assert!(footprint::contains(&shape, [x, 0.]));
    }
    for x in [1., 3.] {
        assert!(!footprint::contains(&shape, [x, 0.]));
    }
    assert_eq!(Document::load(&posts.save().unwrap()).unwrap(), posts);
}
#[test]
fn compound_validation_rejects_invalid_holes_and_limits_but_accepts_islands() {
    use footprint::{Region, Shape};
    let square = |a: f32, b: f32| vec![[a, a], [b, a], [b, b], [a, b]];
    let valid = Shape {
        regions: vec![Region {
            outer: square(-2., 2.),
            holes: vec![square(-1., 1.)],
        }],
    };
    footprint::validate(&valid).unwrap();
    let mut reversed = valid.clone();
    reversed.regions[0].outer.reverse();
    reversed.regions[0].holes[0].reverse();
    footprint::validate(&reversed).unwrap();
    for holes in [
        vec![square(-2., 1.)],
        vec![square(-1., 3.)],
        vec![square(-1., 1.), square(-0.5, 0.5)],
        vec![square(-1., 0.5), square(-0.5, 1.)],
    ] {
        assert!(
            footprint::validate(&Shape {
                regions: vec![Region {
                    outer: square(-2., 2.),
                    holes
                }]
            })
            .is_err()
        );
    }
    let mut island = valid.clone();
    island.regions.push(Region {
        outer: square(-0.2, 0.2),
        holes: vec![],
    });
    footprint::validate(&island).unwrap();
    assert!(footprint::contains(&island, [0., 0.]));
    assert!(!footprint::contains(&island, [0.5, 0.5]));
    let too_many = Shape {
        regions: vec![
            Region {
                outer: square(-1., 1.),
                holes: vec![]
            };
            129
        ],
    };
    assert!(footprint::validate(&too_many).is_err());
}
