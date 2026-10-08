#![cfg(feature = "eldiron")]
use character_generator::{eldiron::*, *};
use rusterix::avatar::{
    Avatar, AvatarBuildRequest, AvatarBuilder, AvatarMarkerColors, AvatarShadingOptions,
};
use scenevm::prelude::{Atom, GeoId, SharedAtlas};
const OVERLAY: &str = include_str!("../examples/ruleset-overlay.toml");
fn asset() -> CharacterAsset {
    let resolved =
        eldiron_ruleset::resolve_project_ruleset(eldiron_ruleset::DEFAULT_RULESET_CONFIG, OVERLAY)
            .unwrap();
    assert!(resolved.validation().is_ok(), "{:?}", resolved.validation());
    let catalog = ruleset::catalog_from_resolved(&resolved).unwrap();
    // Actual item templates still expose existing game mechanics.
    let templates = eldiron_ruleset::ruleset_item_templates(resolved.table()).unwrap();
    assert!(templates.iter().any(|t| t.id == "training_sword"));
    catalog.generate("villager").unwrap()
}
#[test]
fn scenevm_accepts_posed_meshes_and_converted_material_tiles() {
    let asset = asset();
    let adapter = SceneVmCharacter::new(&asset).unwrap();
    let atlas = SharedAtlas::new(256, 256);
    adapter.register_materials(&atlas);
    for m in &adapter.materials {
        let (w, h, data) = atlas.get_tile_data(m.id).unwrap();
        assert_eq!((w, h), (m.size, m.size));
        assert_eq!(data, m.rgba);
        assert!(
            m.packed_material
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[1] == 15 && p[0] != 254)
        );
    }
    let mut vm = scenevm::VM::new_with_shared_atlas(atlas);
    let id = GeoId::Character(42);
    let objects = adapter.objects(id, [3.0, 0.0, 4.0], 0.0, Motion::Walk, 0.25);
    assert_eq!(
        objects.iter().map(|o| o.mesh_indices.len()).sum::<usize>(),
        asset
            .primitives
            .iter()
            .map(|p| p.indices.len())
            .sum::<usize>()
    );
    for object in &objects {
        assert_eq!(object.id, id);
        assert_eq!(object.center.x, 3.0);
        assert_eq!(object.center.z, 4.0);
        assert!(
            object
                .mesh_indices
                .iter()
                .all(|i| (*i as usize) < object.mesh_vertices.len())
        );
        vm.execute(Atom::AddDynamic {
            object: object.clone(),
        });
    }
    assert_eq!(vm.debug_stats().dynamics, objects.len());
    vm.execute(Atom::ClearDynamics);
    assert_eq!(vm.debug_stats().dynamics, 0);
    let rotated = adapter.objects(
        id,
        [0.0; 3],
        std::f32::consts::FRAC_PI_2,
        Motion::Walk,
        0.25,
    );
    for (a, b) in objects.iter().zip(&rotated) {
        for (va, vb) in a.mesh_vertices.iter().zip(&b.mesh_vertices) {
            assert!((va.position.z - 4.0 - vb.position.x).abs() < 1e-5);
            assert!((va.position.x - 3.0 + vb.position.z).abs() < 1e-5);
        }
    }
    let mut changed = asset.clone();
    changed.materials[0].color = [0.1, 0.1, 0.1, 1.0];
    let updated = SceneVmCharacter::new(&changed).unwrap();
    assert_ne!(adapter.materials[0].id, updated.materials[0].id);
}
#[test]
fn avatar_roundtrips_through_eldiron_builder_without_duplicate_weapons() {
    let asset = asset();
    let out = std::env::temp_dir().join(format!("eldiron-generated-avatar-{}", std::process::id()));
    let options = BakeOptions {
        size: 48,
        frames: 4,
    };
    let avatar = export_avatar(&asset, options, &out, 4.0).unwrap();
    let loaded: Avatar =
        serde_json::from_slice(&std::fs::read(out.join("character.eldiron_avatar")).unwrap())
            .unwrap();
    assert_eq!(avatar.id, loaded.id);
    assert_eq!(loaded.animations.len(), asset.motions.len());
    let mask = image::open(out.join("channels.png")).unwrap().to_luma8();
    assert!(!mask.pixels().any(|p| p[0] == 10));
    for animation in &loaded.animations {
        let motion = asset
            .motions
            .iter()
            .find(|m| m.name().eq_ignore_ascii_case(&animation.name))
            .unwrap();
        let duration = motion.duration();
        assert_eq!(
            animation.speed,
            duration * 4.0 / if motion.looping() { 4.0 } else { 3.0 }
        );
        assert_eq!(animation.perspectives.len(), 8);
        for perspective in &animation.perspectives {
            assert_eq!(perspective.frames.len(), 4);
            for (index, frame) in perspective.frames.iter().enumerate() {
                assert!(frame.weapon_main_anchor.is_some());
                assert!(frame.weapon_off_anchor.is_some());
                let built = AvatarBuilder::build_current_stub(AvatarBuildRequest {
                    avatar: &loaded,
                    animation_name: Some(&animation.name),
                    direction: perspective.direction,
                    frame_index: index,
                    marker_colors: AvatarMarkerColors::default(),
                    shading: AvatarShadingOptions::default(),
                })
                .unwrap();
                assert_eq!(built.rgba, frame.texture.data);
            }
        }
    }
    let invalid = export_avatar(&asset, options, &out, 0.0);
    assert!(invalid.is_err());
    std::fs::remove_dir_all(out).unwrap();
}
