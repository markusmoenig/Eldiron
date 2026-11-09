//! Assemble a starter from PNG assets imported and saved through Creator's UI.
//! This example does not read PNG files or configure sprite import metadata.
//! cargo run -p eldiron-shared --release --example kenney_iso_starter -- imported.eldiron output.eldiron [preview-dir]
use eldiron_shared::{
    character::Character,
    project::Project,
    project_io::{decode_project, encode_project},
    rusterix_utils,
    screen::Screen,
};
use rusterix::{
    Map, MapCamera, PixelSource, PlayerCamera, Rusterix, Value, client::widget::game::GameWidget,
};
use std::{collections::BTreeMap, path::PathBuf};
use theframework::prelude::*;
fn polygon(map: &mut Map, points: &[[f32; 2]]) -> u32 {
    map.possible_polygon.clear();
    let v = points
        .iter()
        .map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices.push(rusterix::Vertex::new(id, p[0], p[1]));
            id
        })
        .collect::<Vec<_>>();
    for i in 0..v.len() {
        map.create_linedef_manual(v[i], v[(i + 1) % v.len()]);
    }
    map.close_polygon_manual().unwrap()
}
fn place(
    map: &mut Map,
    id: Uuid,
    name: &str,
    x: f32,
    z: f32,
    yaw: f32,
    ground: bool,
    footprint: [f32; 4],
) -> u32 {
    let [x0, z0, x1, z1] = footprint;
    let sid = polygon(
        map,
        &[
            [x + x0, z + z0],
            [x + x1, z + z0],
            [x + x1, z + z1],
            [x + x0, z + z1],
        ],
    );
    let center = map.find_sector(sid).unwrap().center(map).unwrap();
    let s = map.find_sector_mut(sid).unwrap();
    s.name = name.into();
    s.layer = Some(if ground { 0 } else { 1 });
    s.properties
        .set("source", Value::Source(PixelSource::TileId(id)));
    s.properties.set("rect", Value::Bool(true));
    s.properties.set("iso_yaw", Value::Float(yaw));
    s.properties.set(
        "iso_anchor_offset",
        Value::Vec2([x - center.x, z - center.y]),
    );
    s.properties.set("iso_walkable", Value::Bool(ground));
    s.properties
        .set("iso_precise_footprint", Value::Bool(!ground));
    sid
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let imported = decode_project(&std::fs::read(
        args.get(1).ok_or("Supply a UI-imported project")?,
    )?)?;
    let output = PathBuf::from(args.get(2).ok_or("Supply the output project path")?);
    let mut project = Project::default();
    project.name = "Kenney Isometric Dungeon".into();
    project.time.hours = 12;
    project.regions[0].name = "Miniature Dungeon".into();
    // Copy actual UI import results, including collections and all original frames.
    project.tiles = imported
        .tiles
        .into_iter()
        .filter(|(_, t)| t.isometric.as_ref().is_some_and(|s| s.bitmap))
        .collect();
    project.tile_collections = imported.tile_collections;
    project.tile_board_tiles = imported.tile_board_tiles;
    let tiles = project
        .tiles
        .values()
        .map(|t| (t.alias.clone(), t.id))
        .collect::<BTreeMap<_, _>>();
    for required in [
        "stoneTile",
        "stoneWall",
        "stoneWallWindow",
        "stoneWallArchway",
        "barrels",
        "tableRoundChairs",
        "Male",
    ] {
        assert!(
            tiles.contains_key(required),
            "Import {required} through Creator first"
        );
    }
    assert!(project.isometric_bakes.is_empty());
    assert!(project.procedural_appearances.is_empty());
    project.config = r#"[game]
start_region = "Miniature Dungeon"
start_screen = "Game"
play_screen = "Game"
auto_create_player = true
auto_walk_2d = true
persistent_2d_intents = true
simulation_mode = "realtime"
collision_mode = "tile"
entity_block_mode = "never"
movement_units_per_sec = 2
target_fps = 60
game_tick_ms = 100
[viewport]
width = 1200
height = 850
grid_size = 64
[ruleset]
id = "eldiron.official"
version = "1.0.0"
source = "official"
"#
    .into();
    let map = &mut project.regions[0].map;
    *map = Map::default();
    map.name = "Miniature Dungeon".into();
    map.camera = MapCamera::TwoDIso;
    map.grid_size = 64.;
    map.subdivisions = 1.;
    // A compact cutaway courtyard: the two far walls enclose a readable room.
    // Each floor is an ordinary imported diamond, never projected a second time.
    for x in 0..7 {
        for z in 0..7 {
            let name = if (x == 1 && z == 5) || (x == 5 && z == 2) {
                "stoneMissingTiles"
            } else {
                "stone"
            };
            place(
                map,
                tiles[name],
                "Stone floor",
                x as f32 + 0.5,
                z as f32 + 0.5,
                0.,
                true,
                [-0.5, -0.5, 0.5, 0.5],
            );
        }
    }
    for x in 0..7 {
        let name = if x == 2 {
            "stoneWallWindow"
        } else if x == 4 {
            "stoneWallArchway"
        } else {
            "stoneWall"
        };
        place(
            map,
            tiles[name],
            name,
            x as f32 + 0.5,
            0.5,
            0.,
            false,
            [-0.5, -0.5, 0.5, -0.36],
        );
    }
    for z in 1..7 {
        let name = if z == 2 {
            "stoneWallWindow"
        } else if z == 4 {
            "stoneWallDoorOpen"
        } else {
            "stoneWallAged"
        };
        place(
            map,
            tiles[name],
            name,
            6.5,
            z as f32 + 0.5,
            90.,
            false,
            [0.36, -0.5, 0.5, 0.5],
        );
    }
    for (name, x, z, yaw, extent) in [
        ("barrelsStacked", 1.0, 1.25, 0., 0.34),
        ("woodenCrates", 5.65, 1.5, 90., 0.34),
        ("tableRoundItemsChairs", 5.0, 4.9, 0., 0.48),
        ("tableShortChairs", 1.5, 4.3, 90., 0.4),
        ("chestClosed", 5.8, 3.0, 90., 0.26),
        ("stoneColumnWood", 0.5, 6.5, 0., 0.16),
        ("stoneColumn", 3.5, 0.5, 0., 0.16),
        ("woodenPile", 0.8, 2.7, 0., 0.22),
    ] {
        place(
            map,
            tiles[name],
            name,
            x,
            z,
            yaw,
            false,
            [-extent, -extent, extent, extent],
        );
    }
    let center = rusterix::isometric::Projection {
        origin: Vec2::zero(),
        unit: 64.,
    }
    .screen(Vec2::new(3.5, 3.5), 0.);
    map.offset = Vec2::new(-center.x, center.y - 55.);
    let mut player = Character::new();
    player.name = "Player".into();
    player.map.name = "Player".into();
    player.data = format!(
        r#"[attributes]
player = true
visible = true
player_camera = "2d_iso"
radius = 0.18
tile_id = "{}"
start_equipped_items = []
[input]
w = "control.forward"
s = "control.backward"
a = "control.left"
d = "control.right"
up = "control.forward"
down = "control.backward"
left = "control.left"
right = "control.right"
"#,
        tiles["Male"]
    );
    let mut instance = Character::new();
    instance.name = "Player".into();
    instance.character_id = player.id;
    instance.position = Vec3::new(3.5, 0., 3.5);
    instance.orientation = Vec2::new(0., 1.);
    project.regions[0].characters.insert(instance.id, instance);
    project.characters.insert(player.id, player);
    let mut screen = Screen::new();
    screen.name = "Game".into();
    screen.map.name = "Game".into();
    screen.settings = "[layout]\nmode = \"responsive\"\n".into();
    let sid = polygon(
        &mut screen.map,
        &[[0., 0.], [1200., 0.], [1200., 850.], [0., 850.]],
    );
    screen.map.find_sector_mut(sid).unwrap().properties.set(
        "data",
        Value::Str("[ui]\nrole = \"game\"\ngrid_size = 64\n[camera]\ntype = \"2d_iso\"\n".into()),
    );
    project.screens.insert(screen.id, screen);
    let bytes = encode_project(&project)?;
    std::fs::write(&output, &bytes)?;
    let mut reopened = decode_project(&bytes)?;
    assert_eq!(reopened.tiles.len(), 74);
    assert!(reopened.isometric_bakes.is_empty());
    assert_eq!(reopened.regions[0].map.sectors.len(), 70);
    eprintln!(
        "Saved {} ({} bytes), 74 imported assets, 70 placements, no MZ dependencies",
        output.display(),
        bytes.len()
    );
    if let Some(preview) = args.get(3) {
        let preview = PathBuf::from(preview);
        std::fs::create_dir_all(&preview)?;
        rusterix_utils::insert_content_into_maps(&mut reopened);
        let mut runtime = Rusterix::new_without_audio();
        runtime.set_tiles_for_maps(reopened.tiles.clone(), false, [&reopened.regions[0].map]);
        rusterix_utils::start_server(&mut runtime, &mut reopened, false)?;
        rusterix_utils::sync_display_templates(&mut runtime.assets, &reopened, false);
        let commands = rusterix_utils::setup_client(&mut runtime, &mut reopened);
        runtime.server.process_client_commands(commands);
        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
        }
        let map = &reopened.regions[0].map;
        assert!(map.entities.iter().any(|e| e.is_player()));
        let mut widget = GameWidget::new();
        widget.set_camera_mode(PlayerCamera::D2Iso);
        widget.grid_size = 64.;
        widget.buffer = TheRGBABuffer::new(TheDim::sized(1200, 850));
        widget.graphical_draw(
            map,
            &reopened.time,
            0,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        std::fs::write(preview.join("play.png"), widget.buffer.to_png()?)?;
        let before = map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap()
            .position;
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Forward);
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
        }
        let after = reopened.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap()
            .position;
        assert!(
            after.x > before.x + 0.2 && (after.z - before.z).abs() < 0.05,
            "Iso grid movement failed: {before:?} -> {after:?}"
        );
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        runtime.client.use_editor_canvas = true;
        runtime.client.builder_d2.draw_grid = false;
        runtime.scene_handler.preview_character_setup = true;
        let mut pixels = vec![0; 1200 * 850 * 4];
        runtime.client.draw_d2(
            &reopened.regions[0].map,
            &mut pixels,
            1200,
            850,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        std::fs::write(
            preview.join("editor.png"),
            TheRGBABuffer::from(pixels, 1200, 850).to_png()?,
        )?;
        eprintln!("Native play/editor captures and server iso movement passed");
    }
    Ok(())
}
