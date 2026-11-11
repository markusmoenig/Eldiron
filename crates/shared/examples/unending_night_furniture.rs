//! Add editable inn furniture and exercise ordinary Use intents. Outputs go under generated/.
use eldiron_shared::{project::Project, project_io, rusterix_utils};
use rusterix::EntityAction;
use serde_json::Value as Json;
use std::{path::Path, time::Duration};
use theframework::prelude::*;
const BED: &str = "792ef4e9-189e-4c1e-9b91-000000000011";
const BENCH: &str = "792ef4e9-189e-4c1e-9b91-000000000012";
const BED_ITEM: &str = "792ef4e9-189e-4c1e-9b91-000000000013";
const BENCH_ITEM: &str = "792ef4e9-189e-4c1e-9b91-000000000014";
fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}
fn install(p: &mut Project) -> Result<(), Box<dyn std::error::Error>> {
    let find = |name: &str| {
        p.tiles
            .iter()
            .find(|(_, t)| t.alias.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
            .ok_or(format!("Missing material {name}"))
    };
    let wood = find("Inn / oak")?;
    let cloth = find("Inn / cloth")?;
    let linen = find("Inn / plaster")?;
    let bed = include_str!("../../../creator/embedded/meshes/recipes/inn-bed.mzgeometry.json")
        .replace("ruleset:material/wood", &wood.to_string())
        .replace("ruleset:material/cloth", &cloth.to_string())
        .replace("ruleset:material/linen", &linen.to_string());
    let bench = include_str!("../../../creator/embedded/meshes/recipes/inn-bench.mzgeometry.json")
        .replace("ruleset:material/wood", &wood.to_string());
    p.procedural_appearances.insert(id(BED), bed);
    p.procedural_appearances.insert(id(BENCH), bench);
    // Replace the static benches with placed items, including the outside bench.
    let old_bench = find("Inn / Bench")?;
    let remove = p.regions[0]
        .map
        .sectors
        .iter()
        .filter(|s| {
            s.properties
                .get_default_source()
                .is_some_and(|src| matches!(src,rusterix::PixelSource::TileId(t) if *t==old_bench))
        })
        .map(|s| s.id)
        .collect::<Vec<_>>();
    p.regions[0].map.delete_elements(&[], &[], &remove);
    for (uuid, recipe, name, position, yaw, description) in [
        (
            BED_ITEM,
            BED,
            "Inn Bed",
            Vec3::new(10.45, 0.1, 8.0),
            0.,
            "A blue wool blanket covers a straw mattress in an oak frame.",
        ),
        (
            BENCH_ITEM,
            BENCH,
            "Inn Bench",
            Vec3::new(8., 0.1, 7.9),
            0.,
            "An oak bench worn smooth by travellers.",
        ),
        (
            "792ef4e9-189e-4c1e-9b91-000000000015",
            BENCH,
            "Courtyard Bench",
            Vec3::new(6.3, 0., 10.7),
            90.,
            "An oak bench beside the inn courtyard.",
        ),
    ] {
        let mut item = eldiron_shared::item::Item::new();
        item.id = id(uuid);
        item.item_id = Uuid::nil(); // Placement-owned furniture does not clutter the global Items list.
        item.name = name.into();
        item.position = position;
        item.data = format!(
            "[attributes]\nvisible = true\nstatic = true\nblocking = true\nappearance_geometry = \"{recipe}\"\nappearance_scale = 1.0\nappearance_yaw = {yaw}\n"
        );
        item.authoring = format!("title = \"{name}\"\ndescription = \"{description}\"\n");
        p.regions[0].items.insert(item.id, item);
    }
    // Use the current official humanoid, including its editable Sit and Lie clips.
    // Only replace an inherited bundled copy; retain deliberately customized appearances.
    let current = include_str!("../../ruleset/rulesets/eldiron/v1/assets/humanoid.mzgeometry.json");
    for source in p.procedural_appearances.values_mut() {
        if let Ok(mut d) = serde_json::from_str::<Json>(source)
            && d.get("label")
                .and_then(Json::as_str)
                .is_some_and(|label| label.contains("Ruleset humanoid"))
        {
            let official: Json = serde_json::from_str(current)?;
            let rows = d["rows"].as_array_mut().ok_or("Rows missing")?;
            for name in ["sit", "lie"] {
                rows.retain(|r| r["id"].as_str() != Some(name));
                rows.push(
                    official["rows"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["id"].as_str() == Some(name))
                        .unwrap()
                        .clone(),
                );
            }
            *source = serde_json::to_string_pretty(&d)?;
        }
    }
    p.regions[0].map.changed += 1;
    eldiron_shared::entity_graph::synchronize(p)?;
    rusterix_utils::insert_content_into_maps(p);
    Ok(())
}
fn review(p: &Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = p.clone();
    let mut r = rusterix::Rusterix::new_without_audio();
    rusterix_utils::sync_editor_visual_assets(&mut r, &project);
    rusterix_utils::start_server(&mut r, &mut project, false)?;
    let commands = rusterix_utils::setup_client(&mut r, &mut project);
    r.server.process_client_commands(commands);
    fn tick(r: &mut rusterix::Rusterix, p: &mut Project, n: usize) {
        for _ in 0..n {
            std::thread::sleep(Duration::from_millis(10));
            rusterix_utils::warmup_runtime(r, p, 1);
        }
    }
    tick(&mut r, &mut project, 30);
    r.resize_client_surface((1200, 900));
    for (uuid, posture, approach) in [
        (BED_ITEM, "lying", Vec2::new(9.4, 8.0)),
        (BENCH_ITEM, "sitting", Vec2::new(8., 8.8)),
    ] {
        let item = project.regions[0]
            .map
            .items
            .iter()
            .find(|i| i.creator_id == id(uuid))
            .unwrap()
            .id;
        r.server.local_player_teleport_pos(approach);
        tick(&mut r, &mut project, 20);
        r.server.local_player_action(EntityAction::ItemClicked(
            item,
            0.,
            Some("use".into()),
            None,
        ));
        tick(&mut r, &mut project, 80);
        let map = &project.regions[0].map;
        let actor = map.entities.iter().find(|e| e.is_player()).unwrap();
        assert_eq!(
            actor.attributes.get_str("posture"),
            Some(posture),
            "Official Use must enter furniture posture"
        );
        assert!(actor.attributes.get_bool_default("resting", false));
        assert!(
            actor.attributes.get_str("avatar_animation").is_some_and(
                |a| a.eq_ignore_ascii_case(if posture == "lying" { "lie" } else { "sit" })
            )
        );
        r.draw_game(map, vec![], vec![], vec![]);
        for _ in 0..16 {
            r.client.inc_animation_frame();
        }
        r.draw_game(map, vec![], vec![], vec![]);
        let mut png = TheRGBABuffer::new(TheDim::sized(1200, 900));
        r.client.insert_game_buffer(&mut png);
        std::fs::write(out.join(format!("inn-{posture}.png")), png.to_png()?)?;
        r.server.local_player_action(EntityAction::ItemClicked(
            item,
            0.,
            Some("use".into()),
            None,
        ));
        tick(&mut r, &mut project, 20);
        let actor = project.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        assert_eq!(actor.attributes.get_str("posture"), Some("standing"));
        assert!(!actor.attributes.get_bool_default("resting", true));
        assert!((actor.get_pos_xz() - approach).magnitude() < 0.1);
        // Movement must leave the furniture before normal path finding resumes.
        r.server.local_player_action(EntityAction::ItemClicked(
            item,
            0.,
            Some("use".into()),
            None,
        ));
        tick(&mut r, &mut project, 20);
        r.server
            .local_player_action(EntityAction::TerrainClicked(approach + Vec2::new(0., 0.25)));
        tick(&mut r, &mut project, 30);
        let actor = project.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        assert_eq!(actor.attributes.get_str("posture"), Some("standing"));
        assert!(!actor.attributes.get_bool_default("resting", true));
        r.server.local_player_action(EntityAction::Off);
    }
    r.server.stop();
    eprintln!(
        "Verified Use -> sit/lie, held poses, Use -> stand and approach-position restoration."
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
    if args.get(2).is_some_and(|v| v == "--review") {
        return Ok(());
    }
    let encoded = project_io::encode_project(&p)?;
    let candidate = out.join("UnendingNight.furniture-candidate.eldiron");
    std::fs::write(&candidate, &encoded)?;
    drop(p);
    let reopened = project_io::decode_project(&encoded)?;
    assert!(reopened.regions[0].items.contains_key(&id(BED_ITEM)));
    assert_eq!(
        reopened.regions[0].items[&id(BED_ITEM)].item_id,
        Uuid::nil()
    );
    assert_eq!(
        std::fs::metadata(path)?.modified()?,
        before,
        "Project saved concurrently"
    );
    std::fs::rename(candidate, path)?;
    eprintln!(
        "Updated {} with editable MZ bed and usable bench.",
        path.display()
    );
    Ok(())
}
