//! Attach a historical modular HUD to an existing scene without importing its world.
//! unending_night <scene> <hud-project> <destination> [ignored-review-directory]
//! unending_night --review <project> <ignored-review-directory>
//! unending_night --tidy-areas <project>
use eldiron_shared::{
    project::Project, project_io, ruleset_materials, rusterix_utils, screen::Screen,
};
use rusterix::{Rusterix, Tile, Value};
use std::{collections::HashMap, io::Read, path::Path};
use theframework::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--tidy-areas") {
        let mut project = project_io::decode_project(&std::fs::read(&args[1])?)?;
        tidy_area_names(&mut project);
        std::fs::write(&args[1], project_io::encode_project(&project)?)?;
        return Ok(());
    }
    if args.first().is_some_and(|s| s == "--review") {
        let project = project_io::decode_project(&std::fs::read(&args[1])?)?;
        return review(project, Path::new(&args[2]));
    }
    if args.len() < 3 {
        return Err(
            "Expected scene, historical HUD project, destination and optional review directory"
                .into(),
        );
    }
    let mut project = project_io::decode_project(&std::fs::read(&args[0])?)?;
    let original_materials = project.procedural_materials.clone();
    let original_regions = serde_json::to_value(&project.regions)?;
    let source = std::fs::read(&args[1])?;
    let value: serde_json::Value = if source.starts_with(b"PK") {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(source))?;
        let mut bytes = Vec::new();
        archive.by_name("project.json")?.read_to_end(&mut bytes)?;
        serde_json::from_slice(&bytes)?
    } else {
        serde_json::from_slice(&source)?
    };
    let old_screen = value["screens"]
        .as_object()
        .ok_or("HUD project has no screens")?
        .values()
        .find(|s| s["name"] == "Cellar")
        .ok_or("Cellar HUD screen is missing")?;
    let mut screen: Screen = serde_json::from_value(old_screen.clone())?;
    screen.id = project
        .screens
        .values()
        .find(|s| s.name == "Game")
        .map(|s| s.id)
        .unwrap_or_else(Uuid::new_v4);
    screen.name = "Game".into();
    screen.map.name = screen.name.clone();
    for sector in &mut screen.map.sectors {
        let Some(Value::Str(data)) = sector.properties.get("data").cloned() else {
            continue;
        };
        let mut table: toml::Table = data.parse()?;
        if table
            .get("ui")
            .and_then(|u| u.get("role"))
            .and_then(toml::Value::as_str)
            == Some("game")
        {
            table.insert(
                "camera".into(),
                toml::Value::Table(toml::Table::from_iter([(
                    "type".into(),
                    toml::Value::String("2d_iso".into()),
                )])),
            );
            sector
                .properties
                .set("data", Value::Str(toml::to_string_pretty(&table)?));
        }
    }
    // The six tiny nine-slice assets are screen artwork, independent of the old map.
    for n in 1..=6 {
        let id = format!("b6517950-b060-4af5-8bbb-{n:012}");
        let mut tile: Tile = serde_json::from_value(value["tiles"][&id].clone())?;
        tile.alias = tile.alias.replace("Cellar UI", "Unending Night / UI");
        project.tiles.insert(tile.id, tile);
    }
    project.screens.clear();
    project.screens.insert(screen.id, screen);
    project.name = "Unending Night".into();
    let mut config: toml::Table = project.config.parse()?;
    let game = config
        .get_mut("game")
        .and_then(toml::Value::as_table_mut)
        .ok_or("Missing game settings")?;
    game.insert("start_screen".into(), "Game".into());
    game.insert("play_screen".into(), "Game".into());
    project.config = toml::to_string_pretty(&config)?;
    ruleset_materials::install(&mut project)?;
    let icons = [
        "ui_preferences",
        "ui_equipment",
        "ui_inventory",
        "ui_spellbook",
        "ui_actions",
        "look",
        "use",
    ];
    let mut collection =
        eldiron_shared::project::TileCollectionAsset::new("Unending Night / UI".into());
    collection.description =
        "Editable MZ navigation icons and the modular HUD's nine-slice artwork".into();
    let collection_id = collection.id;
    project.add_tile_collection(collection);
    for (i, id) in
        (1..=6)
            .map(|n| Uuid::parse_str(&format!("b6517950-b060-4af5-8bbb-{n:012}")).unwrap())
            .chain(icons.iter().map(|name| {
                eldiron_shared::rulesets::material_asset_id(&format!("icon/{name}/on"))
            }))
            .enumerate()
    {
        let source = rusterix::TileSource::SingleTile(id);
        project.add_source_to_collection(&collection_id, source);
        project.set_collection_tile_board_position(
            &collection_id,
            source,
            Vec2::new((i % 7) as i32, (i / 7) as i32),
        );
    }
    for (id, source) in &original_materials {
        assert_eq!(
            project.procedural_materials.get(id),
            Some(source),
            "Existing world material changed"
        );
    }
    assert_eq!(
        serde_json::to_value(&project.regions)?,
        original_regions,
        "HUD migration changed the world"
    );
    tidy_area_names(&mut project);
    let destination = Path::new(&args[2]);
    std::fs::create_dir_all(destination.parent().unwrap())?;
    std::fs::write(destination, project_io::encode_project(&project)?)?;
    let reopened = project_io::decode_project(&std::fs::read(destination)?)?;
    assert_eq!(reopened.name, "Unending Night");
    assert!(reopened.screens.values().next().unwrap().is_responsive());
    eprintln!(
        "Saved {} with its modular HUD and seven editable MZ icons",
        destination.display()
    );
    if let Some(out) = args.get(3) {
        review(reopened, Path::new(out))?;
    }
    Ok(())
}

