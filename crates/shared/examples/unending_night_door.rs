//! Install/review the inn's ordinary MZ Item. Generated captures stay under generated/.
use eldiron_shared::{entity_graph, project::Project, project_io, rusterix_utils};
use rusterix::{Entity, Item, Value};
use serde_json::json;
use std::{path::Path, sync::Arc};
use theframework::prelude::*;

const RECIPE: &str = "792ef4e9-189e-4c1e-9b91-000000000001";
const DOOR: &str = "792ef4e9-189e-4c1e-9b91-000000000002";
const INSTANCE: &str = "792ef4e9-189e-4c1e-9b91-000000000003";
const KEY: &str = "792ef4e9-189e-4c1e-9b91-000000000004";
const KEY_INSTANCE: &str = "792ef4e9-189e-4c1e-9b91-000000000005";
const KEY_RECIPE: &str = "792ef4e9-189e-4c1e-9b91-000000000006";
fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}

fn install(p: &mut Project) -> Result<(), Box<dyn std::error::Error>> {
    let official = eldiron_shared::rulesets::latest_official_ruleset().parse::<toml::Table>()?;
    p.rules_document.checkpoint();
    p.rules_document.edit(|rules| {
        let intents = rules
            .entry("intents")
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .ok_or("Invalid intents")?;
        for name in ["open", "close", "open_close"] {
            intents.insert(name.into(), official["intents"][name].clone());
        }
        Ok(())
    })?;
    let mut locales = p.locales.parse::<toml::Table>()?;
    let en = locales
        .entry("en")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or("Invalid English locales")?;
    // Locale keys are nested TOML paths.
    let system = en
        .entry("system")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or("Invalid system locales")?;
    system
        .entry("door_locked")
        .or_insert_with(|| toml::Value::String("The door is locked.".into()));
    system
        .entry("door_obstructed")
        .or_insert_with(|| toml::Value::String("Someone is in the doorway.".into()));
    p.locales = toml::to_string_pretty(&locales)?;
    let wood = p
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Inn / oak")
        .ok_or("Missing inn oak material")?
        .0;
    let source =
        include_str!("../../../creator/embedded/meshes/recipes/hinged-door.mzgeometry.json")
            .replace("ruleset:material/wood", &wood.to_string());
    let mut doc = denrim_materials::geometry::Document::load(&source)?;
    let colors = p
        .palette
        .colors
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let iron =
        eldiron_shared::palette::closest(&TheColor::from_hex("#393e47"), &colors).unwrap() as u16;
    if let denrim_materials::geometry::Operation::Material { palette, .. } =
        &mut doc.rows[0].tiles[0].op
    {
        *palette = iron;
    }
    p.procedural_appearances.insert(id(RECIPE), doc.save()?);
    // Fit the opening through the frame's published recipe parameters. Its
    // footprint, lintel and jamb positions are calculated by the same graph.
    let frame = p
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Inn / Doorway")
        .ok_or("Missing doorway")?
        .0
        .to_owned();
    let binding = p.isometric_bakes[&frame].recipe;
    let mut frame_doc =
        serde_json::from_str::<serde_json::Value>(&p.procedural_appearances[&binding])?;
    let controls = frame_doc["rows"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|r| r["tiles"].as_array_mut().unwrap())
        .find_map(|t| {
            (t["op"]["type"] == "group")
                .then(|| t["op"]["controls"].as_array_mut())
                .flatten()
        })
        .ok_or("Missing frame controls")?;
    for c in controls.iter_mut() {
        if c["id"] == "thickness" {
            c["value"] = json!(0.34);
            c["modifiers"]
                .as_array_mut()
                .unwrap()
                .retain(|b| b["target"]["operation"] != "lintel-box");
        }
        if c["id"] == "header_y" {
            c["expression"] = json!("height - lintel_thickness / 2");
        }
    }
    if !controls.iter().any(|c| c["id"] == "lintel_thickness") {
        controls.push(json!({"id":"lintel_thickness","label":"Lintel thickness (m)","category":"Dimensions",
            "value":0.26,"reference":0.26,"min":0.08,"max":0.5,"order":3,
            "modifiers":[{"type":"parameter","target":{"operation":"lintel-box","parameter":"size1"},"factor":1,"offset":0}]}));
    }
    let frame_doc =
        denrim_materials::geometry::Document::load(&serde_json::to_string(&frame_doc)?)?;
    p.procedural_appearances.insert(binding, frame_doc.save()?);
    p.isometric_bakes.get_mut(&frame).unwrap().signature = 0;
    let mut template = eldiron_shared::item::Item::new();
    template.id = id(DOOR);
    template.name = "Inn Door".into();
    template.data = format!(
        r#"[attributes]
visible = true
static = true
blocking = true
radius = 0.1
active = false
openable = true
locked = false
required_key = "{KEY}"
appearance_geometry = "{RECIPE}"
appearance_scale = 1.0
appearance_yaw = 0.0
description = "A stout oak door reinforced with iron bands."
"#
    );
    p.items.insert(template.id, template);
    let mut door = eldiron_shared::item::Item::new();
    door.id = id(INSTANCE);
    door.name = "Inn Door".into();
    door.item_id = id(DOOR);
    // Seat the closed leaf in the frame; the MZ Open pose supplies jamb clearance.
    door.position = Vec3::new(7.87, 0.09, 9.5);
    door.authoring = concat!(
        "title = \"Inn Door\"\n",
        "description = \"Weathered oak planks bound with iron bands.\"\n\n",
        "[state.off]\ndescription = \"The oak door is shut. Its iron handle is worn smooth by travellers.\"\n\n",
        "[state.on]\ndescription = \"The door stands open, revealing the quiet inn beyond.\"\n"
    ).into();

    p.regions[0].items.insert(door.id, door);

    let key_rows = [("shaft",[0.30,0.055,0.05],[0.,0.055,0.]),
        ("bow",[0.13,0.055,0.17],[-0.19,0.055,0.]),
        ("tooth-a",[0.045,0.055,0.08],[0.10,0.055,0.05]),
        ("tooth-b",[0.045,0.055,0.08],[0.02,0.055,0.05])]
        .into_iter().map(|(name,size,at)| json!({"id":name,"label":name,"tiles":[
            {"id":format!("{name}-box"),"op":{"type":"box","size":size}},
            {"id":format!("{name}-at"),"op":{"type":"transform","translation":at,"rotation":[0,0,0],"scale":[1,1,1]}},
            {"id":format!("{name}-brass"),"op":{"type":"assign_material","row":"brass"}}
        ]})).collect::<Vec<_>>();
    let mut key_rows = key_rows;
    key_rows.insert(0,json!({"id":"brass","label":"Brass","tiles":[{"id":"brass-material","op":{"type":"material","palette":21,"roughness":0.35,"metallic":0.7}}]}));
    key_rows.push(json!({"id":"output","label":"Key","tiles":[{"id":"key-assembly","op":{"type":"assemble","rows":["shaft","bow","tooth-a","tooth-b"]}}]}));
    p.procedural_appearances.insert(id(KEY_RECIPE), serde_json::to_string_pretty(&json!({
        "version":1,"id":"inn-key","label":"Inn / Brass key","rows":key_rows,"output":"output","grip":null
    }))?);
    let mut key = eldiron_shared::item::Item::new();
    key.id = id(KEY);
    key.name = "Inn Key".into();
    key.data = format!(
        "[attributes]\nvisible = true\nstatic = false\nblocking = false\nappearance_geometry = \"{KEY_RECIPE}\"\ndescription = \"The brass key for the inn door.\"\n"
    );
    p.items.insert(key.id, key);
    let mut key = eldiron_shared::item::Item::new();
    key.id = id(KEY_INSTANCE);
    key.name = "Inn Key".into();
    key.item_id = id(KEY);
    key.position = Vec3::new(9.45, 0.05, 12.1);
    p.regions[0].items.insert(key.id, key);
    for screen in p.screens.values_mut() {
        for sector in &mut screen.map.sectors {
            let Some(Value::Str(data)) = sector.properties.get("data").cloned() else {
                continue;
            };
            let Ok(mut t) = data.parse::<toml::Table>() else {
                continue;
            };
            if t.get("ui")
                .and_then(|v| v.get("role"))
                .and_then(toml::Value::as_str)
                != Some("action_bar")
            {
                continue;
            }
            let groups = t
                .get_mut("ui")
                .and_then(|v| v.get_mut("groups"))
                .and_then(toml::Value::as_array_mut)
                .ok_or("Missing toolbar groups")?;
            let group = groups
                .iter_mut()
                .find(|g| g.get("align").and_then(toml::Value::as_str) == Some("right"))
                .ok_or("Missing right toolbar group")?;
            group["buttons"] = toml::Value::try_from(vec![
                json!({"command":"intent.open_close"}),
                json!({"command":"intent.use"}),
                json!({"command":"intent.look"}),
            ])?;
            sector
                .properties
                .set("data", Value::Str(toml::to_string_pretty(&t)?));
        }
    }
    p.sync_ruleset_items()?;
    entity_graph::synchronize(p)?;
    let errors = eldiron_shared::isometric::synchronize(p);
    if !errors.is_empty() {
        return Err(format!("Door frame: {errors:?}").into());
    }
    Ok(())
}

