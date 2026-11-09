#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
use group::{Control, Modifier};
use parameters::Target;
fn base() -> Document {
    Document::load(r#"{"version":1,"id":"unit","label":"Unit","rows":[{"id":"piece","label":"Piece","tiles":[{"id":"box","op":{"type":"box","size":[0.1,1,0.2]}},{"id":"lift","op":{"type":"transform","translation":[0,0.5,0],"rotation":[0,0,0],"scale":[1,1,1]}}]}],"output":"piece"}"#).unwrap()
}
fn binding(operation: &str, parameter: &str, factor: f32, offset: f32) -> Modifier {
    Modifier::Parameter {
        target: Target {
            path: vec![],
            operation: operation.into(),
            parameter: parameter.into(),
        },
        factor,
        offset,
    }
}
fn control(id: &str, value: f32, min: f32, max: f32, modifiers: Vec<Modifier>) -> Control {
    Control {
        expression: None,
        id: id.into(),
        label: id.into(),
        category: "Construction".into(),
        help: String::new(),
        order: 0,
        value,
        reference: value,
        min,
        max,
        modifiers,
    }
}
fn wrap(document: Document, controls: Vec<Control>) -> Document {
    Document {
        version: 1,
        id: "parent".into(),
        label: "Parent".into(),
        rows: vec![Row {
            id: "group".into(),
            label: "Group".into(),
            tiles: vec![Tile {
                id: "group-op".into(),
                op: Operation::Group {
                    label: "Group".into(),
                    document: Box::new(document),
                    controls,
                },
            }],
        }],
        output: "group".into(),
        grip: None,
    }
}
fn settings(doc: &mut Document) -> &mut Vec<Control> {
    let Operation::Group { controls, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    controls
}
#[test]
fn construction_bindings_rebuild_geometry_before_post_modifiers_and_do_not_rewrite_source() {
    let inner = base();
    let mut doc = wrap(
        inner.clone(),
        vec![
            control(
                "height",
                2.,
                0.1,
                5.,
                vec![
                    binding("box", "size1", 1., 0.),
                    binding("lift", "position1", 0.5, 0.),
                ],
            ),
            control(
                "offset",
                1.,
                0.,
                3.,
                vec![Modifier::Affine {
                    parts: vec!["*".into()],
                    joints: vec![],
                    pivot: [0.; 3],
                    scale: [0.; 3],
                    translation: [1., 0., 0.],
                    weight: None,
                    frames: false,
                }],
            ),
        ],
    );
    // Post offset amount uses value-reference; make it active.
    settings(&mut doc)[1].reference = 0.;
    let e = doc.evaluate().unwrap();
    let (lo, hi) = e.mesh.bounds();
    assert!(lo[1].abs() < 0.00001);
    assert!((hi[1] - 2.).abs() < 0.00001);
    assert!((lo[0] - 0.95).abs() < 0.00001);
    let Operation::Group { document, .. } = &doc.rows[0].tiles[0].op else {
        panic!()
    };
    assert_eq!(**document, inner);
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
    // UVs were constructed for the new box, rather than deforming an already-built box.
    let expected = parameters::bind(&inner, settings(&mut doc))
        .unwrap()
        .evaluate()
        .unwrap();
    for (a, b) in e.mesh.parts[0]
        .vertices
        .iter()
        .zip(&expected.mesh.parts[0].vertices)
    {
        assert_eq!(a.uv, b.uv);
    }
}
#[test]
fn repetition_changes_topology_retains_material_uvs_and_stable_copy_ids() {
    let mut inner = base();
    inner.rows[0].tiles.insert(
        1,
        Tile {
            id: "material".into(),
            op: Operation::Material {
                palette: 19,
                roughness: 0.8,
                metallic: 0.,
            },
        },
    );
    // Keep the mesh pipeline and material row separate.
    let material = inner.rows[0].tiles.remove(1);
    inner.rows.insert(
        0,
        Row {
            id: "wood".into(),
            label: "Wood".into(),
            tiles: vec![material],
        },
    );
    inner.rows[1].tiles.push(Tile {
        id: "surface".into(),
        op: Operation::AssignMaterial { row: "wood".into() },
    });
    inner.rows[1].tiles.push(Tile {
        id: "repeat".into(),
        op: Operation::Repeat {
            count: 3,
            translation: [0.5, 0., 0.],
            rotation: [0.; 3],
        },
    });
    let mut doc = wrap(
        inner,
        vec![
            control(
                "copies",
                3.,
                1.,
                128.,
                vec![binding("repeat", "count", 1., 0.)],
            ),
            control(
                "spacing",
                0.5,
                0.1,
                2.,
                vec![binding("repeat", "step0", 1., 0.)],
            ),
        ],
    );
    let first = doc.evaluate().unwrap().mesh;
    assert_eq!(first.parts.len(), 3);
    assert_eq!(first.vertex_count(), 108);
    settings(&mut doc)[0].value = 5.;
    settings(&mut doc)[1].value = 0.75;
    let next = doc.evaluate().unwrap().mesh;
    assert_eq!(next.parts.len(), 5);
    assert_eq!(next.vertex_count(), 180);
    for (i, p) in next.parts.iter().enumerate() {
        assert_eq!(p.surface.as_ref().unwrap().palette, 19);
        if i < 3 {
            assert_eq!(p.id, first.parts[i].id);
        }
        for (v, base) in p.vertices.iter().zip(&next.parts[0].vertices) {
            assert_eq!(v.uv, base.uv);
            assert!((v.position[0] - base.position[0] - i as f32 * 0.75).abs() < 0.00001);
        }
    }
    settings(&mut doc)[0].value = 2.;
    assert_eq!(doc.evaluate().unwrap().mesh.parts.len(), 2);
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
}
#[test]
fn nested_group_interfaces_compose_and_invalid_bindings_are_rejected() {
    let child = wrap(
        base(),
        vec![control(
            "height",
            1.,
            0.1,
            5.,
            vec![
                binding("box", "size1", 1., 0.),
                binding("lift", "position1", 0.5, 0.),
            ],
        )],
    );
    assert!(
        parameters::catalog(&child)
            .iter()
            .any(|p| p.target.parameter == "control:height")
    );
    assert!(
        !parameters::catalog(&child)
            .iter()
            .any(|p| p.target.parameter == "size1")
    );
    let mut parent = wrap(
        child,
        vec![control(
            "outer-height",
            3.,
            0.1,
            5.,
            vec![binding("group-op", "control:height", 1., 0.)],
        )],
    );
    assert!((parent.evaluate().unwrap().mesh.bounds().1[1] - 3.).abs() < 0.00001);
    let valid = parent.clone();
    settings(&mut parent)[0]
        .modifiers
        .push(binding("group-op", "control:height", 1., 0.));
    assert!(
        parent
            .evaluate()
            .unwrap_err()
            .to_string()
            .contains("more than once")
    );
    for modifier in [
        binding("deleted", "size1", 1., 0.),
        binding("group-op", "identity", 1., 0.),
        binding("group-op", "control:height", f32::NAN, 0.),
        binding("group-op", "control:height", 10., 0.),
    ] {
        let mut bad = valid.clone();
        settings(&mut bad)[0].modifiers = vec![modifier];
        assert!(bad.evaluate().is_err());
    }
    assert_eq!(valid.evaluate().unwrap().mesh.bounds().1[1], 3.);
}
#[test]
fn repeats_validate_count_and_geometry_budget_before_allocating_copies() {
    let mut doc = base();
    doc.rows[0].tiles.push(Tile {
        id: "repeat".into(),
        op: Operation::Repeat {
            count: 0,
            translation: [1., 0., 0.],
            rotation: [0.; 3],
        },
    });
    assert!(doc.evaluate().is_err());
    let Operation::Repeat { count, .. } = &mut doc.rows[0].tiles[2].op else {
        panic!()
    };
    *count = 129;
    assert!(doc.evaluate().is_err());
    *doc.rows[0].tiles.last_mut().unwrap() = Tile {
        id: "repeat".into(),
        op: Operation::Repeat {
            count: 128,
            translation: [1., 0., 0.],
            rotation: [0.; 3],
        },
    };
    doc.rows[0].tiles.push(Tile {
        id: "repeat-again".into(),
        op: Operation::Repeat {
            count: 128,
            translation: [0., 0., 1.],
            rotation: [0.; 3],
        },
    });
    assert!(
        doc.evaluate()
            .unwrap_err()
            .to_string()
            .contains("vertex budget")
    );
}

#[test]
fn derived_controls_resolve_forward_dependencies_before_construction_without_rewriting_source() {
    let mut length = control(
        "length",
        1.,
        0.1,
        100.,
        vec![binding("box", "size0", 1., 0.)],
    );
    length.expression = Some("half * 2".into());
    let mut half = control("half", 1., 0.05, 50., vec![]);
    half.expression = Some("count * [post-spacing] / 2".into());
    let doc = wrap(
        base(),
        vec![
            length,
            half,
            control("count", 6., 1., 32., vec![]),
            control("post-spacing", 0.35, 0.1, 2., vec![]),
        ],
    );
    let source = doc.save().unwrap();
    let mesh = doc.evaluate().unwrap().mesh;
    let (lo, hi) = mesh.bounds();
    assert!((hi[0] - lo[0] - 2.1).abs() < 1e-5);
    assert_eq!(doc.save().unwrap(), source);
    let reopened = Document::load(&source).unwrap();
    assert_eq!(reopened.evaluate().unwrap().mesh, mesh);
    let Operation::Group { controls, .. } = &doc.rows[0].tiles[0].op else {
        panic!()
    };
    let resolved = expressions::resolve(controls).unwrap();
    assert!((resolved[0].value - 2.1).abs() < 1e-5);
    // Calculated outputs are read-only interfaces, including for containing groups.
    assert!(
        !parameters::fields(&doc.rows[0].tiles[0].op)
            .iter()
            .any(|f| f.id == "control:length")
    );
    let mut parent = doc.clone();
    assert!(
        parameters::set(
            &mut parent,
            &Target {
                path: vec![],
                operation: "group-op".into(),
                parameter: "control:length".into()
            },
            3.
        )
        .is_err()
    );
}
#[test]
fn control_expressions_validate_math_dependencies_ranges_and_budgets() {
    let expression = |formula: &str| {
        let mut c = control("output", 1., -1000., 1000., vec![]);
        c.expression = Some(formula.into());
        c
    };
    for (formula, expected) in [
        ("1 + 2 * 3 - 4 / 2", 5.),
        ("-(2 + 3) * -2", 10.),
        ("cos_deg(60) * 4", 2.),
        ("sin_deg(30) * 4", 2.),
        ("tan_deg(45)", 1.),
        ("min(5, max(2, 3))", 3.),
        ("clamp(8, 1, 4)", 4.),
        ("floor(2.9) + ceil(2.1) + round(2.6)", 8.),
        ("sqrt(9) + abs(-2)", 5.),
        ("1e-2 * 1E2", 1.),
    ] {
        let controls = vec![expression(formula)];
        assert!(
            (expressions::resolve(&controls).unwrap()[0].value - expected).abs() < 1e-5,
            "{formula}"
        );
    }
    for formula in [
        "output + 1",
        "unknown * 2",
        "1 / 0",
        "sqrt(-1)",
        "tan_deg(90)",
        "cos_deg()",
        "sin(1)",
        "clamp(1, 3, 2)",
        "1; run()",
        "1 +",
        "[]",
        "1e999",
        "1001",
        "1 / 1e-30",
        "",
    ] {
        assert!(
            expressions::resolve(&[expression(formula)]).is_err(),
            "{formula}"
        );
    }
    let mut a = expression("b");
    let mut b = expression("output");
    b.id = "b".into();
    assert!(
        expressions::resolve(&[a.clone(), b])
            .unwrap_err()
            .to_string()
            .contains("Cyclic")
    );
    a.expression = Some("(".repeat(40) + "1" + &")".repeat(40));
    assert!(expressions::resolve(&[a]).is_err());
    assert!(expressions::resolve(&[expression(&"1+".repeat(600))]).is_err());
    assert!(expressions::resolve(&[expression(&("1+".repeat(200) + "1"))]).is_err());
    let mut integer = control("n", 2., 1., 32., vec![binding("unused", "count", 1., 0.)]);
    integer.expression = Some("3 / 2".into());
    assert!(expressions::resolve(&[integer]).is_err());
}
#[test]
fn roof_ridge_is_mitered_and_joined_for_span_pitch_and_thickness_changes() {
    let mut doc = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/gable-roof.mzgeometry.json"
    ))
    .unwrap();
    for span in [1., 5., 12., 20.] {
        for pitch in [0f32, 15., 30., 45., 60., 70.] {
            for thickness in [0.02, 0.12, 0.5] {
                for (id, value) in [("span", span), ("slope", pitch), ("thickness", thickness)] {
                    settings(&mut doc)
                        .iter_mut()
                        .find(|c| c.id == id)
                        .unwrap()
                        .value = value;
                }
                let mesh = doc.evaluate().unwrap().mesh;
                assert_eq!(mesh.parts.len(), 2);
                let rise = span / 2. * pitch.to_radians().tan();
                let top = rise + thickness / pitch.to_radians().cos();
                for part in &mesh.parts {
                    let ridge: Vec<_> = part
                        .vertices
                        .iter()
                        .filter(|v| (v.position[0] - span / 2.).abs() < 2e-4)
                        .collect();
                    // Both roof faces meet on the same vertical seam, at both sides of the extrusion.
                    for y in [rise, top] {
                        for z in [-0.5, 0.5] {
                            assert!(
                                ridge.iter().any(|v| (v.position[1] - y).abs() < 2e-4
                                    && (v.position[2] - z).abs() < 2e-4),
                                "span={span} pitch={pitch} t={thickness} part={} ridge={ridge:?}",
                                part.id
                            );
                        }
                    }
                    assert!(part.surface.is_some());
                    assert!(
                        part.vertices
                            .iter()
                            .all(|v| v.normal.iter().all(|n| n.is_finite()))
                    );
                }
            }
        }
    }
}
#[test]
fn fence_continuous_rails_follow_count_and_spacing_without_stretching_posts() {
    let mut doc = Document::load(include_str!(
        "../../../creator/embedded/meshes/recipes/fence.mzgeometry.json"
    ))
    .unwrap();
    for count in [1., 6., 8., 32.] {
        for spacing in [0.12, 0.35, 0.6, 2.] {
            for (id, value) in [("count", count), ("spacing", spacing)] {
                settings(&mut doc)
                    .iter_mut()
                    .find(|c| c.id == id)
                    .unwrap()
                    .value = value;
            }
            let mesh = doc.evaluate().unwrap().mesh;
            assert_eq!(mesh.parts.len(), count as usize + 2);
            let (lo, hi) = mesh.bounds();
            assert!(lo[0].abs() < 1e-4 && (hi[0] - count * spacing).abs() < 1e-4);
            let rail_parts: Vec<_> = mesh
                .parts
                .iter()
                .filter(|p| p.id.starts_with("rails/"))
                .collect();
            assert_eq!(rail_parts.len(), 2);
            for rail in rail_parts {
                let min = rail
                    .vertices
                    .iter()
                    .map(|v| v.position[0])
                    .fold(f32::INFINITY, f32::min);
                let max = rail
                    .vertices
                    .iter()
                    .map(|v| v.position[0])
                    .fold(f32::NEG_INFINITY, f32::max);
                assert!(min.abs() < 1e-4 && (max - count * spacing).abs() < 1e-4);
            }
            for post in mesh.parts.iter().filter(|p| p.id.starts_with("solid/")) {
                let min = post
                    .vertices
                    .iter()
                    .map(|v| v.position[0])
                    .fold(f32::INFINITY, f32::min);
                let max = post
                    .vertices
                    .iter()
                    .map(|v| v.position[0])
                    .fold(f32::NEG_INFINITY, f32::max);
                assert!((max - min - 0.1).abs() < 1e-4);
            }
        }
    }
}
