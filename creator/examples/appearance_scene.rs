//! Render the ruleset MZ humanoid through Creator's actual GameWidget scene paths.
//! cargo run -p eldiron-creator --example appearance_scene -- <project> <output-dir>
use rusterix::client::widget::game::GameWidget;
use rusterix::{D3Camera, D3IsoCamera, Entity, Item, PlayerCamera, Rusterix, Value};
use shared::{
    project_io::decode_project,
    rusterix_utils::{setup_client, start_server, warmup_runtime},
};
use theframework::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or("test_projects/3DStarter.eldiron".into());
    let out = std::path::PathBuf::from(
        args.next()
            .unwrap_or("/tmp/eldiron-appearance-scenes".into()),
    );
    std::fs::create_dir_all(&out)?;
    let mut project = decode_project(&std::fs::read(path)?)?;
    project.migrate_default_ruleset();
    let options = args.collect::<Vec<_>>();
    if options.iter().any(|s| s == "--pixelized") {
        let mut config = project.config.parse::<toml_edit::DocumentMut>()?;
        config["appearance"]["sprite_pixels_2d"] = toml_edit::value(32);
        config["appearance"]["pixel_size_3d"] = toml_edit::value(4);
        project.config = config.to_string();
    }
    let pixel_comparison = options.iter().any(|s| s == "--pixel-comparison");
    let clothing = options.iter().any(|s| s == "--clothing");
    let races = options.iter().any(|s| s == "--races");
    let weapons = options.iter().any(|s| s == "--weapons");
    let items = options.iter().any(|s| s == "--items") || pixel_comparison;
    let orc = options.iter().any(|s| s == "--orc");
    let weapon_names = ["novice_mace", "hunting_bow", "training_spear"];
    if options.iter().any(|s| s == "--default-palette") {
        project.palette = shared::project::Project::default().palette;
    }
    if clothing || races || weapons || items || orc {
        // Review today's ruleset on an in-memory scene; never rewrite the fixture.
        project.rules_document = Default::default();
    }
    project.sync_ruleset_items()?;
    let item_templates = shared::rulesets::ruleset_item_templates_from_source(
        shared::rulesets::latest_official_ruleset(),
    )?;
    let creator_items = project
        .items
        .values()
        .map(|i| (i.name.clone(), i.id))
        .collect::<std::collections::HashMap<_, _>>();
    let make_item = |name: &str| {
        let template = item_templates.iter().find(|t| t.id == name).unwrap();
        let mut item = Item::default();
        item.item_type = name.into();
        rusterix::server::data::apply_item_data(&mut item, &template.data);
        if let Some(id) = creator_items.get(&template.name) {
            item.creator_id = *id;
        }
        item
    };
    let sword = make_item("training_sword");
    let shield = make_item("round_shield");
    let axe = make_item("hand_axe");
    let outfits = if clothing {
        [
            "linen_shirt",
            "padded_armor",
            "leather_vest",
            "chain_shirt",
            "cleric_vestments",
        ]
        .into_iter()
        .map(|name| {
            let mut worn = vec![
                ("torso", make_item(name)),
                ("legs", make_item("wool_trousers")),
                ("feet", make_item("leather_shoes")),
            ];
            if name == "chain_shirt" {
                worn.push(("main_hand", sword.clone()));
            }
            (name, worn)
        })
        .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let defs = shared::ruleset_materials::definitions(&project.config);
    let icons = defs
        .iter()
        .filter(|d| d.id.starts_with("icon/"))
        .collect::<Vec<_>>();
    let mut icon_pixels = vec![0; icons.len() * 32 * 32 * 4];
    for (i, def) in icons.iter().enumerate() {
        let texture = shared::ruleset_materials::baked(&project, def.id)?;
        assert_eq!((texture.width, texture.height), (32, 32));
        for y in 0..32 {
            let to = (y * icons.len() * 32 + i * 32) * 4;
            icon_pixels[to..to + 128].copy_from_slice(&texture.data[y * 128..y * 128 + 128]);
        }
    }
    std::fs::write(
        out.join("icons.png"),
        TheRGBABuffer::from(icon_pixels, (icons.len() * 32) as u32, 32).to_png()?,
    )?;
    let mut swatches = vec![0; defs.len() * 64 * 64 * 4];
    for (i, def) in defs.iter().enumerate() {
        let texture = shared::ruleset_materials::baked(&project, def.id)?.resized(64, 64);
        for y in 0..64 {
            let start = (y * defs.len() * 64 + i * 64) * 4;
            swatches[start..start + 256].copy_from_slice(&texture.data[y * 256..y * 256 + 256]);
        }
    }
    std::fs::write(
        out.join("materials.png"),
        TheRGBABuffer::from(swatches, (defs.len() * 64) as u32, 64).to_png()?,
    )?;
    let mut runtime = Rusterix::default();
    start_server(&mut runtime, &mut project, false)?;
    let commands = setup_client(&mut runtime, &mut project);
    runtime.server.process_client_commands(commands);
    warmup_runtime(&mut runtime, &mut project, 3);
    let region = project
        .regions
        .iter_mut()
        .find(|r| r.map.name == runtime.client.current_map)
        .ok_or("No active map")?;
    runtime.server.apply_entities_items(&mut region.map);
    let mut map = region.map.clone();
    let player = map
        .entities
        .iter()
        .find(|e| e.is_player())
        .or_else(|| map.entities.first())
        .cloned()
        .unwrap_or_else(Entity::default);
    let mut origin = player.position;
    if let Some(bounds) = map.bounding_box() {
        origin.x = bounds.x + bounds.z * 0.5;
        origin.z = bounds.y + bounds.w * 0.5;
        origin.y = map
            .geometry_floor_height_nearest(Vec2::new(origin.x, origin.z), origin.y)
            .unwrap_or(origin.y);
    }
    // Add a visible comparison group to an in-memory copy of the real scene.
    // They use normal ruleset bindings and go through build_dynamics_2d/3d.
    map.entities.clear();
    let comparison = if items {
        vec![
            (-1.2, -1.45, 0., "Idle"),
            (0., -1.45, 0., "Walk"),
            (1.2, -1.45, 0., "Idle"),
        ]
    } else if weapons {
        (0..9)
            .map(|i| {
                (
                    (i % 3) as f32 * 1.2 - 1.2,
                    (i / 3) as f32 * 1.3 - 1.3,
                    0.,
                    "Idle",
                )
            })
            .collect()
    } else if races {
        vec![
            (-1.1, -0.6, 0., "Idle"),
            (0., -0.6, 0., "Idle"),
            (1.1, -0.6, 0., "Idle"),
            (-1.1, 0.8, 0., "Walk"),
            (0., 0.8, 0., "Walk"),
            (1.1, 0.8, 0., "Attack"),
        ]
    } else if clothing {
        vec![
            (-1.1, -0.55, 0., "Idle"),
            (0., -0.55, 0., "Idle"),
            (1.1, -0.55, 0., "Idle"),
            (-0.55, 0.8, 20., "Walk"),
            (0.55, 0.8, 0., "Walk"),
        ]
    } else {
        vec![
            (-0.9, 0., 0., "Idle"),
            (0.9, 0., 90., "Walk"),
            (0., 1.2, 180., "Attack"),
        ]
    };
    for (i, (x, z, heading, animation)) in comparison.into_iter().enumerate() {
        let mut e = player.clone();
        e.id = 9000 + i as u32;
        e.creator_id = Uuid::new_v4();
        e.position = origin + Vec3::new(x, 0., z);
        let angle: f32 = heading;
        e.orientation = Vec2::new(angle.to_radians().sin(), angle.to_radians().cos());
        e.attributes.remove("avatar_id");
        e.attributes.remove("source");
        e.set_attribute(
            "avatar",
            Value::Str(
                if items {
                    ["humanoid", "orc", "skeleton"][i]
                } else if weapons {
                    ["humanoid", "orc", "skeleton"][i / 3]
                } else if races {
                    ["humanoid", "orc", "skeleton"][i % 3]
                } else if orc {
                    "orc"
                } else {
                    "humanoid"
                }
                .into(),
            ),
        );
        if pixel_comparison {
            e.set_attribute("player", Value::Bool(false));
        }
        e.set_attribute("visible", Value::Bool(true));
        e.set_attribute("avatar_animation", Value::Str(animation.into()));
        e.equipped.clear();
        if items {
            for (slot, name) in [
                ("container", "small_bag"),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
                ("torso", "linen_shirt"),
                ("main_hand", "training_sword"),
                ("off_hand", "round_shield"),
            ] {
                e.equipped.insert(slot.into(), make_item(name));
            }
            e.equipped.insert(
                "focus".into(),
                make_item(if i == 1 {
                    "sunward_charm"
                } else {
                    "ritual_censer"
                }),
            );
        } else if weapons {
            e.equipped
                .insert("main_hand".into(), make_item(weapon_names[i % 3]));
        } else if races && i >= 3 {
            for (slot, name) in [
                ("torso", "chain_shirt"),
                ("legs", "wool_trousers"),
                ("feet", "leather_shoes"),
                ("main_hand", "training_sword"),
                ("off_hand", "round_shield"),
            ] {
                e.equipped.insert(slot.into(), make_item(name));
            }
        } else if clothing {
            for (slot, item) in &outfits[i].1 {
                e.equipped.insert((*slot).into(), item.clone());
            }
        } else if !races && i > 0 {
            e.equipped.insert(
                "main_hand".into(),
                if i == 1 { sword.clone() } else { axe.clone() },
            );
            e.equipped.insert("off_hand".into(), shield.clone());
        }
        map.entities.push(e);
    }
    assert!(
        rusterix::procedural_avatar::Runtime::resolve(&map.entities[0], &runtime.assets).is_some()
    );
    if items {
        map.items.clear();
    }
    for (i, mut item) in if pixel_comparison {
        [
            "training_sword",
            "round_shield",
            "hand_axe",
            "hunting_bow",
            "novice_mace",
            "training_spear",
        ]
        .into_iter()
        .map(make_item)
        .collect()
    } else if items {
        item_templates.iter().map(|t| make_item(&t.id)).collect()
    } else if clothing || races || weapons {
        vec![]
    } else {
        vec![sword, shield, axe]
    }
    .into_iter()
    .enumerate()
    {
        item.id = 9900 + i as u32;
        item.position = origin
            + if pixel_comparison {
                Vec3::new((i % 3) as f32 * 0.9 - 0.9, 0., (i / 3) as f32 * 0.7 + 0.15)
            } else if items {
                Vec3::new((i % 7) as f32 * 0.5 - 1.5, 0., (i / 7) as f32 * 0.42 - 0.6)
            } else {
                Vec3::new(-0.85 + i as f32 * 0.85, 0., 0.8)
            };
        item.attributes.set("visible", Value::Bool(true));
        if items {
            item.attributes.set("active", Value::Bool(true));
        }
        map.items.push(item);
    }
    runtime.set_tiles_for_maps(project.tiles.clone(), false, [&map]);
    runtime.scene_handler.settings.read(&project.config)?;
    for (name, mode) in [("3d", PlayerCamera::D3Iso), ("2d", PlayerCamera::D2)] {
        if options.iter().any(|s| s == "--only-2d") && name != "2d" {
            continue;
        }
        if options.iter().any(|s| s == "--only-3d") && name != "3d" {
            continue;
        }
        let mut widget = GameWidget::new();
        widget.rect.width = 960.;
        widget.rect.height = 640.;
        widget.grid_size = if pixel_comparison { 128. } else { 64. };
        widget.player_pos = Vec2::new(origin.x, origin.z);
        widget.buffer = TheRGBABuffer::new(TheDim::sized(960, 640));
        widget.set_camera_mode(mode);
        let mut camera = D3IsoCamera::new();
        camera.scale = if pixel_comparison {
            3.5
        } else if items {
            4.8
        } else if weapons {
            4.8
        } else {
            3.5
        };
        camera.set_parameter_vec3("center", origin + Vec3::new(0., 0.6, 0.));
        widget.camera_d3 = Box::new(camera);
        for frame in 0..48 {
            widget.prepare_frame(
                &map,
                &project.time,
                frame / 12,
                &runtime.assets,
                &mut runtime.scene_handler,
            );
            widget.apply_entities(
                &map,
                &runtime.assets,
                frame / 12,
                &mut runtime.scene_handler,
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // Capture the complete attack at mid-swing using the same retrigger serial
        // used by the server, after the asynchronous map builders have settled.
        for entity in &mut map.entities {
            if entity.attributes.get_str("avatar_animation") == Some("Attack") {
                let serial = entity.attributes.get_int_default("avatar_attack_serial", 0) + 1;
                entity.set_attribute("avatar_attack_serial", Value::Int(serial));
            }
        }
        widget.prepare_frame(
            &map,
            &project.time,
            4,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        widget.apply_entities(&map, &runtime.assets, 4, &mut runtime.scene_handler);
        std::thread::sleep(std::time::Duration::from_millis(175));
        widget.prepare_frame(
            &map,
            &project.time,
            4,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        widget.apply_entities(&map, &runtime.assets, 4, &mut runtime.scene_handler);
        eprintln!("{name}: {:?}", runtime.scene_handler.vm.vm.debug_stats());
        if name == "3d" {
            assert!(runtime.scene_handler.vm.vm.debug_stats().dynamics >= 3);
        }
        let mut pixels = vec![0; 960 * 640 * 4];
        if pixel_comparison {
            // Render both settings from this one frozen prepared scene. No pose,
            // camera, material, world or time changes between the two captures.
            for (suffix, sprite_grid, block) in [("before", 0, 1), ("after", 32, 4)] {
                runtime
                    .scene_handler
                    .vm
                    .vm
                    .set_entity_pixelization(sprite_grid, block);
                runtime.scene_handler.vm.render_frame(&mut pixels, 960, 640);
                std::fs::write(
                    out.join(format!("{name}-{suffix}.png")),
                    TheRGBABuffer::from(pixels.clone(), 960, 640).to_png()?,
                )?;
            }
        }
        runtime.scene_handler.vm.render_frame(&mut pixels, 960, 640);
        std::fs::write(
            out.join(format!("{name}.png")),
            TheRGBABuffer::from(pixels, 960, 640).to_png()?,
        )?;
    }
    // Also export the actual directional sprite pixels for close inspection.
    let asset =
        rusterix::procedural_avatar::Runtime::resolve(&map.entities[1], &runtime.assets).unwrap();
    let mut sheet = vec![0; 64 * 8 * 64 * 4];
    for direction in 0..8 {
        let mut e = map.entities[1].clone();
        let angle = (direction as f32 * 45.).to_radians();
        e.orientation = Vec2::new(angle.sin(), angle.cos());
        let sprite = asset.sprite(&e, &runtime.assets, "walk", 0.3)?;
        for y in 0..64 {
            let to = (y * 512 + direction * 64) * 4;
            sheet[to..to + 256].copy_from_slice(&sprite[y * 256..y * 256 + 256]);
        }
    }
    std::fs::write(
        out.join("directions.png"),
        TheRGBABuffer::from(sheet, 512, 64).to_png()?,
    )?;
    if races || weapons {
        let width = map.entities.len() * 64;
        let mut sheet = vec![0; width * 256 * 4];
        for (column, entity) in map.entities.iter().enumerate() {
            let appearance =
                rusterix::procedural_avatar::Runtime::resolve(entity, &runtime.assets).unwrap();
            for (row, (heading, clip)) in [
                (0_f32, "idle"),
                (90., "idle"),
                (180., "walk"),
                (0., "attack"),
            ]
            .into_iter()
            .enumerate()
            {
                let mut e = entity.clone();
                e.orientation = Vec2::new(heading.to_radians().sin(), heading.to_radians().cos());
                let rgba = appearance.sprite(
                    &e,
                    &runtime.assets,
                    clip,
                    if clip == "attack" { 0.175 } else { 0.3 },
                )?;
                for y in 0..64 {
                    let to = ((row * 64 + y) * width + column * 64) * 4;
                    sheet[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                }
            }
        }
        std::fs::write(
            out.join(if weapons {
                "weapon-attachments.png"
            } else {
                "races.png"
            }),
            TheRGBABuffer::from(sheet, width as u32, 256).to_png()?,
        )?;
    }
    if weapons {
        let mut sheet = vec![0; 192 * 192 * 4];
        let mut icons = vec![0; 96 * 32 * 4];
        for (column, name) in weapon_names.iter().enumerate() {
            let appearance = &runtime.assets.procedural_avatars[*name];
            for (row, heading) in [0_f32, 45., 90.].into_iter().enumerate() {
                let mut entity = Entity::default();
                entity.orientation =
                    Vec2::new(heading.to_radians().sin(), heading.to_radians().cos());
                let rgba = appearance.sprite(&entity, &runtime.assets, "idle", 0.)?;
                for y in 0..64 {
                    let to = ((row * 64 + y) * 192 + column * 64) * 4;
                    sheet[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                }
            }
            let icon = &runtime.assets.item_icons[*name][0];
            for y in 0..32 {
                let to = (y * 96 + column * 32) * 4;
                icons[to..to + 128].copy_from_slice(&icon.data[y * 128..y * 128 + 128]);
            }
        }
        std::fs::write(
            out.join("weapon-models.png"),
            TheRGBABuffer::from(sheet, 192, 192).to_png()?,
        )?;
        std::fs::write(
            out.join("weapon-icons.png"),
            TheRGBABuffer::from(icons, 96, 32).to_png()?,
        )?;
    }
    if clothing {
        // Compare front, side and back using the exact runtime sprite sampler.
        let mut sheet = vec![0; 64 * 5 * 64 * 3 * 4];
        for (column, entity) in map.entities.iter().enumerate() {
            for (row, heading) in [0_f32, 90., 180.].into_iter().enumerate() {
                let mut e = entity.clone();
                let angle = heading.to_radians();
                e.orientation = Vec2::new(angle.sin(), angle.cos());
                let rgba = asset.sprite(&e, &runtime.assets, "idle", 0.)?;
                for y in 0..64 {
                    let to = ((row * 64 + y) * 320 + column * 64) * 4;
                    sheet[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                }
            }
        }
        std::fs::write(
            out.join("outfits.png"),
            TheRGBABuffer::from(sheet, 320, 192).to_png()?,
        )?;
        let mut sheet = vec![0; 64 * 5 * 64 * 3 * 4];
        for (column, e) in map.entities.iter().enumerate() {
            for (row, (clip, time)) in [("idle", 0.), ("walk", 0.3), ("attack", 0.3)]
                .into_iter()
                .enumerate()
            {
                let rgba = asset.sprite(e, &runtime.assets, clip, time)?;
                for y in 0..64 {
                    let to = ((row * 64 + y) * 320 + column * 64) * 4;
                    sheet[to..to + 256].copy_from_slice(&rgba[y * 256..y * 256 + 256]);
                }
            }
        }
        std::fs::write(
            out.join("outfit-motion.png"),
            TheRGBABuffer::from(sheet, 320, 192).to_png()?,
        )?;
    }
    eprintln!("Ruleset scene reviews: {}", out.display());
    Ok(())
}
