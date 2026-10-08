use character_generator::*;
const SOURCE: &str = include_str!("../examples/guard.toml");

#[test]
fn knees_fold_backwards_on_the_recovery_leg() {
    let a = Catalog::parse(SOURCE).unwrap().generate("guard").unwrap();
    for (time, recovery, support) in [
        (0.25, "left_shin", "right_shin"),
        (0.75, "right_shin", "left_shin"),
    ] {
        let pose = a.local_pose(Motion::Walk, time);
        let index = |name| a.joints.iter().position(|j| j.name == name).unwrap();
        // +X rotation folds a downward shin toward -Z (backwards), never forwards.
        assert!(pose[index(recovery)].rotation[0] > 0.30);
        assert!(pose[index(support)].rotation[0] >= 0.0);
        assert!(pose[index(recovery)].rotation[0] > pose[index(support)].rotation[0] + 0.15);
    }
    for frame in 0..64 {
        let pose = a.local_pose(Motion::Walk, frame as f32 / 64.0);
        for (j, t) in a.joints.iter().zip(pose) {
            if j.name.ends_with("shin") {
                assert!(t.rotation[0] >= 0.0);
            }
        }
    }
}

#[test]
fn new_equipment_fits_and_tracks_its_attachment_across_styles() {
    let c = Catalog::parse(SOURCE).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        for id in ["guard", "spike_guard"] {
            let a = c
                .generate_with_options(
                    id,
                    &GenerationOptions {
                        style,
                        head_scale: 1.2,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(!a.primitives.iter().any(|p| p.name == "hair"));
            let bind = a.bind_pose();
            let pose = a.world_pose(&a.local_pose(Motion::Walk, 0.25));
            for (name, joint) in [
                ("helmet", "head"),
                ("round_shield", "left_hand"),
                (
                    if id == "guard" {
                        "iron_sword_blade"
                    } else {
                        "iron_spike"
                    },
                    "right_hand",
                ),
            ] {
                let p = a.primitives.iter().find(|p| p.name == name).unwrap();
                let j = a.joints.iter().position(|j| j.name == joint).unwrap();
                for v in &p.vertices {
                    assert_eq!(v.joints[0] as usize, j);
                    assert_eq!(v.weights, [1.0, 0.0, 0.0, 0.0]);
                    assert!(v.uv.iter().all(|u| (0.0..=1.0).contains(u)));
                    let (rest, _) = a.posed_vertex(v, &bind, &bind);
                    for (a, b) in rest.iter().zip(v.position) {
                        assert!((a - b).abs() < 1e-5);
                    }
                    let (pos, n) = a.posed_vertex(v, &bind, &pose);
                    assert!(pos.iter().all(|x| x.is_finite()));
                    assert!((n.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
                }
            }
            let head = a.primitives.iter().find(|p| p.name == "head").unwrap();
            let helmet = a.primitives.iter().find(|p| p.name == "helmet").unwrap();
            for axis in 0..3 {
                let bounds = |p: &Primitive| {
                    p.vertices
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
                            (lo.min(v.position[axis]), hi.max(v.position[axis]))
                        })
                };
                let (hl, hh) = bounds(head);
                let (lo, hi) = bounds(helmet);
                assert!(hi > hh);
                if axis != 1 {
                    assert!(lo < hl);
                }
            }
        }
    }
    assert!(
        Catalog::parse(&SOURCE.replace("attachment = \"off_hand\"", "attachment = \"main_hand\""))
            .is_err()
    );
    assert!(Catalog::parse(&SOURCE.replace("[0.42, 0.55, 0.055]", "[0.0, 0.55, 0.055]")).is_err());
    assert!(
        Catalog::parse(&SOURCE.replace(
            "generator = \"helmet\"",
            "generator = \"helmet\"\nattachment = \"off_hand\""
        ))
        .is_err()
    );
    assert!(Catalog::parse(&SOURCE.replace("attachment = \"off_hand\"", "")).is_err());
}

#[test]
fn helmet_brim_clears_the_face_for_every_style_and_head_scale() {
    let catalog = Catalog::parse(SOURCE).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        for head_scale in [0.75, 1.0, 1.35] {
            let a = catalog
                .generate_with_options(
                    "guard",
                    &GenerationOptions {
                        style,
                        head_scale,
                        ..Default::default()
                    },
                )
                .unwrap();
            let brim = a
                .primitives
                .iter()
                .find(|p| p.name == "helmet")
                .unwrap()
                .vertices
                .iter()
                .map(|v| v.position[1])
                .fold(f32::INFINITY, f32::min);
            for name in ["eye", "eyebrow", "mouth"] {
                let marks: Vec<_> = a.primitives.iter().filter(|p| p.name == name).collect();
                assert_eq!(marks.len(), if name == "mouth" { 1 } else { 2 });
                for mark in marks {
                    for v in &mark.vertices {
                        assert!(
                            v.position[1] < brim - a.height * 0.003,
                            "{style:?} {head_scale}: {name} intersects helmet brim"
                        );
                        assert!(v.normal[2] > 0.0, "face should look forward");
                    }
                }
            }
        }
    }
}

#[test]
fn faces_are_visible_below_helmets_in_default_front_sprites() {
    let catalog = Catalog::parse(SOURCE).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        let a = catalog
            .generate_with_options(
                "guard",
                &GenerationOptions {
                    style,
                    ..Default::default()
                },
            )
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "eldiron-face-test-{}-{style:?}",
            std::process::id()
        ));
        let meta = bake_atlas(
            &a,
            BakeOptions {
                size: 96,
                frames: 8,
            },
            &dir,
        )
        .unwrap();
        let mask = image::open(dir.join("channels.png")).unwrap().to_luma8();
        for frame in meta
            .frames
            .iter()
            .filter(|f| f.direction == "front" && f.motion == "idle")
        {
            let [ox, oy, w, h] = frame.rect;
            for channel in [7, 8] {
                assert!(
                    (0..w).any(|x| (0..h).any(|y| mask.get_pixel(ox + x, oy + y)[0] == channel)),
                    "{style:?} frame {} missing face channel {channel}",
                    frame.frame
                );
            }
        }
        for frame in meta
            .frames
            .iter()
            .filter(|f| f.direction == "back" && f.motion == "idle")
        {
            let [ox, oy, w, h] = frame.rect;
            assert!(
                !(0..w).any(|x| (0..h).any(|y| mask.get_pixel(ox + x, oy + y)[0] == 8)),
                "{style:?}: facial marks bleed through the back of the head"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
