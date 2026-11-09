#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
fn human() -> Document {
    Document::load(HUMAN_EXAMPLE).unwrap()
}
fn set(doc: &mut Document, id: &str, value: f32) {
    let Operation::Group { controls, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    controls.iter_mut().find(|c| c.id == id).unwrap().value = value;
}
fn near(a: [f32; 3], b: [f32; 3]) {
    for k in 0..3 {
        assert!((a[k] - b[k]).abs() < 0.00005, "{a:?} != {b:?}");
    }
}
#[test]
fn compact_human_preserves_the_body_and_uses_editable_textured_equipment() {
    let doc = human();
    assert_eq!(doc.rows.len(), 7);
    assert_eq!(doc.rows.iter().map(|r| r.tiles.len()).sum::<usize>(), 9);
    let e = doc.evaluate().unwrap();
    let old = Document::load(HUMAN_DETAIL_EXAMPLE)
        .unwrap()
        .evaluate()
        .unwrap();
    assert!(
        e.mesh.vertex_count() < 32768,
        "Curved equipment stays within the mesh budget"
    );
    assert!(
        e.mesh
            .parts
            .iter()
            .any(|p| p.id == "shield-face" && p.texture.is_some())
    );
    assert_eq!(e.mesh.rig.as_ref().unwrap().joints.len(), 17);
    for part in e
        .mesh
        .parts
        .iter()
        .filter(|p| !p.id.starts_with("shield") && !p.id.starts_with("sword/"))
    {
        let old = old.mesh.parts.iter().find(|p| p.id == part.id).unwrap();
        assert_eq!(part.id, old.id);
        assert_eq!(part.surface, old.surface);
        for (a, b) in part.vertices.iter().zip(&old.vertices) {
            near(a.position, b.position);
            near(a.normal, b.normal);
        }
    }
    let Value::Mesh(sword) = &e.operations["sword-op-0"] else {
        panic!()
    };
    let (lo, hi) = sword.bounds();
    assert!(
        (0.8..0.95).contains(&(hi[1] - lo[1])),
        "The starter sword should use the shortened proportions"
    );
    let details = doc.group_document(&["body-op-0".into()]).unwrap();
    assert_eq!(details.rows.len(), 2);
    let Operation::Profiles { parts, .. } = &details.rows[1].tiles[0].op else {
        panic!()
    };
    assert_eq!(parts.len(), 29);
    assert!(details.rows.iter().all(|r| r.tiles.len() == 1));
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
}
#[test]
fn body_controls_resize_mesh_and_skeleton_and_keep_height_exact() {
    let base = human();
    let before = base.evaluate().unwrap();
    let Operation::Group { controls, .. } = &base.rows[0].tiles[0].op else {
        panic!()
    };
    for c in controls.iter().filter(|c| !c.is_palette()) {
        for value in [c.min, c.max] {
            let mut doc = base.clone();
            set(&mut doc, &c.id, value);
            let e = doc.evaluate().unwrap();
            let Value::Mesh(body) = &e.rows["body"] else {
                panic!()
            };
            let (lo, hi) = body.bounds();
            let target = if c.id == "height" { value } else { 1.8 };
            assert!(
                (hi[1] - lo[1] - target).abs() < 0.0001,
                "{} at {}",
                c.id,
                value
            );
            let rest = e.mesh.posed(None, 0.).unwrap();
            for (a, b) in e
                .mesh
                .parts
                .iter()
                .flat_map(|p| &p.vertices)
                .zip(rest.mesh.parts.iter().flat_map(|p| &p.vertices))
            {
                near(a.position, b.position);
            }
            if c.id != "chest-depth" {
                assert_ne!(
                    e.mesh.rig, before.mesh.rig,
                    "{} must update the rest skeleton too",
                    c.id
                );
            }
        }
    }
}
#[test]
fn equipped_items_follow_resized_hands_in_all_clips() {
    let mut doc = human();
    for (id, value) in [
        ("height", 2.1),
        ("arm-length", 120.),
        ("leg-length", 115.),
        ("torso-length", 85.),
        ("shoulder-width", 125.),
        ("head-size", 112.),
    ] {
        set(&mut doc, id, value);
    }
    let e = doc.evaluate().unwrap();
    for name in ["idle", "walk", "attack"] {
        let Value::Motion(m) = &e.rows[name] else {
            panic!()
        };
        for phase in [0., 0.18, 0.38, 0.55, 0.72, 1.] {
            let posed = e.mesh.posed(Some(m), m.duration * phase).unwrap();
            for a in &e.mesh.attachments {
                let source = if a.id == "sword" {
                    &e.operations["sword-op-0"]
                } else {
                    &e.operations["shield-op-0"]
                };
                let Value::Mesh(source) = source else {
                    panic!()
                };
                let frame = posed.joints[a.joint].compose(rig::Pose::from_frame(&a.socket));
                near(frame.translation, posed.attachments[&a.id].translation);
                let expected = frame
                    .compose(rig::Pose::from_frame(&a.grip).inverse())
                    .point(source.parts[0].vertices[0].position);
                let actual = posed
                    .mesh
                    .parts
                    .iter()
                    .find(|p| p.id == source.parts[0].id)
                    .unwrap()
                    .vertices[0]
                    .position;
                near(expected, actual);
            }
        }
    }
}
#[test]
fn group_colors_are_palette_indices_and_details_remain_independently_editable() {
    let mut doc = human();
    set(&mut doc, "shirt-color", 41.);
    let details = doc.group_document_mut(&["body-op-0".into()]).unwrap();
    let Operation::Profiles { parts, .. } = &mut details.rows[1].tiles[0].op else {
        panic!()
    };
    parts.iter_mut().find(|p| p.id == "nose").unwrap().sections[1].radius[1] *= 1.15;
    let e = doc.evaluate().unwrap();
    assert_eq!(
        e.mesh
            .parts
            .iter()
            .find(|p| p.id == "body-torso")
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .palette,
        41
    );
    let mut bad = doc.clone();
    set(&mut bad, "shirt-color", 3.5);
    assert!(bad.evaluate().is_err());
    let saved = doc.save().unwrap();
    assert_eq!(Document::load(&saved).unwrap(), doc);
}
#[test]
fn invalid_group_definitions_are_rejected() {
    let mut doc = human();
    let Operation::Group { controls, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    controls[0].id = controls[1].id.clone();
    assert!(doc.evaluate().is_err());
    let mut doc = human();
    set(&mut doc, "height", 0.);
    assert!(doc.evaluate().is_err());
    let mut doc = human();
    let Operation::Group { controls, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    let group::Modifier::Affine { parts, .. } = &mut controls[0].modifiers[0] else {
        panic!()
    };
    parts.push("missing".into());
    assert!(doc.evaluate().is_err());
    let mut doc = Document::load(SWORD_GROUP_EXAMPLE).unwrap();
    for _ in 0..7 {
        doc = Document {
            version: 1,
            id: "nested".into(),
            label: "Nested".into(),
            rows: vec![Row {
                id: "item".into(),
                label: "Item".into(),
                tiles: vec![Tile {
                    id: "group".into(),
                    op: Operation::Group {
                        label: "Group".into(),
                        document: Box::new(doc),
                        controls: vec![],
                    },
                }],
            }],
            output: "item".into(),
            grip: None,
        };
    }
    assert!(doc.evaluate().is_err());
}

#[test]
fn sword_handle_control_updates_the_equipment_grip() {
    let mut doc = human();
    let Operation::Group { controls, .. } = &mut doc.rows[1].tiles[0].op else {
        panic!()
    };
    controls
        .iter_mut()
        .find(|c| c.id == "handle-length")
        .unwrap()
        .value = 0.30;
    let e = doc.evaluate().unwrap();
    let attachment = e.mesh.attachments.iter().find(|a| a.id == "sword").unwrap();
    assert!((attachment.grip.translation[1] + 0.175).abs() < 0.00001);
    let Value::Mesh(source) = &e.operations["sword-op-0"] else {
        panic!()
    };
    let Value::Motion(m) = &e.rows["attack"] else {
        panic!()
    };
    let posed = e.mesh.posed(Some(m), 0.38 * m.duration).unwrap();
    let expected = posed.joints[attachment.joint]
        .compose(rig::Pose::from_frame(&attachment.socket))
        .compose(rig::Pose::from_frame(&attachment.grip).inverse())
        .point(source.parts[0].vertices[0].position);
    near(
        expected,
        posed
            .mesh
            .parts
            .iter()
            .find(|p| p.id == source.parts[0].id)
            .unwrap()
            .vertices[0]
            .position,
    );
}

#[test]
fn independent_dimension_controls_preserve_other_axes_and_legacy_uniform_fit() {
    use group::{Control, Modifier};
    let doc = Document::load(r#"{"version":1,"id":"box","label":"Box","rows":[{"id":"box","label":"Box","tiles":[{"id":"box-0","op":{"type":"box","size":[1,2,3]}}]}],"output":"box"}"#).unwrap();
    let original = doc.evaluate().unwrap().mesh;
    for axis in 0..3 {
        let mut mesh = original.clone();
        let (lo, _) = mesh.bounds();
        let control = Control {
            expression: None,
            id: "dimension".into(),
            label: "Size".into(),
            category: "Dimensions".into(),
            help: String::new(),
            order: 0,
            value: (axis + 1) as f32 * 2.,
            reference: (axis + 1) as f32,
            min: 0.01,
            max: 100.,
            modifiers: vec![Modifier::FitAxis { axis }],
        };
        group::apply(&mut mesh, &[control.clone()]).unwrap();
        let (next_lo, hi) = mesh.bounds();
        near(next_lo, lo);
        let expected = std::array::from_fn(|k| (k + 1) as f32 * if k == axis { 2. } else { 1. });
        near(std::array::from_fn(|k| hi[k] - next_lo[k]), expected);
        let mut uniform = original.clone();
        group::apply(
            &mut uniform,
            &[Control {
                modifiers: vec![Modifier::FitSize { axis }],
                ..control
            }],
        )
        .unwrap();
        let (lo, hi) = uniform.bounds();
        near(std::array::from_fn(|k| hi[k] - lo[k]), [2., 4., 6.]);
    }
}
