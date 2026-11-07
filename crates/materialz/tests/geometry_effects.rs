#![cfg(feature = "integration")]
use denrim_materials::geometry::{effects::ParticleEffect, *};
#[test]
fn effects_follow_transform_attachment_pose_and_roundtrip() {
    let mut doc = Document::load(SWORD_EXAMPLE).unwrap();
    let output = doc.output.clone();
    doc.rows
        .iter_mut()
        .find(|r| r.id == output)
        .unwrap()
        .tiles
        .push(Tile {
            id: "flame_tip".into(),
            op: Operation::ParticleEffect {
                effect: ParticleEffect {
                    position: [0., 0.5, 0.],
                    ..Default::default()
                },
            },
        });
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
    let mesh = doc.evaluate().unwrap().mesh;
    assert_eq!(mesh.effects.len(), 1);
    let mut mesh = mesh.clone();
    mesh.transform([1., 0., 0.], [0., 0., 90.], [2.; 3]);
    let p = mesh.effects[0].definition.position;
    assert!(p[0].abs() < 1e-5 && p[1].abs() < 1e-5);
    let rig = rig::Rig {
        joints: vec![rig::Joint {
            id: "hand".into(),
            label: "Hand".into(),
            parent: None,
            local: rig::Pose::identity(),
        }],
        tip: Some("hand".into()),
    };
    rig::attach(
        &mut mesh,
        &rig,
        "hand",
        &Frame {
            translation: [0.; 3],
            rotation: [0.; 3],
        },
        Frame {
            translation: [0., 1., 0.],
            rotation: [0.; 3],
        },
        "held",
    )
    .unwrap();
    assert_eq!(mesh.effects[0].id, "held/flame_tip");
    assert_eq!(mesh.effects[0].joint, Some(0));
    let mut motion = rig::Motion {
        events: vec![],
        rig,
        duration: 1.,
        looping: false,
        tracks: vec![rig::Track {
            joint: "hand".into(),
            keys: vec![
                rig::Key {
                    time: 0.,
                    translation: [0.; 3],
                    rotation: [0.; 3],
                    visible: true,
                },
                rig::Key {
                    time: 1.,
                    translation: [1., 0., 0.],
                    rotation: [0.; 3],
                    visible: true,
                },
            ],
        }],
    };
    let posed = mesh.posed(Some(&motion), 1.).unwrap();
    assert!((posed.mesh.effects[0].definition.position[0] - 1.).abs() < 1e-5);
    assert_eq!(posed.mesh.effects[0].joint, None);
    for time in [0., 0.25, 0.8, 1., 2.] {
        let joints = motion.rig.sample(Some(&motion), time).unwrap();
        let expected = mesh
            .posed_with_joints(Some(&motion), time, Some(&joints))
            .unwrap()
            .mesh
            .effects;
        assert_eq!(
            mesh.posed_effects_with_joints(Some(&motion), time, Some(&joints))
                .unwrap(),
            expected
        );
        assert_eq!(
            mesh.posed_effects_with_joints(Some(&motion), time, None)
                .unwrap(),
            expected
        );
    }
    // A keyed hidden effect follows the parent exactly, including transition poses.
    motion.tracks[0].keys[1].visible = false;
    let joints = motion.rig.sample(Some(&motion), 1.).unwrap();
    assert!(
        mesh.posed_effects_with_joints(Some(&motion), 1., Some(&joints))
            .unwrap()
            .is_empty()
    );
    assert!(
        mesh.posed_effects_with_joints(Some(&motion), 0., Some(&[]))
            .is_err()
    );
    let mut invalid = joints;
    invalid[0].translation[0] = f32::NAN;
    assert!(
        mesh.posed_effects_with_joints(Some(&motion), 0., Some(&invalid))
            .is_err()
    );
}
#[test]
fn invalid_particle_parameters_are_rejected() {
    let mut effect = ParticleEffect::default();
    effect.palette[0] = 256;
    assert!(effect.validate().is_err());
    effect.palette[0] = 31;
    effect.rate = f32::NAN;
    assert!(effect.validate().is_err());
    effect.rate = 10000.;
    assert!(effect.validate().is_err());
}

#[test]
fn motion_events_handle_skipped_frames_loop_boundaries_and_completion() {
    let mut motion = rig::Motion {
        rig: rig::Rig {
            joints: vec![],
            tip: None,
        },
        duration: 2.,
        looping: true,
        tracks: vec![],
        events: vec![
            rig::Event {
                time: 0.,
                name: "start".into(),
            },
            rig::Event {
                time: 0.5,
                name: "step".into(),
            },
        ],
    };
    assert_eq!(
        motion
            .events_between(-f32::EPSILON, 0.)
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["start"]
    );
    assert!(motion.events_between(0., 0.).is_empty());
    assert_eq!(
        motion
            .events_between(0.9, 2.1)
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["step", "start"]
    );
    motion.looping = false;
    assert_eq!(motion.events_between(0., 10.).len(), 1);
    assert!(motion.events_between(10., 11.).is_empty());
    motion.events[0].time = f32::NAN;
    assert!(motion.validate().is_err());
}

#[test]
fn joint_transitions_preserve_bone_length_and_exact_endpoints() {
    let rig = rig::Rig {
        joints: vec![
            rig::Joint {
                id: "root".into(),
                label: "Root".into(),
                parent: None,
                local: rig::Pose::identity(),
            },
            rig::Joint {
                id: "tip".into(),
                label: "Tip".into(),
                parent: Some(0),
                local: rig::Pose::from_frame(&Frame {
                    translation: [0., 1., 0.],
                    rotation: [0.; 3],
                }),
            },
        ],
        tip: None,
    };
    let from = rig.rest();
    let root = rig::Pose::from_frame(&Frame {
        translation: [0.; 3],
        rotation: [0., 0., 90.],
    });
    let to = vec![root, root.compose(rig.joints[1].local)];
    assert_eq!(rig.blend(&from, &to, 0.).unwrap(), from);
    assert_eq!(rig.blend(&from, &to, 1.).unwrap(), to);
    let mid = rig.blend(&from, &to, 0.5).unwrap();
    let distance = (0..3)
        .map(|i| (mid[1].translation[i] - mid[0].translation[i]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!((distance - 1.).abs() < 0.0001);
    assert!(rig.blend(&from, &to, f32::NAN).is_err());
}
