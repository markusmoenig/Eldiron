//! One editable MZ source supplies both skinned meshes and directional sprites.
use denrim_materials::geometry::{Document, Value as GeoValue};
use eldiron_ruleset as rules;
use rusterix::{Assets, Entity, procedural_avatar::Appearance};

const ANIMALS: [&str; 7] = ["deer", "pig", "cow", "sheep", "goat", "horse", "chicken"];

#[test]
fn fauna_meshes_and_motion_render_from_the_same_mz_source() {
    let definitions = rules::bundled_appearance_assets_for_ruleset(
        rules::OFFICIAL_RULESET_ID,
        rules::OFFICIAL_RULESET_VERSION,
    );
    let mut assets = Assets::default();
    assets.palette = rules::palette_template_from_source(rules::latest_official_ruleset()).unwrap();
    let mut overview = image::RgbaImage::new(8 * 128, ANIMALS.len() as u32 * 128);
    for (species, name) in ANIMALS.iter().enumerate() {
        let asset = definitions.iter().find(|a| a.id == *name).unwrap();
        assert_eq!(asset.asset_id, rules::geometry_asset_id(name).to_string());
        let doc = Document::load(asset.source).unwrap();
        let evaluated = doc.evaluate().unwrap();
        let mesh = &evaluated.mesh;
        assert!(mesh.parts.len() >= 20, "{name}: authored anatomy");
        assert_eq!(
            mesh,
            &Document::load(&serde_json::to_string(&doc).unwrap())
                .unwrap()
                .evaluate()
                .unwrap()
                .mesh
        );
        let rig = mesh.rig.as_ref().unwrap();
        assert!(rig.joints.len() >= 10);
        assert!(
            mesh.parts
                .iter()
                .flat_map(|p| &p.vertices)
                .all(|v| v.skin.is_some())
        );
        for clip in ["idle", "walk", "run", "graze", "attack", "dead"] {
            let GeoValue::Motion(motion) = &evaluated.rows[clip] else {
                panic!("{name}/{clip}")
            };
            let start = mesh.posed(Some(motion), 0.).unwrap();
            let sample = mesh.posed(Some(motion), motion.duration * 0.45).unwrap();
            assert_ne!(
                start.mesh, sample.mesh,
                "{name}/{clip} must deform the mesh"
            );
            if clip == "dead" {
                let end = mesh.posed(Some(motion), motion.duration).unwrap();
                assert!(
                    end.mesh.bounds().0[1] >= -0.03,
                    "{name}: death pose intersects the ground"
                );
            }
            assert!(
                sample
                    .mesh
                    .parts
                    .iter()
                    .flat_map(|p| &p.vertices)
                    .all(|v| v.position.into_iter().all(f32::is_finite))
            );
        }
        let appearance = Appearance::new(asset.source, &[], 1).unwrap();
        let mut actor = Entity::default();
        for view in 0..8 {
            let radians = (view as f32 * 45.).to_radians();
            actor.orientation = vek::Vec2::new(radians.sin(), radians.cos());
            let rgba = appearance
                .sprite_isometric(&actor, &assets, "walk", 0.25, 0., None)
                .unwrap();
            assert!(
                rgba.chunks_exact(4).filter(|p| p[3] > 0).count() > 70,
                "{name}: empty view {view}"
            );
            // A wide animal cannot be clipped to the old humanoid height frame.
            for edge in 0..128 {
                assert_eq!(
                    rgba[(edge * 128) * 4 + 3],
                    0,
                    "{name}: left edge, view {view}"
                );
                assert_eq!(
                    rgba[(edge * 128 + 127) * 4 + 3],
                    0,
                    "{name}: right edge, view {view}"
                );
            }
            let frame = image::RgbaImage::from_raw(128, 128, rgba.as_ref().clone()).unwrap();
            let large =
                image::imageops::resize(&frame, 128, 128, image::imageops::FilterType::Nearest);
            image::imageops::overlay(&mut overview, &large, view * 128, species as i64 * 128);
        }
        assert_ne!(
            appearance.sprite(&actor, &assets, "walk", 0.).unwrap(),
            appearance.sprite(&actor, &assets, "walk", 0.3).unwrap()
        );
        let corpse = definitions
            .iter()
            .find(|a| a.id == format!("{name}_remains"))
            .unwrap();
        let remains = Document::load(corpse.source)
            .unwrap()
            .evaluate()
            .unwrap()
            .mesh;
        let (low, high) = remains.bounds();
        assert!(low[1] >= -0.02, "{name}: corpse below floor: {low:?}");
        assert!(high[1] < mesh.bounds().1[1], "{name}: corpse must lie down");
    }
    if let Some(path) = std::env::var_os("ELDIRON_FAUNA_PREVIEW") {
        overview.save(path).unwrap();
    }
}
