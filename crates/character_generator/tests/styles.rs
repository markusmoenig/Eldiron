use character_generator::*;
const SOURCE: &str = include_str!("../examples/villager.toml");
fn count(a: &CharacterAsset) -> usize {
    a.primitives.iter().map(|p| p.indices.len() / 3).sum()
}
#[test]
fn style_is_a_caller_setting_and_keeps_rig_materials_and_attachments_compatible() {
    let catalog = Catalog::parse(SOURCE).unwrap();
    let blocky = catalog.generate("villager").unwrap();
    assert_eq!(blocky.style.preset, StylePreset::Blocky);
    assert_eq!(count(&blocky), 494);
    for preset in [StylePreset::Stylized, StylePreset::Natural] {
        let a = catalog
            .generate_with_options(
                "villager",
                &GenerationOptions {
                    style: preset,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(count(&a) > count(&blocky));
        assert_eq!(
            serde_json::to_vec(&a.joints).unwrap(),
            serde_json::to_vec(&blocky.joints).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&a.materials).unwrap(),
            serde_json::to_vec(&blocky.materials).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&a.sockets).unwrap(),
            serde_json::to_vec(&blocky.sockets).unwrap()
        );
        let bind = a.bind_pose();
        let pose = a.world_pose(&a.local_pose(Motion::Walk, 0.25));
        for p in &a.primitives {
            for v in &p.vertices {
                assert!((v.weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
                assert!(v.uv.iter().all(|x| (0.0..=1.0).contains(x)));
                let (position, n) = a.posed_vertex(v, &bind, &pose);
                assert!(position.iter().all(|x| x.is_finite()));
                assert!(
                    (n.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5,
                    "{} {:?} {:?}",
                    p.name,
                    v.position,
                    v.normal
                );
                let (rest, _) = a.posed_vertex(v, &bind, &bind);
                for (a, b) in rest.iter().zip(v.position) {
                    assert!((a - b).abs() < 1e-5);
                }
            }
            for tri in p.indices.as_chunks::<3>().0 {
                let [a, b, c] = tri.map(|i| p.vertices[i as usize].position);
                let u: [f32; 3] = std::array::from_fn(|i| b[i] - a[i]);
                let v: [f32; 3] = std::array::from_fn(|i| c[i] - a[i]);
                let cross = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                assert!(
                    cross.iter().map(|x| x * x).sum::<f32>() > 1e-12,
                    "degenerate {}",
                    p.name
                );
            }
        }
    }
    // Ruleset/character definitions do not own the creation style.
    assert!(
        Catalog::parse(&SOURCE.replace(
            "body = \"humanoid\"",
            "body = \"humanoid\"\nstyle = \"natural\""
        ))
        .is_err()
    );
}
#[test]
fn detail_shading_and_head_scale_change_geometry_without_changing_game_data() {
    let catalog = Catalog::parse(SOURCE).unwrap();
    let base = GenerationOptions {
        style: StylePreset::Natural,
        ..Default::default()
    };
    let standard = catalog.generate_with_options("villager", &base).unwrap();
    let detailed = catalog
        .generate_with_options(
            "villager",
            &GenerationOptions {
                segments: Some(20),
                ..base.clone()
            },
        )
        .unwrap();
    assert!(count(&detailed) > count(&standard));
    let flat = catalog
        .generate_with_options(
            "villager",
            &GenerationOptions {
                shading: Some(Shading::Flat),
                ..base.clone()
            },
        )
        .unwrap();
    let smooth_head = standard
        .primitives
        .iter()
        .find(|p| p.name == "head")
        .unwrap();
    let flat_head = flat.primitives.iter().find(|p| p.name == "head").unwrap();
    assert_eq!(smooth_head.indices, flat_head.indices);
    assert!(
        smooth_head
            .vertices
            .iter()
            .zip(&flat_head.vertices)
            .any(|(a, b)| a.normal != b.normal)
    );
    let large = catalog
        .generate_with_options(
            "villager",
            &GenerationOptions {
                head_scale: 1.2,
                ..base.clone()
            },
        )
        .unwrap();
    let head = large.primitives.iter().find(|p| p.name == "head").unwrap();
    let width = |p: &Primitive| {
        p.vertices
            .iter()
            .map(|v| v.position[0])
            .fold(f32::NEG_INFINITY, f32::max)
            - p.vertices
                .iter()
                .map(|v| v.position[0])
                .fold(f32::INFINITY, f32::min)
    };
    assert!((width(head) / width(smooth_head) - 1.2).abs() < 1e-5);
    for bad in [
        GenerationOptions {
            segments: Some(3),
            ..base.clone()
        },
        GenerationOptions {
            segments: Some(25),
            ..base.clone()
        },
        GenerationOptions {
            head_scale: f32::NAN,
            ..base.clone()
        },
        GenerationOptions {
            head_scale: 0.2,
            ..base.clone()
        },
        GenerationOptions {
            style: StylePreset::Blocky,
            segments: Some(12),
            ..Default::default()
        },
    ] {
        assert!(catalog.generate_with_options("villager", &bad).is_err());
    }
}

#[test]
fn rounded_body_has_closed_connected_hips_neck_and_shoulders() {
    use std::collections::{BTreeMap, BTreeSet};
    let catalog = Catalog::parse(SOURCE).unwrap();
    for style in [StylePreset::Stylized, StylePreset::Natural] {
        for segments in [4, 8, 12, 15, 24] {
            let asset = catalog
                .generate_with_options(
                    "villager",
                    &GenerationOptions {
                        style,
                        segments: Some(segments),
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(!asset.primitives.iter().any(|p| {
                [
                    "torso",
                    "pelvis",
                    "nose",
                    "left_arm",
                    "right_arm",
                    "left_leg",
                    "right_leg",
                ]
                .contains(&p.name.as_str())
            }));
            let mut body = Primitive {
                name: "body".into(),
                material: 0,
                vertices: vec![],
                indices: vec![],
            };
            for p in asset
                .primitives
                .iter()
                .filter(|p| ["upper_body", "lower_body", "neck", "head"].contains(&p.name.as_str()))
            {
                let base = body.vertices.len() as u32;
                body.vertices.extend(p.vertices.iter().cloned());
                body.indices.extend(p.indices.iter().map(|i| i + base));
            }
            let mut positions = BTreeMap::new();
            let mut remap = Vec::new();
            let mut skin = Vec::new();
            for v in &body.vertices {
                let key = v.position.map(f32::to_bits);
                let next = positions.len();
                let id = *positions.entry(key).or_insert(next);
                if id == skin.len() {
                    skin.push((v.joints, v.weights));
                }
                assert_eq!(skin[id], (v.joints, v.weights), "boundary skinning differs");
                remap.push(id);
            }
            let mut edges = BTreeMap::new();
            let mut neighbors = vec![BTreeSet::new(); positions.len()];
            for tri in body.indices.as_chunks::<3>().0 {
                let ids: Vec<_> = tri.iter().map(|i| remap[*i as usize]).collect();
                for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
                    assert_ne!(a, b);
                    let edge = edges.entry((a.min(b), a.max(b))).or_insert((0, 0));
                    edge.0 += 1;
                    edge.1 += if a < b { 1 } else { -1 };
                    neighbors[a].insert(b);
                    neighbors[b].insert(a);
                }
            }
            assert!(
                edges.values().all(|edge| *edge == (2, 0)),
                "open, internal or incorrectly wound faces"
            );
            let mut visited = BTreeSet::new();
            let mut pending = vec![0];
            while let Some(i) = pending.pop() {
                if visited.insert(i) {
                    pending.extend(neighbors[i].iter().copied());
                }
            }
            assert_eq!(
                visited.len(),
                positions.len(),
                "separate shoulder component"
            );
            let bind = asset.bind_pose();
            for motion in [
                Motion::Idle,
                Motion::Walk,
                Motion::Cast,
                Motion::Attack,
                Motion::Parry,
                Motion::Use,
                Motion::Sit,
                Motion::Death,
            ] {
                for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let pose =
                        asset.world_pose(&asset.local_pose(motion, motion.duration() * fraction));
                    let mut posed = BTreeMap::new();
                    for (i, v) in body.vertices.iter().enumerate() {
                        let (p, n) = asset.posed_vertex(v, &bind, &pose);
                        assert!(p.iter().chain(&n).all(|v| v.is_finite()));
                        if let Some(previous) = posed.insert(remap[i], p) {
                            assert_eq!(previous, p, "animated seam opened");
                        }
                    }
                }
            }
        }
    }
}
