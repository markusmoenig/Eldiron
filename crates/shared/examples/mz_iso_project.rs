//! Generate a native Eldiron MZ sprite test project, with editable source recipes.
//! cargo run -p eldiron-shared --example mz_iso_project -- generated/mz-iso2d-native
use denrim_materials::geometry::{Document, Operation};
use eldiron_shared::{
    character::Character,
    isometric::{self, Bake},
    project::Project,
    project_io::{decode_project, encode_project},
    rulesets, rusterix_utils,
    screen::Screen,
};
use rusterix::{
    Map, MapCamera, PixelSource, PlayerCamera, Rusterix, Tile, Value,
    client::widget::game::GameWidget,
};
use std::{collections::BTreeMap, path::PathBuf};
use theframework::prelude::*;

fn polygon(map: &mut Map, points: &[[f32; 2]]) -> u32 {
    map.possible_polygon.clear();
    let vertices = points
        .iter()
        .map(|p| map.add_vertex_at(p[0], p[1]))
        .collect::<Vec<_>>();
    for i in 0..vertices.len() {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % vertices.len()]);
    }
    map.close_polygon_manual().expect("Valid tile footprint")
}
fn rect(map: &mut Map, x: f32, z: f32, w: f32, d: f32) -> u32 {
    polygon(map, &[[x, z], [x + w, z], [x + w, z + d], [x, z + d]])
}
fn part(
    map: &mut Map,
    tile: Uuid,
    name: &str,
    anchor: [f32; 2],
    points: &[[f32; 2]],
    cutaway: bool,
) -> u32 {
    let id = polygon(map, points);
    let center = map.find_sector(id).unwrap().center(map).unwrap();
    let sector = map.find_sector_mut(id).unwrap();
    sector.name = name.into();
    sector.layer = Some(1);
    sector
        .properties
        .set("source", Value::Source(PixelSource::TileId(tile)));
    sector.properties.set(
        "iso_anchor_offset",
        Value::Vec2([anchor[0] - center.x, anchor[1] - center.y]),
    );
    sector.properties.set("iso_cutaway", Value::Bool(cutaway));
    id
}
fn rotate(p: [f32; 2], yaw: f32) -> [f32; 2] {
    let (s, c) = yaw.to_radians().sin_cos();
    [c * p[0] + s * p[1], -s * p[0] + c * p[1]]
}
/// Reuse the same quarter-height MZ block at each level. Only the exposed top
/// contributes a navigation surface; lower blocks fill the solid column.
fn raised_tile(map: &mut Map, tile: Uuid, name: &str, x: i32, z: i32, quarters: i32) {
    for level in 1..=quarters {
        // Independent footprint geometry permits several placements at one X/Z.
        map.possible_polygon.clear();
        let v = [[x, z], [x + 1, z], [x + 1, z + 1], [x, z + 1]].map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices
                .push(rusterix::Vertex::new(id, p[0] as f32, p[1] as f32));
            id
        });
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let s = map.find_sector_mut(id).unwrap();
        s.name = name.into();
        s.layer = Some(1);
        s.properties
            .set("source", Value::Source(PixelSource::TileId(tile)));
        s.properties
            .set("floor_height", Value::Float(level as f32 * 0.25));
        s.properties
            .set("iso_walkable", Value::Bool(level == quarters));
    }
}
fn collision(map: &mut Map, tile: Uuid, a: [f32; 2], b: [f32; 2], width: f32) {
    map.possible_polygon.clear();
    let a = map.add_vertex_at(a[0], a[1]);
    let b = map.add_vertex_at(b[0], b[1]);
    let id = map.create_linedef_manual(a, b);
    let line = map.find_linedef_mut(id).unwrap();
    line.properties
        .set("source", Value::Source(PixelSource::TileId(tile)));
    line.properties.set("wall_height", Value::Float(2.4));
    line.properties.set("wall_width", Value::Float(width));
    map.possible_polygon.clear();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("generated/mz-iso2d-native".into()),
    );
    std::fs::create_dir_all(&out)?;
    let grid = std::env::args().any(|a| a == "--grid");
    let camera_name = if grid { "2d_iso_grid" } else { "2d_iso" };
    let camera_mode = if grid {
        PlayerCamera::D2IsoGrid
    } else {
        PlayerCamera::D2Iso
    };
    let mut project = Project::default();
    let defaults = decode_project(include_bytes!(
        "../../../creator/embedded/materials/default-bakes.eldiron"
    ))?;
    eldiron_shared::ruleset_materials::seed_bakes(&mut project, &defaults)?;
    project.name = "MZ Isometric Heights".into();
    project.time.hours = 12;
    project.config = r#"[game]
