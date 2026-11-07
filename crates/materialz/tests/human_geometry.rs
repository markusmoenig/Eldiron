#![cfg(feature = "integration")]
use denrim_materials::geometry::{rig::*, *};
fn human() -> Document {
    Document::load(HUMAN_DETAIL_EXAMPLE).unwrap()
}
fn near(a: [f32; 3], b: [f32; 3]) {
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 0.00002, "{a:?} != {b:?}");
    }
}
#[test]
fn human_is_an_editable_row_document_with_two_equipped_items() {
    let doc = human();
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
    let e = doc.evaluate().unwrap();
    let rig = e.mesh.rig.as_ref().unwrap();
    assert_eq!(rig.joints.len(), 17);
    assert_eq!(e.mesh.attachments.len(), 2);
    assert!(
        doc.rows
            .iter()
            .filter(|r| r
                .tiles
                .iter()
                .any(|t| matches!(t.op, Operation::Loft { .. })))
            .count()
            > 20
    );
    assert!(e.mesh.vertex_count() < 50000);
    assert_eq!(
        e.rows
            .values()
            .filter(|v| matches!(v, Value::Motion(_)))
            .count(),
        3
    );
}
#[test]
fn skin_rest_pose_reconstructs_mesh_and_weights_are_normalized() {
    let e = human().evaluate().unwrap();
    let posed = e.mesh.posed(None, 0.).unwrap();
    let mut blended = 0;
    for (part, out) in e.mesh.parts.iter().zip(&posed.mesh.parts) {
        for (v, p) in part.vertices.iter().zip(&out.vertices) {
            near(v.position, p.position);
            near(v.normal, p.normal);
            let w = v.skin.as_ref().unwrap();
            assert!((w.weights.iter().sum::<f32>() - 1.).abs() < 1e-6);
            if w.weights[1] > 0.01 && w.weights[1] < 0.99 {
                blended += 1;
            }
        }
    }
    assert!(
        blended > 30,
        "The limbs need blended skin, not just rigid disconnected pieces"
    );
}
#[test]
fn sword_grip_and_vertices_follow_the_hand_during_walk_and_attack() {
    let e = human().evaluate().unwrap();
    let attachment = e
        .mesh
        .attachments
        .iter()
        .find(|a| a.id == "held-sword")
        .unwrap();
    let Value::Mesh(source) = &e.rows["sword/sword"] else {
        panic!()
    };
    let original = source.parts[0].vertices[0].position;
    for name in ["walk", "attack"] {
        let Value::Motion(clip) = &e.rows[name] else {
            panic!()
        };
        for phase in [0., 0.18, 0.38, 0.55, 0.72, 1.] {
            let posed = e.mesh.posed(Some(clip), phase * clip.duration).unwrap();
            let grip = posed.joints[attachment.joint].compose(Pose::from_frame(&attachment.socket));
            near(
                grip.translation,
                posed.attachments["held-sword"].translation,
            );
            let expected = grip
                .compose(Pose::from_frame(&attachment.grip).inverse())
                .point(original);
            let actual = posed
                .mesh
                .parts
                .iter()
                .find(|p| p.id == "sword/blade")
                .unwrap()
                .vertices[0]
                .position;
            near(expected, actual);
        }
    }
}
#[test]
fn loop_endpoints_match_and_attack_holds_its_final_pose() {
    let e = human().evaluate().unwrap();
    for name in ["idle", "walk", "attack"] {
        let Value::Motion(clip) = &e.rows[name] else {
            panic!()
        };
        let a = e
            .mesh
            .posed(Some(clip), if clip.looping { 0. } else { clip.duration })
            .unwrap();
        let b = e.mesh.posed(Some(clip), clip.duration * 3.).unwrap();
        for (p, q) in a
            .mesh
            .parts
            .iter()
            .flat_map(|p| &p.vertices)
            .zip(b.mesh.parts.iter().flat_map(|p| &p.vertices))
        {
            near(p.position, q.position);
        }
    }
}
#[test]
fn invalid_rigs_tracks_and_bindings_are_rejected() {
    let mut doc = human();
    let tile = doc
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| matches!(t.op, Operation::Skin { .. }))
        .unwrap();
    if let Operation::Skin { joints, .. } = &mut tile.op {
        *joints = vec!["missing".into()];
    }
    assert!(doc.evaluate().is_err());
    let mut doc = human();
    let tile = doc
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| matches!(t.op, Operation::Motion { .. }))
        .unwrap();
    if let Operation::Motion { tracks, .. } = &mut tile.op {
        tracks[0].keys[1].time = -1.;
    }
    assert!(doc.evaluate().is_err());
    let mut doc = human();
    let tile = doc
        .rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find(|t| {
            matches!(
                t.op,
                Operation::Joint {
                    parent: Some(_),
                    ..
                }
            )
        })
        .unwrap();
    if let Operation::Joint { parent, .. } = &mut tile.op {
        *parent = Some("skeleton".into());
    }
    assert!(doc.evaluate().is_err());
}

