use character_generator::*;
#[test]
fn head_traits_change_geometry_but_keep_faces_headwear_and_rig_valid() {
    let source = include_str!("../examples/heads.toml");
    let catalog = Catalog::parse(source).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        let assets: Vec<_> = ["round", "angular", "broad"]
            .iter()
            .map(|id| {
                catalog
                    .generate_with_options(
                        id,
                        &GenerationOptions {
                            style,
                            ..Default::default()
                        },
                    )
                    .unwrap()
            })
            .collect();
        assert_ne!(
            serde_json::to_vec(&assets[0].primitives).unwrap(),
            serde_json::to_vec(&assets[1].primitives).unwrap()
        );
        for a in &assets {
            assert_eq!(
                serde_json::to_vec(&a.joints).unwrap(),
                serde_json::to_vec(&assets[0].joints).unwrap()
            );
            for ear in ["left_ear", "right_ear"] {
                assert!(a.primitives.iter().any(|p| p.name == ear));
            }
            for p in &a.primitives {
                for v in &p.vertices {
                    assert!(v.position.iter().all(|x| x.is_finite()));
                    assert!((v.normal.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
                }
            }
        }
    }
    let mut guards = Catalog::parse(include_str!("../examples/guard.toml")).unwrap();
    for (jaw, cheekbones) in [(0.7, 0.8), (0.7, 1.2), (1.3, 0.8), (1.3, 1.2)] {
        guards.characters.get_mut("guard").unwrap().head = HeadDefinition {
            jaw,
            cheekbones,
            eye_spacing: 1.25,
            nose_length: 1.4,
            nose_width: 1.4,
            ear_size: 1.4,
            ..Default::default()
        };
        for style in [
            StylePreset::Blocky,
            StylePreset::Stylized,
            StylePreset::Natural,
        ] {
            let a = guards
                .generate_with_options(
                    "guard",
                    &GenerationOptions {
                        style,
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
            assert!(
                a.primitives
                    .iter()
                    .filter(|p| p.name == "eyebrow")
                    .flat_map(|p| &p.vertices)
                    .all(|v| v.position[1] < brim)
            );
        }
    }
    for change in ["jaw = 0.0", "jaw = nan"] {
        assert!(Catalog::parse(&source.replace("jaw = 1.2", change)).is_err());
    }
    assert!(Catalog::parse(&source.replace("#33443f", "bad")).is_err());
}
#[test]
fn viewer_contains_mesh_and_all_clips_without_external_dependencies() {
    let a = Catalog::parse(include_str!("../examples/heads.toml"))
        .unwrap()
        .generate("round")
        .unwrap();
    let path =
        std::env::temp_dir().join(format!("eldiron-viewer-test-{}.html", std::process::id()));
    export_viewer(&a, &path).unwrap();
    let html = std::fs::read_to_string(&path).unwrap();
    assert!(!html.contains("__MODEL__"));
    assert!(!html.contains("src=\"http"));
    let payload = html
        .split("const data=")
        .nth(1)
        .unwrap()
        .split(", asset=data.asset")
        .next()
        .unwrap();
    let data: serde_json::Value = serde_json::from_str(payload).unwrap();
    assert_eq!(
        data["asset"]["joints"].as_array().unwrap().len(),
        a.joints.len()
    );
    for motion in &a.motions {
        let clip = &data["clips"][motion.name()];
        assert_eq!(clip["looping"], motion.looping());
        assert_eq!(clip["samples"].as_array().unwrap().len(), 33);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rounded_nose_is_a_small_deformation_of_the_face_surface() {
    let mut catalog = Catalog::parse(include_str!("../examples/heads.toml")).unwrap();
    let options = GenerationOptions {
        style: StylePreset::Natural,
        ..Default::default()
    };
    catalog
        .characters
        .get_mut("angular")
        .unwrap()
        .head
        .nose_length = 0.6;
    let short = catalog.generate_with_options("angular", &options).unwrap();
    catalog
        .characters
        .get_mut("angular")
        .unwrap()
        .head
        .nose_length = 1.4;
    let long = catalog.generate_with_options("angular", &options).unwrap();
    assert!(!long.primitives.iter().any(|p| p.name == "nose"));
    let head = |a: &CharacterAsset| {
        a.primitives
            .iter()
            .find(|p| p.name == "head")
            .unwrap()
            .vertices
            .clone()
    };
    let mut changed = false;
    for (a, b) in head(&short).iter().zip(head(&long)) {
        assert_eq!(a.position[..2], b.position[..2]);
        let delta = b.position[2] - a.position[2];
        assert!(delta >= -1e-6 && delta <= long.height * 0.009);
        changed |= delta > long.height * 0.002;
    }
    assert!(changed);
}
