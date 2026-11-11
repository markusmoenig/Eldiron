//! Minimal playable projects assembled from the current application defaults.
use crate::prelude::*;
use rusterix::{GeometryObject, MapCamera, PixelSource, Value};

// The working centered cursor from Hideout, bundled with the Game UI artwork.
const CURSOR_ID: &str = "77813fc5-bfeb-4b01-a3b7-9dea900b9a89";

#[derive(Clone, Copy, Debug)]
pub(crate) enum ProjectDimension {
    TwoD,
    TwoDIso,
    ThreeD,
}

/// Complete the common project defaults without loading a saved project archive.
pub(crate) fn populate(project: &mut Project, dimension: ProjectDimension) {
    let is_3d = matches!(dimension, ProjectDimension::ThreeD);
    let is_iso = matches!(dimension, ProjectDimension::TwoDIso);
    let camera = match dimension {
        ProjectDimension::ThreeD => "iso",
        ProjectDimension::TwoDIso => "2d_iso",
        ProjectDimension::TwoD => "2d",
    };
    project.time.hours = 12;
    let mut config: toml_edit::DocumentMut = project.config.parse().expect("valid bundled config");
    config["game"]["start_region"] = toml_edit::value("Region");
    config["game"]["start_screen"] = toml_edit::value("Game");
    config["game"]["play_screen"] = toml_edit::value("Game");
    config["viewport"]["cursor_id"] = toml_edit::value(CURSOR_ID);
    if is_3d {
        config["game"]["collision_mode"] = toml_edit::value("mesh");
    }
    if is_iso {
        config["game"]["auto_walk_2d"] = toml_edit::value(true);
        config["game"]["persistent_2d_intents"] = toml_edit::value(true);
    }
    project.config = config.to_string();

    let tile = |alias: &str| {
        let (id, _) = project
            .tiles
            .iter()
            .find(|(_, tile)| tile.alias == alias)
            .expect("bundled starter material is installed");
        PixelSource::TileId(*id)
    };
    let region = &mut project.regions[0];
    region.name = "Region".into();
    region.map.name = region.name.clone();
    // Region::new() supplies an editor demonstration box. Start with a floor instead.
    region.map.geometry_objects.clear();
    region.map.camera = if is_3d {
        MapCamera::ThreeDIso
    } else if is_iso {
        MapCamera::TwoDIso
    } else {
        MapCamera::TwoD
    };
    if is_3d {
        let mut floor = GeometryObject::box_from_bounds(
            "Floor",
            Vec3::new(-8.0, -0.25, -8.0),
            Vec3::new(8.0, 0.0, 8.0),
        );
        let material = tile("Street");
        for face in &mut floor.faces {
            face.tile = Some(material.clone());
        }
        region.map.geometry_objects.push(floor);
        region.editing_position_3d = Vec3::new(8.0, 8.0, 8.0);
        region.editing_look_at_3d = Vec3::zero();
        region.editing_position_iso_3d = Some(region.editing_position_3d);
        region.editing_look_at_iso_3d = Some(region.editing_look_at_3d);
    } else {
        let grass = tile("Grass");
        let street = tile("Street");
        // Use ordinary painted cells so the Rect tool can replace or erase each tile.
        for y in -4..4 {
            for x in -4..4 {
                let id = rectangle(&mut region.map, x as f32, y as f32, 1.0, 1.0);
                let cell = region.map.find_sector_mut(id).unwrap();
                cell.layer = Some(if is_iso { 0 } else { 1 });
                if is_iso {
                    cell.properties.set("iso_walkable", Value::Bool(true));
                }
                cell.properties.set("rect", Value::Bool(true));
                let source = if (-1..=0).contains(&y) {
                    &street
                } else {
                    &grass
                };
                cell.properties.set("source", Value::Source(source.clone()));
            }
        }
    }

    let mut player = Character::new();
    player.name = "Player".into();
    player.map.name = player.name.clone();
    // Inherit the current official ruleset's visuals and character defaults.
    let mut data: toml_edit::DocumentMut = include_str!("../embedded/toml/character.toml")
        .parse()
        .expect("valid bundled character data");
    data["attributes"]["player"] = toml_edit::value(true);
    data["attributes"]["player_camera"] = toml_edit::value(camera);
    data["attributes"]["race"] = toml_edit::value("Human");
    data["attributes"]["class"] = toml_edit::value("Warrior");
    for (key, action) in [
        ("w", "forward"),
        ("s", "backward"),
        ("a", "left"),
        ("d", "right"),
        ("up", "forward"),
        ("down", "backward"),
        ("left", "left"),
        ("right", "right"),
    ] {
        data["input"][key] = toml_edit::value(format!("control.{action}"));
    }
    for (key,intent) in [("u","use"),("l","look"),("o","open_close")] {
        data["input"][key] = toml_edit::value(format!("intent.{intent}"));
    }
    player.data = data.to_string();
    let mut instance = Character::new();
    instance.name = player.name.clone();
    instance.character_id = player.id;
    region.characters.insert(instance.id, instance);
    project.characters.insert(player.id, player);

    let mut screen = Screen::new();
    screen.name = "Game".into();
    screen.map.name = screen.name.clone();
    screen.settings = "[layout]\nmode = \"responsive\"\n".into();
    let width = config["viewport"]["width"].as_integer().unwrap_or(960) as f32;
    let height = config["viewport"]["height"].as_integer().unwrap_or(540) as f32;
    let grid = config["viewport"]["grid_size"]
        .as_integer()
        .unwrap_or(32)
        .max(1) as f32;
    let (left, top) = rusterix::utils::align_screen_to_grid(width, height, grid);
    // Screen geometry uses grid units, not pixels. Author the widgets with
    // pixel precision so their outlines also line up in the screen editor.
    let subdivisions = screen.map.subdivisions;
    screen.map.subdivisions = grid * 2.0;
    let widget_id = rectangle(&mut screen.map, left, top, width / grid, height / grid);
    let widget = screen.map.find_sector_mut(widget_id).unwrap();
    widget.name = "Game".into();
    widget.properties.set(
        "data",
        Value::Str(format!(
            "[ui]\nrole = \"game\"\n\n[camera]\ntype = \"{camera}\"\n"
        )),
    );
    install_toolbar(project, &mut screen, left, top, width, height, grid);
    screen.map.subdivisions = subdivisions;
    project.screens.insert(screen.id, screen);
    if is_iso {
        let errors = shared::isometric::synchronize(project);
        debug_assert!(
            errors.is_empty(),
            "Bundled iso starter materials: {errors:?}"
        );
    }
}