#[test]
fn prepared_skin_transforms_match_quaternion_reference_for_clips_and_transitions() {
    let e = human().evaluate().unwrap();
    let rig = e.mesh.rig.as_ref().unwrap();
    let rest = rig.rest();
    let mut cases = Vec::new();
    for name in ["idle", "walk", "attack"] {
        let Value::Motion(motion) = &e.rows[name] else {
            panic!()
        };
        for phase in [0., 0.17, 0.55, 0.91, 1.27] {
            let time = phase * motion.duration;
            cases.push((Some(motion), time, rig.sample(Some(motion), time).unwrap()));
        }
    }
    let Value::Motion(walk) = &e.rows["walk"] else {
        panic!()
    };
    let Value::Motion(attack) = &e.rows["attack"] else {
        panic!()
    };
    let joints = rig
        .blend(
            &rig.sample(Some(walk), 0.2).unwrap(),
            &rig.sample(Some(attack), 0.4).unwrap(),
            0.37,
        )
        .unwrap();
    cases.push((Some(attack), 0.4, joints));
    for (motion, time, joints) in cases {
        let transforms: Vec<_> = joints
            .iter()
            .zip(&rest)
            .map(|(p, b)| p.compose(b.inverse()))
            .collect();
        let posed = e
            .mesh
            .posed_with_joints(motion, time, Some(&joints))
            .unwrap();
        for (input, output) in e.mesh.parts.iter().zip(&posed.mesh.parts) {
            // Keyed hidden parts intentionally retain their semantic slot with no vertices.
            if output.vertices.is_empty() {
                continue;
            }
            assert_eq!(input.vertices.len(), output.vertices.len());
            for (v, actual) in input.vertices.iter().zip(&output.vertices) {
                let (mut position, mut normal) = ([0.; 3], [0.; 3]);
                if let Some(weights) = &v.skin {
                    for i in 0..4 {
                        if weights.weights[i] == 0. {
                            continue;
                        }
                        let transform = transforms[weights.joints[i] as usize];
                        let p = transform.point(v.position);
                        let n = transform.vector(v.normal);
                        for k in 0..3 {
                            position[k] += p[k] * weights.weights[i];
                            normal[k] += n[k] * weights.weights[i];
                        }
                    }
                    let len = normal.iter().map(|n| n * n).sum::<f32>().sqrt().max(1e-10);
                    normal = normal.map(|n| n / len);
                } else {
                    position = v.position;
                    normal = v.normal;
                }
                near(actual.position, position);
                near(actual.normal, normal);
                assert_eq!(actual.uv, v.uv);
            }
        }
        for attachment in &e.mesh.attachments {
            let expected = joints[attachment.joint].compose(Pose::from_frame(&attachment.socket));
            assert_eq!(posed.attachments[&attachment.id], expected);
        }
    }
}