start_region = "MZ Clearing"
start_screen = "Game"
play_screen = "Game"
auto_create_player = true
auto_walk_2d = true
persistent_2d_intents = true
simulation_mode = "realtime"
collision_mode = "tile"
entity_block_mode = "never"
movement_units_per_sec = 3
target_fps = 30
game_tick_ms = 250
[viewport]
width = 1000
height = 700
grid_size = 40
[ruleset]
id = "eldiron.official"
version = "1.0.0"
source = "official"
"#
    .into();
    let mut materials = BTreeMap::new();
    for (name, source) in [
        (
            "grass",
            include_str!("../../../creator/embedded/materials/01-grass.material"),
        ),
        (
            "wall",
            include_str!("../../../creator/embedded/materials/05-walls.material"),
        ),
        (
            "path",
            include_str!("../../../creator/embedded/materials/04-street.material"),
        ),
    ] {
        let id = Uuid::new_v4();
        let asset = denrim_materials::integration::EditableMaterialAsset::parse(
            &id.to_string(),
            source,
            "surface",
        )?;
        project.procedural_materials.insert(id, asset.save()?);
        materials.insert(name, id);
    }
    let mut tiles = BTreeMap::new();
    let mut add = |name: &str,
                   recipe: Uuid,
                   ground: bool,
                   yaw: f32,
                   motion: Option<String>,
                   blocking: bool| {
        let id = Uuid::new_v4();
        let mut tile = Tile::empty();
        tile.id = id;
        tile.alias = format!("MZ Iso / {name}");
        tile.blocking = blocking;
        project.tiles.insert(id, tile);
        project.isometric_bakes.insert(
            id,
            Bake {
                recipe,
                ground,
                yaw,
                directions: if motion.is_some() { 8 } else { 1 },
                frames: 16,
                motion,
                signature: 0,
            },
        );
        tiles.insert(name.to_string(), id);
    };
    add("grass", materials["grass"], true, 0., None, false);
    add("path", materials["path"], true, 0., None, false);
    drop(add);
    for (name, source) in [
        ("wall", include_str!("iso_2d/wall.mzgeometry.json")),
        ("corner", include_str!("iso_2d/corner.mzgeometry.json")),
        ("doorway", include_str!("iso_2d/doorway.mzgeometry.json")),
        ("tree", include_str!("iso_2d/tree.mzgeometry.json")),
        (
            "stone-step",
            include_str!("iso_2d/stone-step.mzgeometry.json"),
        ),
        (
            "grass-step",
            include_str!("iso_2d/grass-step.mzgeometry.json"),
        ),
    ] {
        let recipe = Uuid::new_v4();
        let mut source = source.to_string();
        for (name, id) in &materials {
            source = source.replace(&format!("experiment:{name}"), &id.to_string());
        }
        project.procedural_appearances.insert(recipe, source);
        let orientations: Vec<_> = match name {
            "wall" => vec![("wall-x".to_string(), 0.), ("wall-z".to_string(), 90.)],
            "corner" => (0..4)
                .map(|i| (format!("corner-{}", i * 90), i as f32 * 90.))
                .collect(),
            _ => vec![(name.to_string(), 0.)],
        };
        for (name, yaw) in orientations {
            let id = Uuid::new_v4();
            let mut tile = Tile::empty();
            tile.id = id;
            tile.alias = format!("MZ Iso / {name}");
            tile.blocking = !["doorway", "stone-step", "grass-step"].contains(&name.as_str());
            project.tiles.insert(id, tile);
            project.isometric_bakes.insert(
                id,
                Bake {
                    recipe,
                    ground: false,
                    yaw,
                    directions: 1,
                    motion: None,
                    frames: 1,
                    signature: 0,
                },
            );
            tiles.insert(name, id);
        }
    }
    let recipe = Uuid::new_v4();
    let body = rulesets::bundled_appearance_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .find(|a| a.id == "humanoid")
    .ok_or("Humanoid missing")?;
    let mut doc = Document::load(body.source)?;
    doc.rows
        .retain(|r| ["body", "human", "idle", "walk"].contains(&r.id.as_str()));
    if let Operation::Assemble { rows } =
        &mut doc.rows.iter_mut().find(|r| r.id == "human").unwrap().tiles[0].op
    {
        *rows = vec!["body".into()];
    }
    project
        .procedural_appearances
        .insert(recipe, serde_json::to_string_pretty(&doc)?);
    let character_tile = Uuid::new_v4();
    let mut tile = Tile::empty();
    tile.id = character_tile;
    tile.alias = "MZ Iso / character".into();
    project.tiles.insert(character_tile, tile);
    project.isometric_bakes.insert(
        character_tile,
        Bake {
            recipe,
            ground: false,
            yaw: 0.,
            directions: 8,
            motion: Some("walk".into()),
            frames: 16,
            signature: 0,
        },
    );
    let errors = isometric::synchronize(&mut project);
    assert!(errors.is_empty(), "{errors:?}");

    let map = &mut project.regions[0].map;
    *map = Map::default();
    map.name = "MZ Clearing".into();
    map.subdivisions = 100.;
    map.grid_size = 40.;
    map.camera = MapCamera::TwoDIso;
    for z in 0..18 {
        for x in 0..22 {
            let path = (x >= 2 && x <= 9 && (z == 2 || z == 8))
                || (z >= 2 && z <= 8 && (x == 2 || x == 8));
            let id = rect(map, x as f32, z as f32, 1., 1.);
            let sector = map.find_sector_mut(id).unwrap();
            sector.layer = Some(0);
            sector.properties.set("rect", Value::Bool(true));
            sector.properties.set(
                "source",
                Value::Source(PixelSource::TileId(
                    tiles[if path { "path" } else { "grass" }],
                )),
            );
        }
    }
    for x in [4.5, 5.5, 6.5] {
        for z in [3.5, 7.5] {
            if x == 5.5 && z == 7.5 {
                continue;
            }
            part(
                map,
                tiles["wall-x"],
                "Straight wall / X",
                [x, z],
                &[
                    [x - 0.5, z - 0.11],
                    [x + 0.5, z - 0.11],
                    [x + 0.5, z + 0.11],
                    [x - 0.5, z + 0.11],
                ],
                z > 7.,
            );
        }
    }
    for z in [4.5, 5.5, 6.5] {
        for x in [3.5, 7.5] {
            part(
                map,
                tiles["wall-z"],
                "Straight wall / Z",
                [x, z],
                &[
                    [x - 0.11, z - 0.5],
                    [x + 0.11, z - 0.5],
                    [x + 0.11, z + 0.5],
                    [x - 0.11, z + 0.5],
                ],
                x < 4.,
            );
        }
    }
    for (name, yaw, x, z, cut) in [
        ("corner-0", 0., 3.5, 3.5, false),
        ("corner-270", 270., 7.5, 3.5, false),
        ("corner-180", 180., 7.5, 7.5, true),
        ("corner-90", 90., 3.5, 7.5, true),
    ] {
        let points = [
            [-0.11, -0.11],
            [0.5, -0.11],
            [0.5, 0.11],
            [0.11, 0.11],
            [0.11, 0.5],
            [-0.11, 0.5],
        ]
        .map(|p| {
            let p = rotate(p, yaw);
            [x + p[0], z + p[1]]
        });
        part(map, tiles[name], name, [x, z], &points, cut);
    }
    part(
        map,
        tiles["doorway"],
        "Doorway / walk through",
        [5.5, 7.5],
        &[[5., 7.39], [6., 7.39], [6., 7.61], [5., 7.61]],
        true,
    );
    // Door jambs collide independently. The opening stays clear in the pathfinder.
    for x in [5.05, 5.95] {
        collision(map, tiles["wall-x"], [x, 7.39], [x, 7.61], 0.1);
    }
    let inside = rect(map, 3.62, 3.62, 3.76, 3.76);
    let interior = map.find_sector_mut(inside).unwrap();
    interior.name = "Room interior".into();
    interior.properties.set("iso_interior", Value::Bool(true));
    let tree = [9.5, 7.5];
    let points = (0..12)
        .map(|i| {
            let a = i as f32 / 12. * std::f32::consts::TAU;
            [tree[0] + a.cos() * 0.3, tree[1] + a.sin() * 0.3]
        })
        .collect::<Vec<_>>();
    part(map, tiles["tree"], "Tree", tree, &points, false);
    // Two-tile-wide staircase rising in +X to an eight-quarter (2-unit) terrace.
    for x in 9..17 {
        for z in 3..5 {
            raised_tile(map, tiles["stone-step"], "Stair", x, z, x - 8);
        }
    }
    for x in 17..21 {
        for z in 2..7 {
            raised_tile(map, tiles["stone-step"], "Upper terrace", x, z, 8);
        }
    }
    // A stepped grass hill: all three rings can be climbed from any side.
    for x in 4..9 {
        for z in 11..16 {
            let quarters = if x == 6 && z == 13 {
                3
            } else if (5..8).contains(&x) && (12..15).contains(&z) {
                2
            } else {
                1
            };
            raised_tile(map, tiles["grass-step"], "Grass hill", x, z, quarters);
        }
    }
    for (a, b) in [
        ([0.1, 0.1], [21.9, 0.1]),
        ([21.9, 0.1], [21.9, 17.9]),
        ([21.9, 17.9], [0.1, 17.9]),
        ([0.1, 17.9], [0.1, 0.1]),
    ] {
        collision(map, tiles["wall-x"], a, b, 0.05);
    }
    project.regions[0].map.subdivisions = 1.;
    project.regions[0].map.grid_size = 24.;
    let center = rusterix::isometric::Projection {
        origin: Vec2::zero(),
        unit: 24.,
    }
    .screen(Vec2::new(11., 9.), 0.);
    project.regions[0].map.offset = Vec2::new(-center.x, center.y);
    project.regions[0].name = "MZ Clearing".into();
    // Exercise Eldiron's actual collision/pathfinding representation, not the web harness.
    let mini = project.regions[0].map.as_mini(
        &project
            .tiles
            .iter()
            .filter(|(_, t)| t.blocking)
            .map(|(id, _)| *id)
            .collect(),
    );
    let (top, blocked) =
        mini.elevation
            .move_distance(Vec3::new(8.5, 0., 4.5), Vec2::new(10., 0.), 0.22, 0.35, 0.5);
    assert!(
        !blocked && top == Vec3::new(18.5, 2., 4.5),
        "Stairs: {top:?}"
    );
    let (edge, blocked) = mini
        .elevation
        .move_distance(top, Vec2::new(0., 4.), 0.22, 0.35, 0.5);
    assert!(
        blocked && edge.z < 6.8 && edge.y == 2.,
        "Terrace edge: {edge:?}"
    );
    let (hill, blocked) =
        mini.elevation
            .move_distance(Vec3::new(6.5, 0., 10.5), Vec2::new(0., 3.), 0.22, 0.35, 0.5);
    assert!(!blocked && hill.y == 0.75, "Hill: {hill:?}");
    let (through, blocked) = mini.move_distance(Vec2::new(5.5, 8.7), Vec2::new(0., -3.2), 0.22);
    assert!(
        !blocked && (through - Vec2::new(5.5, 5.5)).magnitude() < 0.01,
        "Doorway blocked: {through:?}"
    );
    let (stopped, blocked) = mini.move_distance(Vec2::new(4.5, 8.7), Vec2::new(0., -3.2), 0.22);
    assert!(
        blocked && stopped.y > 7.7,
        "Solid wall did not block movement: {stopped:?}"
    );
    for (start, target) in [
        (Vec2::new(5.5, 8.5), Vec2::new(5.5, 5.5)),
        (Vec2::new(5.5, 5.5), Vec2::new(5.5, 8.5)),
    ] {
        let mut pos = start;
        for _ in 0..160 {
            pos = mini.move_towards(pos, target, 0.05, 0.22, 1.).0;
        }
        assert!(
            (pos - target).magnitude() < 0.05,
            "Native path could not enter/leave room: {pos:?}"
        );
    }
    let mut player = Character::new();
    player.name = "Player".into();
    player.map.name = "Player".into();
    player.data = format!(
        r#"[attributes]
player = true
visible = true
player_camera = "{camera_name}"
radius = 0.22
tile_id = "{character_tile}"
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
"#
    );
    let mut instance = Character::new();
    instance.name = "Player".into();
    instance.character_id = player.id;
    instance.position = Vec3::new(5.5, 0., 8.7);
    instance.orientation = Vec2::new(0., -1.);
    project.regions[0].characters.insert(instance.id, instance);
    project.characters.insert(player.id, player);
    let mut screen = Screen::new();
    screen.name = "Game".into();
    screen.map.name = "Game".into();
    screen.settings = "[layout]\nmode = \"responsive\"\n".into();
    let id = rect(&mut screen.map, 0., 0., 1000., 700.);
    let widget = screen.map.find_sector_mut(id).unwrap();
    widget.name = "Game".into();
    widget.properties.set(
        "data",
        Value::Str(format!(
            "[ui]\nrole = \"game\"\ngrid_size = 40\n[camera]\ntype = \"{camera_name}\"\n"
        )),
    );
    project.screens.insert(screen.id, screen);
    let bytes = encode_project(&project)?;
    std::fs::write(out.join("MZIso2D.eldiron"), &bytes)?;
    let mut reopened = decode_project(&bytes)?;
    assert_eq!(reopened.regions[0].map.camera, MapCamera::TwoDIso);
    assert_eq!(reopened.isometric_bakes.len(), 13);
    assert!(isometric::synchronize(&mut reopened).is_empty());
    eprintln!(
        "Saved native project with room, 2-unit-high stair terrace, grass hill, and 13 source-linked MZ bakes."
    );
    if std::env::var_os("ELDIRON_ISO_NO_RENDER").is_some() {
        return Ok(());
    }
    rusterix_utils::insert_content_into_maps(&mut reopened);
    let mut runtime = Rusterix::new_without_audio();
    runtime.set_tiles_for_maps(reopened.tiles.clone(), false, [&reopened.regions[0].map]);
    rusterix_utils::start_server(&mut runtime, &mut reopened, false)?;
    rusterix_utils::sync_display_templates(&mut runtime.assets, &reopened, false);
    let commands = rusterix_utils::setup_client(&mut runtime, &mut reopened);
    assert_eq!(commands.len(), 1, "Client should register one player");
    assert_eq!(
        runtime.client.active_game_widget_camera_mode(),
        Some(camera_mode.clone())
    );
    runtime.server.process_client_commands(commands);
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
    }
    assert!(
        reopened.regions[0]
            .map
            .entities
            .iter()
            .any(|e| e.is_player())
    );
    // Iso-grid axes: A is world -Z, D is world +Z.
    for (action, inside) in [
        (rusterix::EntityAction::Left, true),
        (rusterix::EntityAction::Right, false),
    ] {
        runtime.server.local_player_action(action);
        let mut arrived = false;
        for _ in 0..220 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
            let player = reopened.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap();
            assert!(
                (player.position.x - 5.5).abs() < 0.1,
                "Isometric controls drifted from doorway"
            );
            arrived = if inside {
                player.position.z < 6.2
            } else {
                player.position.z > 8.4
            };
            if arrived {
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Native server control could not enter/leave the room"
        );
    }
    // Check the running server, including keyboard traversal, click routes, and
    // ledge blocking. Teleport only positions the test at the start of each route.
    for (start, action, target) in [
        (
            Vec2::new(8.5, 4.5),
            rusterix::EntityAction::Forward,
            Vec3::new(17.5, 2., 4.5),
        ),
        (
            Vec2::new(17.5, 4.5),
            rusterix::EntityAction::Backward,
            Vec3::new(8.5, 0., 4.5),
        ),
        (
            Vec2::new(8.5, 4.5),
            rusterix::EntityAction::TerrainClicked(Vec2::new(18.5, 4.5)),
            Vec3::new(18.5, 2., 4.5),
        ),
        (
            Vec2::new(6.5, 10.5),
            rusterix::EntityAction::TerrainClicked(Vec2::new(6.5, 13.5)),
            Vec3::new(6.5, 0.75, 13.5),
        ),
        (
            Vec2::new(6.5, 13.5),
            rusterix::EntityAction::Left,
            Vec3::new(6.5, 0., 10.5),
        ),
    ] {
        runtime.server.local_player_teleport_pos(start);
        for _ in 0..15 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
        }
        runtime.server.local_player_action(action.clone());
        let mut actual = Vec3::zero();
        let mut arrived = false;
        for _ in 0..800 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
            actual = reopened.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap()
                .position;
            arrived = (actual - target).magnitude() < 0.2;
            if arrived {
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Height traversal {camera_name} {action:?}: {actual:?}, expected {target:?}"
        );
    }
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(18.5, 5.5));
    for i in 0..120 {
        if i == 15 {
            runtime
                .server
                .local_player_action(rusterix::EntityAction::Right);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        rusterix_utils::warmup_runtime(&mut runtime, &mut reopened, 1);
    }
    runtime
        .server
        .local_player_action(rusterix::EntityAction::Off);
    let edge = reopened.regions[0]
        .map
        .entities
        .iter()
        .find(|e| e.is_player())
        .unwrap()
        .position;
    assert!(
        edge.z > 6. && edge.z < 6.8 && edge.y == 2.,
        "Server walked off ledge: {edge:?}"
    );
    let mut map = reopened.regions[0].map.clone();
    runtime.set_tiles_for_maps(reopened.tiles.clone(), false, [&map]);
    let mut widget = GameWidget::new();
    widget.set_camera_mode(camera_mode);
    widget.buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
    widget.grid_size = 40.;
    for (name, x, z) in [
        ("outside", 5.5, 8.7),
        ("doorway", 5.5, 7.5),
        ("inside", 5.5, 5.5),
        ("stairs", 13.5, 4.5),
        ("upper-terrace", 18.5, 4.5),
        ("hill", 6.5, 13.5),
    ] {
        map.entities[0].position = Vec3::new(x, mini.elevation.height_at(Vec2::new(x, z)), z);
        widget.graphical_draw(
            &map,
            &reopened.time,
            0,
            &runtime.assets,
            &mut runtime.scene_handler,
        );
        std::fs::write(out.join(format!("{name}.png")), widget.buffer.to_png()?)?;
        let p = Vec2::new(x, z);
        assert!((widget.screen_to_ground(widget.ground_to_screen(p)) - p).magnitude() < 0.0001);
    }
    // Exercise the actual stopped-editor draw path, not a plan/outline proxy.
    runtime.client.use_editor_canvas = true;
    runtime.client.builder_d2.draw_grid = true;
    runtime.scene_handler.preview_character_setup = true;
    let mut editor_pixels = vec![0; 1000 * 700 * 4];
    runtime.client.draw_d2(
        &map,
        &mut editor_pixels,
        1000,
        700,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    let mut custom_pixels = vec![0; editor_pixels.len()];
    runtime.client.draw_custom_d2(
        &map,
        &mut custom_pixels,
        1000,
        700,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    assert_eq!(editor_pixels, custom_pixels, "Editor draw paths diverged");
    std::fs::write(
        out.join("editor.png"),
        TheRGBABuffer::from(editor_pixels, 1000, 700).to_png()?,
    )?;
    // Standard MZ ruleset actors also render directly, with no prebuilt atlas binding.
    map.entities[0].attributes.remove("source");
    map.entities[0].attributes.remove("tile_id");
    map.entities[0].set_attribute("avatar", Value::Str("humanoid".into()));
    runtime.scene_handler.preview_character_setup = false;
    let live = runtime
        .scene_handler
        .isometric_avatars(&map, &runtime.assets, 0);
    assert_eq!(live.len(), 1, "Live MZ iso character missing");
    widget.graphical_draw(
        &map,
        &reopened.time,
        1,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    std::fs::write(out.join("live-mz-character.png"), widget.buffer.to_png()?)?;
    runtime.server.stop();
    eprintln!(
        "Verified native startup, room entry, stair/hill traversal, click routes, ledge blocking, height picking and source roundtrip."
    );
    Ok(())
}
