//! Review the authored orc anatomy, fitted clothing, sockets and heavy clips.
//! cargo run -p eldiron-creator --example orc_appearance -- <output> [--gpu] [--animate] [--before <recipe>]
use denrim_materials::geometry::Document;
use shared::rulesets;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let out = std::path::PathBuf::from(
        args.first()
            .map_or("/tmp/eldiron-orc-review", String::as_str),
    );
    std::fs::create_dir_all(&out)?;
    use rusterix::{Assets, Entity, Item, procedural_avatar::Appearance};
    use std::sync::Arc;
    use theframework::prelude::*;
    let mut assets = Assets::default();
    assets.palette = rulesets::palette_template_from_source(rulesets::latest_official_ruleset())?;
    for def in rulesets::bundled_material_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    ) {
        let editable = rusterix::ruleset_material::editable(&def, &assets.palette)?;
        assets.ruleset_materials.insert(
            format!("ruleset:material/{}", def.id),
            rusterix::ruleset_material::bake(&editable, &assets.palette, 64)?,
        );
    }
    for asset in rulesets::bundled_appearance_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .filter(|a| {
        matches!(
            a.id,
            "humanoid"
                | "orc"
                | "skeleton"
                | "hunting_bow"
                | "training_spear"
                | "training_sword"
                | "hand_axe"
                | "novice_mace"
                | "garment_linen_shirt"
                | "garment_wool_trousers"
                | "garment_leather_shoes"
                | "garment_chain_shirt"
                | "garment_padded_armor"
                | "garment_leather_vest"
                | "garment_cleric_vestments"
                | "round_shield"
                | "small_bag"
                | "worn_small_bag"
                | "ritual_censer"
                | "worn_ritual_censer"
                | "sunward_charm"
                | "worn_sunward_charm"
        )
    }) {
        assets.procedural_avatars.insert(
            asset.id.into(),
            Arc::new(Appearance::new(
                asset.source,
                asset.equipment,
                rusterix::procedural_avatar::fingerprint(&asset.source),
            )?),
        );
    }
    let templates =
        rulesets::ruleset_item_templates_from_source(rulesets::latest_official_ruleset())?;
    let make_item = |name: &str| {
        let mut item = Item::default();
        rusterix::server::data::apply_item_data(
            &mut item,
            &templates.iter().find(|t| t.id == name).unwrap().data,
        );
        item
    };

    // The shield face uses the same MZ icon resource as the game renderer.
    for def in rulesets::bundled_icon_material_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .filter(|d| d.id == "icon/round_shield/on")
    {
        let editable = rusterix::ruleset_material::editable(&def, &assets.palette)?;
        assets.item_icons.insert(
            "round_shield:on".into(),
            vec![rusterix::ruleset_material::bake(
                &editable,
                &assets.palette,
                32,
            )?],
        );
    }
    let before = args
        .iter()
        .position(|s| s == "--before")
        .and_then(|i| args.get(i + 1));
    if let Some(path) = before {
        let source = std::fs::read_to_string(path)?;
        assets.procedural_avatars.insert(
            "orc-before".into(),
            Arc::new(Appearance::new(
                &source,
                &[
                    ("sword-op-1", "main_hand", "sword"),
                    ("shield-op-1", "off_hand", "shield"),
                ],
                rusterix::procedural_avatar::fingerprint(&source),
            )?),
        );
    }
    let dressed = |torso: &str| {
        let mut e = Entity::default();
        for (slot, name) in [
            ("torso", torso),
            ("legs", "wool_trousers"),
            ("feet", "leather_shoes"),
        ] {
            e.equipped.insert(slot.into(), make_item(name));
        }
        e
    };
    let mut armored = dressed("chain_shirt");
    for (slot, name) in [
        ("main_hand", "novice_mace"),
        ("off_hand", "round_shield"),
        ("container", "small_bag"),
        ("focus", "sunward_charm"),
    ] {
        armored.equipped.insert(slot.into(), make_item(name));
    }
    let mut cases = vec![("Human reference", "humanoid", Entity::default())];
    if before.is_some() {
        cases.push(("Previous orc", "orc-before", Entity::default()));
    }
    cases.extend([
        ("Orc anatomy", "orc", Entity::default()),
        ("Orc linen", "orc", dressed("linen_shirt")),
        ("Orc armor and equipment", "orc", armored.clone()),
    ]);
    let mut metrics = serde_json::Map::new();
    let mut heights = std::collections::HashMap::new();
    for (key, appearance) in &assets.procedural_avatars {
        if matches!(key.as_str(), "humanoid" | "orc" | "orc-before") {
            let mut doc = Document::load(&appearance.source)?;
            doc.rows.truncate(1);
            doc.output = "body".into();
            doc.grip = None;
            let mesh = doc.evaluate()?.mesh;
            let (lo, hi) = mesh.bounds();
            heights.insert(key.clone(), hi[1] - lo[1]);
            metrics.insert(key.clone(),serde_json::json!({"bounds":[lo,hi],"joints":mesh.rig.as_ref().unwrap().joints.iter().map(|j|serde_json::json!({"id":j.id,"parent":j.parent,"frame":j.local.frame()})).collect::<Vec<_>>()}));
        }
    }
    let width = cases.len() * 64;
    let mut sprites = vec![0; width * 64 * 4 * 4];
    for (column, (_, source, entity)) in cases.iter().enumerate() {
        for (row, angle) in [0_f32, 45., 90., 180.].into_iter().enumerate() {
            let mut e = entity.clone();
            e.orientation = Vec2::new(angle.to_radians().sin(), angle.to_radians().cos());
            let rgba = assets.procedural_avatars[*source].sprite(&e, &assets, "idle", 0.)?;
            for y in 0..64 {
                let to = ((row * 64 + y) * width + column * 64) * 4;
                sprites[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
            }
        }
    }
    std::fs::write(
        out.join("anatomy-2d.png"),
        TheRGBABuffer::from(sprites, width as u32, 256).to_png()?,
    )?;
    std::fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({"cases":cases.iter().map(|c|c.0).collect::<Vec<_>>(),"metrics":metrics}),
        )?,
    )?;
    if args.iter().any(|s| s == "--gpu") {
        use rusterix::{D3Camera, D3IsoCamera};
        use scenevm::{Atom, GeoId, RenderMode, SceneVM};
        let mut vm = SceneVM::new(1400, 700);
        let mut uploaded = Default::default();
        let mut camera = D3IsoCamera::new();
        camera.scale = 1.65;
        camera.center = Vec3::new(0., 1., 0.);
        camera.elevation_deg = 12.;
        vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        vm.execute(Atom::SetGP0(Vec4::new(0.02, 0.02, 0.02, 0.)));
        vm.execute(Atom::SetGP1(Vec4::new(1., 1., 1., 0.6)));
        vm.execute(Atom::SetGP2(Vec4::new(0.4, 0.8, 0.5, 1.)));
        vm.execute(Atom::SetGP3(Vec4::new(1., 1., 1., 0.7)));
        vm.execute(Atom::SetGP8(Vec4::new(0., 5., 1., 1.)));
        vm.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 1.)));
        for (view, angle) in [
            ("front", 90_f32),
            ("side", 0.),
            ("back", 270.),
            ("three-quarter", 50.),
        ] {
            camera.azimuth_deg = angle;
            let c = camera.as_scenevm_camera_for_surface(1400., 700.);
            vm.execute(Atom::SetCamera3D { camera: c });
            vm.execute(Atom::ClearDynamics);
            for (i, (_, source, entity)) in cases.iter().enumerate() {
                let mut e = entity.clone();
                e.orientation = Vec2::new(0., 1.);
                let pos = c.right * ((i as f32 - (cases.len() - 1) as f32 / 2.) * 1.20);
                for object in assets.procedural_avatars[*source].objects(
                    &e,
                    &assets,
                    GeoId::Character(i as u32),
                    pos,
                    heights[*source],
                    "idle",
                    0.,
                    &mut vm,
                    &mut uploaded,
                    false,
                )? {
                    vm.execute(Atom::AddDynamic { object });
                }
            }
            vm.execute(Atom::BuildAtlas);
            let mut rgba = vec![0; 1400 * 700 * 4];
            vm.render_frame(&mut rgba, 1400, 700);
            std::fs::write(
                out.join(format!("anatomy-{view}.png")),
                TheRGBABuffer::from(rgba, 1400, 700).to_png()?,
            )?;
        }
        if args.iter().any(|s| s == "--animate") {
            let animated = [
                ("Orc anatomy", Entity::default()),
                ("Orc linen", dressed("linen_shirt")),
                ("Orc armored", armored),
            ];
            camera.azimuth_deg = 50.;
            camera.elevation_deg = 18.;
            let c = camera.as_scenevm_camera_for_surface(1400., 700.);
            vm.execute(Atom::SetCamera3D { camera: c });
            for (clip, seconds) in [("walk", 1.35), ("attack", 0.78)] {
                for frame in 0..=32 {
                    vm.execute(Atom::ClearDynamics);
                    let mut sheet = vec![0; 192 * 64 * 4];
                    for (i, (_, entity)) in animated.iter().enumerate() {
                        let mut e = entity.clone();
                        e.orientation = Vec2::new(0., 1.);
                        if clip == "attack" {
                            e.equipped
                                .insert("main_hand".into(), make_item("novice_mace"));
                        }
                        let pos = c.right * ((i as f32 - 1.) * 1.45);
                        let time = frame as f32 / 32. * seconds;
                        for object in assets.procedural_avatars["orc"].objects(
                            &e,
                            &assets,
                            GeoId::Character(i as u32),
                            pos,
                            heights["orc"],
                            clip,
                            time,
                            &mut vm,
                            &mut uploaded,
                            false,
                        )? {
                            vm.execute(Atom::AddDynamic { object });
                        }
                        e.orientation =
                            Vec2::new(45_f32.to_radians().sin(), 45_f32.to_radians().cos());
                        let rgba =
                            assets.procedural_avatars["orc"].sprite(&e, &assets, clip, time)?;
                        for y in 0..64 {
                            let to = (y * 192 + i * 64) * 4;
                            sheet[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                        }
                    }
                    if frame == 0 {
                        vm.execute(Atom::BuildAtlas);
                    }
                    let mut rgba = vec![0; 1400 * 700 * 4];
                    vm.render_frame(&mut rgba, 1400, 700);
                    std::fs::write(
                        out.join(format!("{clip}-3d-{frame}.png")),
                        TheRGBABuffer::from(rgba, 1400, 700).to_png()?,
                    )?;
                    std::fs::write(
                        out.join(format!("{clip}-2d-{frame}.png")),
                        TheRGBABuffer::from(sheet, 192, 64).to_png()?,
                    )?;
                }
            }
        }
    }
    eprintln!("Orc anatomy and animation reviews: {}", out.display());
    Ok(())
}
