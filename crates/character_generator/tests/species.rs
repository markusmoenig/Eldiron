use character_generator::*;

#[test]
fn species_have_distinct_anatomy_and_valid_animated_equipment() {
    let source: toml::Table =
        toml::from_str(include_str!("../examples/bench-ruleset.toml")).unwrap();
    let mut catalog = ruleset::catalog_from_table(&source).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        for id in ["human", "orc", "skeleton"] {
            let asset = catalog
                .generate_with_options(
                    id,
                    &GenerationOptions {
                        style,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(asset.joints.len(), 15);
            assert_eq!(asset.sockets.len(), 2);
            if id == "orc" {
                assert_eq!(
                    asset.primitives.iter().filter(|p| p.name == "tusk").count(),
                    2
                );
            }
            if id == "skeleton" {
                assert!(asset.primitives.iter().any(|p| p.name == "cranium"));
                assert!(
                    !asset
                        .primitives
                        .iter()
                        .any(|p| matches!(p.name.as_str(), "hair" | "head" | "eye"))
                );
                assert!(asset.primitives.iter().any(|p| p.name == "helmet"));
            }
            let bind = asset.bind_pose();
            for motion in &asset.motions {
                for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let pose =
                        asset.world_pose(&asset.local_pose(*motion, phase * motion.duration()));
                    for p in &asset.primitives {
                        assert!(p.indices.iter().all(|i| (*i as usize) < p.vertices.len()));
                        for v in &p.vertices {
                            assert!((v.weights.iter().sum::<f32>() - 1.0).abs() < 1e-5);
                            let (position, normal) = asset.posed_vertex(v, &bind, &pose);
                            assert!(position.iter().chain(normal.iter()).all(|x| x.is_finite()));
                            assert!((normal.iter().map(|n| n * n).sum::<f32>() - 1.0).abs() < 1e-4);
                        }
                    }
                }
            }
        }
    }
    let skeleton = catalog.characters.get_mut("skeleton").unwrap();
    skeleton.outfit.clear();
    skeleton.equipment.clear();
    let bare = catalog.generate("skeleton").unwrap();
    for name in [
        "rib",
        "spine",
        "pelvic_arch",
        "humerus",
        "radius",
        "femur",
        "tibia",
        "toe",
    ] {
        assert!(
            bare.primitives.iter().any(|p| p.name == name),
            "missing {name}"
        );
    }
}

#[test]
fn omitted_species_preserves_humans_and_unknown_species_is_rejected() {
    let source = include_str!("../examples/villager.toml");
    assert!(
        Catalog::parse(source)
            .unwrap()
            .characters
            .values()
            .all(|c| c.species == Species::Human)
    );
    let invalid = source.replace(
        "body = \"humanoid\"",
        "body = \"humanoid\"\nspecies = \"unknown\"",
    );
    assert!(Catalog::parse(&invalid).is_err());
}
