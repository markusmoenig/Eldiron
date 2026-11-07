#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
fn garment() -> (Document, Document) {
    let wearer = Document::load(HUMAN_EXAMPLE)
        .unwrap()
        .wearer_body()
        .unwrap();
    let doc = Document {
        version: 1,
        id: "test-garment".into(),
        label: "Garment".into(),
        rows: vec![
            Row {
                id: "wearer".into(),
                label: "Wearer".into(),
                tiles: vec![Tile {
                    id: "body-input".into(),
                    op: Operation::Asset {
                        source: WEARER_SOURCE.into(),
                        document: Box::new(wearer.clone()),
                    },
                }],
            },
            Row {
                id: "shell".into(),
                label: "Shell".into(),
                tiles: vec![
                    Tile {
                        id: "ref".into(),
                        op: Operation::Reference {
                            row: "wearer".into(),
                        },
                    },
                    Tile {
                        id: "parts".into(),
                        op: Operation::SelectParts {
                            parts: vec!["body-torso".into()],
                        },
                    },
                    Tile {
                        id: "offset".into(),
                        op: Operation::SurfaceOffset { distance: 0.012 },
                    },
                ],
            },
        ],
        output: "shell".into(),
        grip: None,
    };
    (doc, wearer)
}
#[test]
fn fitted_shell_reuses_the_current_body_without_changing_uvs_weights_or_sockets() {
    let (doc, mut wearer) = garment();
    let Operation::Group { controls, .. } = &mut wearer.rows[0].tiles[0].op else {
        panic!()
    };
    for c in controls {
        if c.id == "height" {
            c.value = c.min;
        }
        if c.id == "shoulder-width" {
            c.value = c.max;
        }
    }
    let original = wearer.evaluate().unwrap().mesh;
    let fitted = doc.with_wearer(&wearer).evaluate().unwrap().mesh;
    assert_eq!(fitted.parts.len(), 1);
    assert_eq!(fitted.rig, original.rig);
    let body = original
        .parts
        .iter()
        .find(|p| p.id == "body-torso")
        .unwrap();
    for (a, b) in fitted.parts[0].vertices.iter().zip(&body.vertices) {
        assert_eq!(a.uv, b.uv);
        assert_eq!(a.skin, b.skin);
        for k in 0..3 {
            assert!((a.position[k] - b.position[k] - b.normal[k] * 0.012).abs() < 0.00001);
        }
    }
    let mut character = Document::load(HUMAN_EXAMPLE)
        .unwrap()
        .evaluate()
        .unwrap()
        .mesh;
    let attachments = character.attachments.clone();
    character
        .replace_parts(doc.evaluate().unwrap().mesh)
        .unwrap();
    assert_eq!(character.attachments, attachments);
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
}
#[test]
fn invalid_mesh_selection_offsets_and_skeletons_are_rejected() {
    let (mut doc, _) = garment();
    let Operation::SelectParts { parts } = &mut doc.rows[1].tiles[1].op else {
        panic!()
    };
    parts.push("absent".into());
    assert!(doc.evaluate().is_err());
    let (mut doc, wearer) = garment();
    doc.rows[1].tiles[2].op = Operation::SurfaceOffset { distance: 0.2 };
    assert!(doc.evaluate().is_err());
    let mut body = wearer.evaluate().unwrap().mesh;
    let before = body.clone();
    let mut incompatible = body.clone();
    incompatible.rig.as_mut().unwrap().joints[0].id = "other-root".into();
    assert!(body.replace_parts(incompatible).is_err());
    assert_eq!(body, before);
}
