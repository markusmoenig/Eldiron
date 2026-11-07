//! Export authored weapon animation poses using the shared 2D/3D appearance evaluator.
use denrim_materials::geometry::Document;
use shared::rulesets;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let out = std::path::PathBuf::from(
        args.first()
            .map_or("/tmp/mz-weapon-animation", String::as_str),
    );
    std::fs::create_dir_all(&out)?;
    let mut data = serde_json::Map::new();
    for asset in rulesets::bundled_appearance_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .filter(|a| matches!(a.id, "humanoid" | "orc" | "skeleton"))
    {
        let mut doc = Document::load(asset.source)?;
        doc.rows.truncate(1);
        doc.output = "body".into();
        doc.grip = None;
        let mesh = doc.evaluate()?.mesh;
        let rig = mesh.rig.as_ref().unwrap();
        data.insert(asset.id.into(),serde_json::json!({"bounds":mesh.bounds(),"joints":rig.joints.iter().map(|j|serde_json::json!({"id":j.id,"parent":j.parent.map(|p|rig.joints[p].id.clone()),"frame":j.local.frame()})).collect::<Vec<_>>()}));
    }
    std::fs::write(out.join("rigs.json"), serde_json::to_string_pretty(&data)?)?;
    if args.iter().any(|s| s == "--rigs") {
        return Ok(());
    }
    use rusterix::{
        Assets, Entity, Item, Value,
        procedural_avatar::{Appearance, attack_duration},
    };
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
    let phases = [0_f32, 0.12, 0.30, 0.55, 0.60, 0.64, 0.75, 0.84, 1.];
    let weapons = [
        "hunting_bow",
        "training_spear",
        "training_sword",
        "hand_axe",
        "novice_mace",
    ];
    for name in weapons {
        let mut sheet = vec![0; 576 * 576 * 4];
        for (race_index, race) in ["humanoid", "orc", "skeleton"].into_iter().enumerate() {
            let mut entity = Entity::default();
            entity.equipped.insert("main_hand".into(), make_item(name));
            for (slot, garment) in [
                ("torso", "linen_shirt"),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
            ] {
                entity.equipped.insert(slot.into(), make_item(garment));
            }
            let appearance = &assets.procedural_avatars[race];
            for (view, heading) in [0_f32, 45., 90.].into_iter().enumerate() {
                entity.orientation =
                    Vec2::new(heading.to_radians().sin(), heading.to_radians().cos());
                for (column, phase) in phases.into_iter().enumerate() {
                    let pixels = appearance.sprite(
                        &entity,
                        &assets,
                        "attack",
                        phase * attack_duration(&entity),
                    )?;
                    for y in 0..64 {
                        let to = (((race_index * 3 + view) * 64 + y) * 576 + column * 64) * 4;
                        sheet[to..to + 256].copy_from_slice(&pixels[y * 256..y * 256 + 256]);
                    }
                }
            }
        }
        std::fs::write(
            out.join(format!("{name}-poses.png")),
            TheRGBABuffer::from(sheet, 576, 576).to_png()?,
        )?;
    }
    if args.iter().any(|s| s == "--gpu") {
        use rusterix::{D3Camera, D3IsoCamera};
        use scenevm::{Atom, GeoId, RenderMode, SceneVM};
        let mut vm = SceneVM::new(768, 512);
        let mut uploaded = Default::default();
        let mut camera = D3IsoCamera::new();
        camera.scale = 1.6;
        camera.center = Vec3::new(0., 1., 0.35);
        camera.azimuth_deg = 50.;
        camera.elevation_deg = 20.;
        vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        vm.execute(Atom::SetBackground(Vec4::new(0.12, 0.12, 0.12, 1.)));
        vm.execute(Atom::SetGP0(Vec4::new(0.02, 0.02, 0.02, 0.)));
        vm.execute(Atom::SetGP1(Vec4::new(1., 1., 1., 0.6)));
        vm.execute(Atom::SetGP2(Vec4::new(0.4, 0.8, 0.5, 1.)));
        vm.execute(Atom::SetGP3(Vec4::new(1., 1., 1., 0.7)));
        vm.execute(Atom::SetGP8(Vec4::new(0., 5., 1., 1.)));
        vm.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 1.)));
        vm.execute(Atom::SetCamera3D {
            camera: camera.as_scenevm_camera_for_surface(768., 512.),
        });
        let animate = args.iter().any(|s| s == "--animate");
        let sample_phases = if animate {
            (0..=32).map(|i| i as f32 / 32.).collect::<Vec<_>>()
        } else {
            phases.to_vec()
        };
        for name in weapons {
            if animate && !matches!(name, "hunting_bow" | "training_spear") {
                continue;
            }
            for (index, phase) in sample_phases.iter().copied().enumerate() {
                vm.execute(Atom::ClearDynamics);
                let mut sprites = vec![0; 192 * 64 * 4];
                for (i, race) in ["humanoid", "orc", "skeleton"].into_iter().enumerate() {
                    let mut entity = Entity::default();
                    entity.equipped.insert("main_hand".into(), make_item(name));
                    entity.orientation = Vec2::new(0., 1.);
                    for (slot, garment) in [
                        ("torso", "linen_shirt"),
                        ("legs", "wool_trousers"),
                        ("feet", "leather_shoes"),
                    ] {
                        entity.equipped.insert(slot.into(), make_item(garment));
                    }
                    entity.set_attribute("avatar_animation", Value::Str("Attack".into()));
                    let height = if race == "orc" { 1.95 } else { 1.8 };
                    let objects = assets.procedural_avatars[race].objects(
                        &entity,
                        &assets,
                        GeoId::Character(i as u32),
                        Vec3::new((i as f32 - 1.) * 1.15, 0., 0.),
                        height,
                        "attack",
                        phase * attack_duration(&entity),
                        &mut vm,
                        &mut uploaded,
                        false,
                    )?;
                    for object in objects {
                        vm.execute(Atom::AddDynamic { object });
                    }
                    let a = 45_f32.to_radians();
                    entity.orientation = Vec2::new(a.sin(), a.cos());
                    let rgba = assets.procedural_avatars[race].sprite(
                        &entity,
                        &assets,
                        "attack",
                        phase * attack_duration(&entity),
                    )?;
                    for y in 0..64 {
                        let to = (y * 192 + i * 64) * 4;
                        sprites[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                    }
                }
                if index == 0 {
                    vm.execute(Atom::BuildAtlas);
                }
                let mut pixels = vec![0; 768 * 512 * 4];
                vm.render_frame(&mut pixels, 768, 512);
                std::fs::write(
                    out.join(format!("{name}-3d-{index}.png")),
                    TheRGBABuffer::from(pixels, 768, 512).to_png()?,
                )?;
                std::fs::write(
                    out.join(format!("{name}-sprites-{index}.png")),
                    TheRGBABuffer::from(sprites, 192, 64).to_png()?,
                )?;
            }
        }
    }
    eprintln!("Authored weapon animation reviews: {}", out.display());
    Ok(())
}
