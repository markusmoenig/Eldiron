//! Repair editable input bindings and door cutaway, verifying mouse release and shortcuts.
use eldiron_shared::{entity_graph, project::Project, project_io, rusterix_utils};
use rusterix::{EntityAction, Value};
use std::{path::Path, time::Duration};
use theframework::prelude::*;
const DOOR: &str = "792ef4e9-189e-4c1e-9b91-000000000003";
const BED: &str = "792ef4e9-189e-4c1e-9b91-000000000013";
const BENCH: &str = "792ef4e9-189e-4c1e-9b91-000000000014";
fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}
fn install(p: &mut Project) -> Result<(), Box<dyn std::error::Error>> {
    let player = p
        .characters
        .iter()
        .find(|(_, c)| {
            c.data
                .parse::<toml::Table>()
                .ok()
                .and_then(|t| t.get("attributes")?.get("player")?.as_bool())
                .unwrap_or(false)
        })
        .map(|(id, _)| *id)
        .ok_or("No player")?;
    for (key, command) in [
        ("u", "intent.use"),
        ("l", "intent.look"),
        ("o", "intent.open_close"),
    ] {
        entity_graph::set_owner_input(p, &entity_graph::character_key(player), key, command)?;
    }
    let region = p.regions[0].id;
    entity_graph::set_owner_attribute(
        p,
        &entity_graph::instance_key(region, "item", id(DOOR)),
        "iso_cutaway_area",
        serde_json::json!("Inn"),
    )?;
    entity_graph::synchronize(p)?;
    rusterix_utils::insert_content_into_maps(p);
    Ok(())
}
fn review(p: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = p.clone();
    let mut r = rusterix::Rusterix::new_without_audio();
    rusterix_utils::sync_editor_visual_assets(&mut r, &p);
    rusterix_utils::start_server(&mut r, &mut p, false)?;
    let commands = rusterix_utils::setup_client(&mut r, &mut p);
    r.server.process_client_commands(commands);
    fn tick(r: &mut rusterix::Rusterix, p: &mut Project, n: usize) {
        for _ in 0..n {
            std::thread::sleep(Duration::from_millis(10));
            rusterix_utils::warmup_runtime(r, p, 1);
        }
    }
    tick(&mut r, &mut p, 30);
    r.resize_client_surface((1200, 900));
    fn frame(r: &mut rusterix::Rusterix, map: &rusterix::Map) {
        r.draw_game(map, vec![], vec![], vec![]);
        let mut surface = TheRGBABuffer::new(TheDim::sized(1200, 900));
        r.client.insert_game_buffer(&mut surface);
    }
    // Full press/release, including the Off emitted by Creator's mouse-up handler.
    for (uuid, posture, approach) in [
        (BED, "lying", Vec2::new(9.4, 8.)),
        (BENCH, "sitting", Vec2::new(8., 8.8)),
    ] {
        r.server.local_player_teleport_pos(approach);
        tick(&mut r, &mut p, 15);
        let item = p.regions[0]
            .map
            .items
            .iter()
            .find(|i| i.creator_id == id(uuid))
            .unwrap()
            .id;
        r.server.local_player_action(EntityAction::ItemClicked(
            item,
            0.,
            Some("use".into()),
            None,
        ));
        tick(&mut r, &mut p, 5);
        r.server.local_player_action(EntityAction::Off);
        tick(&mut r, &mut p, 60);
        let actor = p.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        assert_eq!(
            actor.attributes.get_str("posture"),
            Some(posture),
            "Mouse release cancelled rest"
        );
        r.server
            .local_player_action(EntityAction::TerrainClicked(approach));
        tick(&mut r, &mut p, 15);
    }
    r.server.local_player_teleport_pos(Vec2::new(8.5, 10.5));
    tick(&mut r, &mut p, 15);
    frame(&mut r, &p.regions[0].map);
    let door = p.regions[0]
        .map
        .items
        .iter()
        .find(|i| i.creator_id == id(DOOR))
        .unwrap()
        .clone();
    assert!(!rusterix::isometric::item_cutaway(&p.regions[0].map, &door));
    let mut projection = None;
    r.client
        .for_each_game_widget_mut(|g| projection = g.isometric_projection);
    let projection = projection.ok_or("Missing iso projection")?;
    let sprites =
        r.scene_handler
            .isometric_avatars(&p.regions[0].map, &r.assets, r.client.animation_frame);
    // Find a real clickable door pixel rather than a ground-plane approximation.
    let map = &p.regions[0].map;
    let center = projection.screen(door.get_pos_xz(), door.position.y + 0.8);
    let mut hit = None;
    'scan: for y in -70..70 {
        for x in -70..70 {
            let pt = (center + Vec2::new(x as f32, y as f32)).map(|v| v.round());
            if rusterix::isometric::pick_item(map, &r.assets, projection, pt, &sprites)
                == Some(door.id)
            {
                hit = Some(pt.map(|v| v as i32));
                break 'scan;
            }
        }
    }
    let point = hit.ok_or("Door silhouette missing")?;
    for (key, intent) in [("u", "use"), ("l", "look"), ("o", "open_close")] {
        assert_eq!(
            r.client
                .user_event("key_down".into(), Value::Str(key.into())),
            EntityAction::Off
        );
        r.client.user_event("key_up".into(), Value::Str(key.into()));
        r.client
            .touch_hover(point, map, &r.assets, &mut r.scene_handler);
        // Rendering between selection and clicking must preserve the intent.
        frame(&mut r, map);
        let action = r
            .client
            .touch_down(point, map, &r.assets, &mut r.scene_handler);
        assert!(
            matches!(action,Some(EntityAction::ItemClicked(item,_,Some(ref selected),_)) if item==door.id && selected==intent),
            "{key}: {action:?}; expected door {}, point {point:?}, offset {:?}",
            door.id,
            r.client.target_offset
        );
        r.client.touch_up(point, map, &r.assets);
    }
    r.server.local_player_teleport_pos(Vec2::new(8.5, 8.5));
    tick(&mut r, &mut p, 15);
    assert!(rusterix::isometric::item_cutaway(&p.regions[0].map, &door));
    frame(&mut r, &p.regions[0].map);
    let mut capture = TheRGBABuffer::new(TheDim::sized(1200, 900));
    r.client.insert_game_buffer(&mut capture);
    std::fs::write(out.join("inn-door-cutaway.png"), capture.to_png()?)?;
    r.server.stop();
    eprintln!(
        "Verified mouse release preserves sit/lie, U/L/O survive key-up and rendering, exact door targeting, and room-specific cutaway."
    );
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let path = Path::new(args.first().ok_or("Project path")?);
    let out = Path::new(args.get(1).ok_or("Ignored output directory")?);
    std::fs::create_dir_all(out)?;
    let before = std::fs::metadata(path)?.modified()?;
    let mut p = project_io::decode_project(&std::fs::read(path)?)?;
    install(&mut p)?;
    review(&p, out)?;
    let bytes = project_io::encode_project(&p)?;
    let candidate = out.join("UnendingNight.intents-candidate.eldiron");
    std::fs::write(&candidate, &bytes)?;
    drop(p);
    let reopened = project_io::decode_project(&bytes)?;
    assert!(
        reopened.regions[0].items[&id(DOOR)]
            .data
            .contains("iso_cutaway_area")
    );
    assert_eq!(
        std::fs::metadata(path)?.modified()?,
        before,
        "Project saved concurrently"
    );
    std::fs::rename(candidate, path)?;
    eprintln!(
        "Updated {} with editable input bindings and door cutaway.",
        path.display()
    );
    Ok(())
}