fn tidy_area_names(project: &mut Project) {
    let mesh_tiles = project
        .tiles
        .iter()
        .filter(|(_, t)| t.alias.starts_with("Inn / ") && t.isometric.is_some())
        .map(|(id, _)| *id)
        .collect::<std::collections::HashSet<_>>();
    let mut cleared = 0;
    for region in &mut project.regions {
        let terrain = region.map.properties.get("default_terrain_tile").cloned();
        for sector in &mut region.map.sectors {
            if terrain.is_some() && sector.properties.get("source") == terrain.as_ref() {
                if sector.name.is_empty() {
                    sector.name = sector
                        .properties
                        .get_str_default("art_study_part", String::new())
                        .into();
                }
                continue;
            }
            let Some(Value::Source(rusterix::PixelSource::TileId(id))) =
                sector.properties.get("source")
            else {
                continue;
            };
            if !mesh_tiles.contains(id) {
                continue;
            }
            let meaningful_area = sector.properties.get_bool_default("iso_interior", false);
            if !sector.name.is_empty() && !(meaningful_area && sector.name == "Inn") {
                sector
                    .properties
                    .set("art_study_part", Value::Str(sector.name.clone()));
                sector.name = if meaningful_area {
                    "Inn".into()
                } else {
                    String::new()
                };
                cleared += 1;
            }
        }
        let names = region
            .map
            .sectors
            .iter()
            .filter(|s| !s.name.is_empty())
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>();
        eprintln!("{} named gameplay areas: {:?}", region.name, names);
    }
    eprintln!(
        "Removed {cleared} placement labels from gameplay area names; placements and their footprints are retained"
    );
}

fn review(mut project: Project, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let mut icons = HashMap::new();
    for name in [
        "ui_preferences",
        "ui_equipment",
        "ui_inventory",
        "ui_spellbook",
        "ui_actions",
        "look",
        "use",
    ] {
        let name = format!("icon/{name}/on");
        icons.insert(
            name.clone(),
            ruleset_materials::baked(&project, &name)?.data,
        );
    }
    rusterix_utils::insert_content_into_maps(&mut project);
    let mut runtime = Rusterix::new_without_audio();
    runtime.set_tiles_for_maps(project.tiles.clone(), false, [&project.regions[0].map]);
    rusterix_utils::start_server(&mut runtime, &mut project, false)?;
    rusterix_utils::sync_display_templates(&mut runtime.assets, &project, false);
    let commands = rusterix_utils::setup_client(&mut runtime, &mut project);
    runtime.server.process_client_commands(commands);
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        rusterix_utils::warmup_runtime(&mut runtime, &mut project, 1);
    }
    for (path, pixels) in icons {
        let name = path
            .strip_prefix("icon/")
            .unwrap()
            .strip_suffix("/on")
            .unwrap();
        assert_eq!(
            runtime.assets.textures[name].data, pixels,
            "Command icon bypassed its MZ recipe"
        );
    }
    runtime.client.resize_surface(1280, 800, &runtime.assets);
    let map = &project.regions[0].map;
    let capture = |runtime: &mut Rusterix, name: &str| -> Result<(), Box<dyn std::error::Error>> {
        runtime.draw_game(map, vec![], vec![], vec![]);
        let mut buffer = TheRGBABuffer::new(TheDim::sized(1280, 800));
        runtime.client.insert_game_buffer(&mut buffer);
        std::fs::write(out.join(format!("{name}.png")), buffer.to_png().unwrap())?;
        Ok(())
    };
    capture(&mut runtime, "play")?;
    for (i, name) in ["preferences", "equipment", "inventory", "spellbook"]
        .into_iter()
        .enumerate()
    {
        let point = Vec2::new(36 + i as i32 * 46, 773);
        runtime
            .client
            .touch_down(point, map, &runtime.assets, &mut runtime.scene_handler);
        runtime.client.touch_up(point, map, &runtime.assets);
        capture(&mut runtime, name)?;
        runtime
            .client
            .touch_down(point, map, &runtime.assets, &mut runtime.scene_handler);
        runtime.client.touch_up(point, map, &runtime.assets);
    }
    runtime.server.stop();
    eprintln!("Verified persisted MZ command artwork and captured the live modular HUD/panels");
    Ok(())
}