/// The same editable action-bar widget and nine-slice artwork as UnendingNight.
/// Command artwork stays linked to the project's ordinary MZ ruleset recipes.
fn install_toolbar(
    project: &mut Project,
    screen: &mut Screen,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    grid: f32,
) {
    let settings = include_str!("../embedded/screens/game-toolbar.toml");
    let layout: toml::Table = settings.parse().expect("valid bundled toolbar settings");
    let bar_height = layout["ui"]["slot_size"].as_integer().unwrap() as f32
        + 2.0 * layout["ui"]["padding"].as_integer().unwrap() as f32;
    let id = rectangle(
        &mut screen.map,
        left,
        top + (height - bar_height) / grid,
        width / grid,
        bar_height / grid,
    );
    let toolbar = screen.map.find_sector_mut(id).unwrap();
    toolbar.name = "Toolbar".into();
    toolbar
        .properties
        .set("source", Value::Source(PixelSource::Off));
    toolbar.properties.set("data", Value::Str(settings.into()));

    let tiles: Vec<rusterix::Tile> =
        serde_json::from_str(include_str!("../embedded/screens/game-toolbar-tiles.json"))
            .expect("valid bundled toolbar artwork");
    let mut sources = tiles.iter().map(|tile| tile.id).collect::<Vec<_>>();
    for tile in tiles {
        project.tiles.insert(tile.id, tile);
    }
    sources.extend(
        [
            "ui_preferences",
            "ui_equipment",
            "ui_inventory",
            "ui_spellbook",
            "ui_actions",
            "look",
            "use",
            "open",
            "close",
        ]
        .into_iter()
        .map(|name| shared::rulesets::material_asset_id(&format!("icon/{name}/on"))),
    );
    let mut collection = shared::project::TileCollectionAsset::new("Game UI".into());
    collection.description =
        "Editable MZ command icons and the game toolbar's nine-slice artwork".into();
    let collection_id = collection.id;
    project.add_tile_collection(collection);
    for (index, id) in sources.into_iter().enumerate() {
        let source = rusterix::TileSource::SingleTile(id);
        project.add_source_to_collection(&collection_id, source);
        project.set_collection_tile_board_position(
            &collection_id,
            source,
            Vec2::new((index % 7) as i32, (index / 7) as i32),
        );
    }
}

fn rectangle(map: &mut Map, x: f32, y: f32, width: f32, height: f32) -> u32 {
    let vertices = [
        map.add_vertex_at(x, y),
        map.add_vertex_at(x + width, y),
        map.add_vertex_at(x + width, y + height),
        map.add_vertex_at(x, y + height),
    ];
    for i in 0..4 {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
    }
    map.close_polygon_manual().expect("valid rectangle")
}