fn verify(p: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = rusterix::Rusterix::new_without_audio();
    rusterix_utils::sync_editor_visual_assets(&mut runtime, p);
    rusterix_utils::sync_display_templates(&mut runtime.assets, p, false);
    let assets = &runtime.assets;
    let mut door = Item::new();
    door.id = 600;
    rusterix::server::data::apply_item_data(&mut door, &p.items[&id(DOOR)].data);
    door.position = p.regions[0].items[&id(INSTANCE)].position;
    let actor = Entity::default();
    let mut sprites = rusterix::procedural_avatar::Runtime::default();
    let mut states = Vec::new();
    for (name, verb) in [
        ("closed", "close"),
        ("open", "open"),
        ("closed-again", "close"),
    ] {
        rusterix::item_state::request(&mut door, &actor, verb).unwrap()?;
        let closed_hit = rusterix::item_state::intersects(
            &door,
            assets,
            Vec2::new(door.position.x + 0.63, door.position.z),
            0.09,
            1.8,
            0.2,
        );
        assert_eq!(closed_hit, Some(name != "open"));
        let open_leaf = rusterix::item_state::intersects(
            &door,
            assets,
            Vec2::new(door.position.x - 0.8, door.position.z + 0.35),
            0.09,
            1.8,
            0.2,
        );
        assert_eq!(open_leaf, Some(name == "open"));
        let sprite = sprites
            .isometric_item_sprite(&door, assets)
            .ok_or("Missing iso item sprite")?;
        let mut png = TheRGBABuffer::new(TheDim::sized(128, 128));
        png.pixels_mut().copy_from_slice(&sprite.pixels);
        std::fs::write(out.join(format!("door-{name}.png")), png.to_png()?)?;
        let mut vm = scenevm::SceneVM::default();
        assert!(sprites.item_objects(
            &mut vm,
            &door,
            assets,
            scenevm::GeoId::Item(600),
            door.position
        ));
        if name != "closed-again" {
            let mut map = p.regions[0].map.clone();
            map.items.push(door.clone());
            let mut scene = rusterix::SceneHandler::default();
            scene.settings.read(&p.config)?;
            scene.settings.apply_hour(12.);
            scene.build_dynamics_2d(&map, 0, assets, &Default::default());
            let avatars = scene.isometric_avatars(&map, assets, 0);
            let projection = rusterix::isometric::Projection {
                unit: 52.,
                origin: Vec2::new(500., 450.)
                    - rusterix::isometric::Projection {
                        unit: 52.,
                        origin: Vec2::zero(),
                    }
                    .screen(Vec2::new(8.5, 9.5), 0.),
            };
            let mut image = TheRGBABuffer::new(TheDim::sized(1000, 760));
            let lighting = rusterix::isometric::lighting::Lighting::new(
                &scene.settings,
                scene.vm.vm.lights.values(),
                0,
            );
            rusterix::isometric::draw_scene_gpu_cached(
                &map,
                assets,
                &mut image,
                projection,
                0.,
                None,
                &avatars,
                None,
                &lighting,
                &mut scene.vm,
                &[],
            )?;
            std::fs::write(out.join(format!("inn-{name}-iso.png")), image.to_png()?)?;
            // The native 3D fixture uses the same item, recipe, state and transform.
            let mut map = rusterix::Map::default();
            let mut local_door = door.clone();
            local_door.position = Vec3::new(-0.79, 0.09, 0.);
            map.items.push(local_door);
            let mut floor = rusterix::GeometryObject::box_from_bounds(
                "Floor",
                Vec3::new(-2.5, -0.15, -2.5),
                Vec3::new(2.5, 0., 2.5),
            );
            for face in &mut floor.faces {
                face.tile = Some(rusterix::PixelSource::PaletteIndex(7));
            }
            map.geometry_objects.push(floor);
            map.camera = rusterix::MapCamera::ThreeDIso;
            let mut widget = rusterix::client::widget::game::GameWidget::new();
            let mut scene3d = rusterix::SceneHandler::default();
            scene3d.base_settings.read(&p.config)?;
            scene3d.settings = scene3d.base_settings.clone();
            widget.rect.width = 1000.;
            widget.rect.height = 760.;
            widget.buffer = TheRGBABuffer::new(TheDim::sized(1000, 760));
            widget.set_camera_mode(rusterix::PlayerCamera::D3Iso);
            use rusterix::D3Camera;
            let mut camera = rusterix::D3IsoCamera::new();
            camera.scale = 3.0;
            camera.set_parameter_vec3("center", Vec3::new(0., 0.8, 0.));
            widget.camera_d3 = Box::new(camera);
            widget.graphical_build(&map, assets, &mut scene3d);
            for frame in 0..20 {
                widget.apply_entities(&map, assets, frame, &mut scene3d);
                widget.prepare_frame(&map, &p.time, frame, assets, &mut scene3d);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let mut pixels = vec![0; 1000 * 760 * 4];
            scene3d.vm.render_frame(&mut pixels, 1000, 760);
            image.pixels_mut().copy_from_slice(&pixels);
            std::fs::write(out.join(format!("door-{name}-3d.png")), image.to_png()?)?;
        }
        states.push(sprite.pixels);
    }
    assert_ne!(states[0], states[1]);
    assert_eq!(states[0], states[2]);
    let mut locked = door.clone();
    locked.set_attribute("locked", Value::Bool(true));
    assert!(
        rusterix::item_state::request(&mut locked, &actor, "open")
            .unwrap()
            .is_err()
    );
    let mut actor = actor;
    let mut key = Item::new();
    key.set_attribute("creator_template_id", Value::Str(KEY.into()));
    actor.inventory = vec![None; 8];
    actor.add_item(key)?;
    assert!(
        rusterix::item_state::request(&mut locked, &actor, "open")
            .unwrap()
            .is_ok()
    );
    assert!(!locked.attributes.get_bool_default("locked", true));
    assert_eq!(actor.iter_inventory().count(), 1);
    // Lock/state belong to each instance, never to the shared recipe.
    assert!(!door.attributes.get_bool_default("active", true));
    assert!(Arc::ptr_eq(&states[0], &states[2]));
    eprintln!(
        "Verified shared iso sprites / native 3D meshes / footprints, reversible states, keys and instance independence."
    );
    Ok(())
}

fn verify_live(p: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = p.clone();
    let mut runtime = rusterix::Rusterix::new_without_audio();
    rusterix_utils::sync_editor_visual_assets(&mut runtime, &project);
    rusterix_utils::insert_content_into_maps(&mut project);
    rusterix_utils::start_server(&mut runtime, &mut project, false)?;
    let commands = rusterix_utils::setup_client(&mut runtime, &mut project);
    runtime.server.process_client_commands(commands);
    fn tick(r: &mut rusterix::Rusterix, p: &mut Project, frames: usize) {
        for _ in 0..frames {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(r, p, 1);
        }
    }
    tick(&mut runtime, &mut project, 30);
    let door = project.regions[0]
        .map
        .items
        .iter()
        .find(|i| i.creator_id == id(INSTANCE))
        .ok_or("Door did not spawn")?
        .id;
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(8.5, 10.5));
    tick(&mut runtime, &mut project, 20);
    // Explicit Look uses the placed item's authoring, privately anchored to that item.
    runtime
        .server
        .local_player_action(rusterix::EntityAction::ItemClicked(
            door,
            0.,
            Some("look".into()),
            None,
        ));
    tick(&mut runtime, &mut project, 20);
    let map = &project.regions[0].map;
    let texts = runtime.server.get_world_texts(&map.id);
    assert_eq!(
        texts.len(),
        1,
        "Official Look failed to emit a world description"
    );
    assert_eq!(
        texts[0].1.anchor,
        rusterix::world_text::TextAnchor::Item(door)
    );
    assert!(texts[0].1.text.contains("The oak door is shut."));
    assert!(!texts[0].1.text.contains("Off is Closed"));
    runtime
        .client
        .receive_world_texts(map, &runtime.assets, texts);
    assert!(
        runtime.client.screen_is_responsive(),
        "Game screen must be responsive"
    );
    runtime.resize_client_surface((1400, 900));
    assert_eq!(runtime.client.viewport, Vec2::new(1400, 900));
    runtime.draw_game(map, vec![], vec![], vec![]);
    let mut capture = TheRGBABuffer::new(TheDim::sized(1400, 900));
    runtime.client.insert_game_buffer(&mut capture);
    assert_eq!(runtime.client.target_offset, Vec2::zero());
    std::fs::write(out.join("inn-look-iso.png"), capture.to_png()?)?;
    runtime.client.clear_say_messages();
    runtime
        .server
        .local_player_action(rusterix::EntityAction::TerrainClicked(Vec2::new(8.5, 8.5)));
    tick(&mut runtime, &mut project, 80);
    let player = project.regions[0]
        .map
        .entities
        .iter()
        .find(|e| e.is_player())
        .ok_or("Player did not spawn")?;
    assert!(
        player.position.z > 9.45,
        "Closed door failed to block: {:?}",
        player.position
    );
    runtime
        .server
        .local_player_action(rusterix::EntityAction::Off);
    runtime
        .server
        .local_player_action(rusterix::EntityAction::ItemClicked(
            door,
            0.,
            Some("open_close".into()),
            None,
        ));
    tick(&mut runtime, &mut project, 20);
    assert!(
        project.regions[0]
            .map
            .items
            .iter()
            .find(|i| i.id == door)
            .unwrap()
            .attributes
            .get_bool_default("active", false),
        "Official Open intent did not open the door"
    );
    runtime
        .server
        .local_player_action(rusterix::EntityAction::TerrainClicked(Vec2::new(8.5, 8.5)));
    tick(&mut runtime, &mut project, 100);
    let player = project.regions[0]
        .map
        .entities
        .iter()
        .find(|e| e.is_player())
        .unwrap();
    assert!(
        player.position.z < 9.3,
        "Open door blocked the doorway: {:?}",
        player.position
    );
    runtime
        .server
        .local_player_action(rusterix::EntityAction::Off);
    runtime
        .server
        .local_player_action(rusterix::EntityAction::ItemClicked(
            door,
            0.,
            Some("close".into()),
            None,
        ));
    tick(&mut runtime, &mut project, 20);
    assert!(
        !project.regions[0]
            .map
            .items
            .iter()
            .find(|i| i.id == door)
            .unwrap()
            .attributes
            .get_bool_default("active", true),
        "Official Close intent did not close the door"
    );
    runtime.server.stop();
    eprintln!(
        "Verified live official Open/Close intents, closed-door collision and walking through the open doorway."
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let path = Path::new(args.first().ok_or("Supply the existing project path")?);
    let out = Path::new(args.get(1).ok_or("Supply an ignored review directory")?);
    std::fs::create_dir_all(out)?;
    let before = std::fs::metadata(path)?.modified()?;
    let mut p = project_io::decode_project(&std::fs::read(path)?)?;
    if args.get(2).is_some_and(|a| a == "--review-live") {
        return verify_live(&p, out);
    }
    if args.get(2).is_some_and(|a| a == "--review") {
        verify(&p, out)?;
        return verify_live(&p, out);
    }
    install(&mut p)?;
    verify(&p, out)?;
    verify_live(&p, out)?;
    let bytes = project_io::encode_project(&p)?;
    let candidate = out.join("UnendingNight.door-candidate.eldiron");
    std::fs::write(&candidate, &bytes)?;
    drop(p);
    let reopened = project_io::decode_project(&bytes)?;
    assert_eq!(reopened.regions[0].items[&id(INSTANCE)].item_id, id(DOOR));
    assert!(reopened.procedural_appearances[&id(RECIPE)].contains("state_switch"));
    assert_eq!(
        std::fs::metadata(path)?.modified()?,
        before,
        "Project was saved concurrently"
    );
    std::fs::rename(candidate, path)?;
    eprintln!(
        "Installed normal Inn Door/Inn Key items and Open/Close, Use, Look toolbar in {}",
        path.display()
    );
    Ok(())
}
