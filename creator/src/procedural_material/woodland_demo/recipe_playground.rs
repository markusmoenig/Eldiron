//! An additive layout made from the same editable recipes offered by the UI.
use super::*;
use denrim_materials::geometry::{
    Document, Operation,
    group::{Control, Modifier},
};

fn controls(doc: &mut Document) -> &mut Vec<Control> {
    let Operation::Group { controls, .. } = &mut doc.rows[0].tiles[0].op else {
        panic!()
    };
    controls
}
fn set(doc: &mut Document, id: &str, value: f32) {
    controls(doc).iter_mut().find(|c| c.id == id).unwrap().value = value;
}
fn template(source: &str, label: &str, values: &[(&str, f32)]) -> Document {
    let mut doc = Document::load(source).unwrap();
    doc.label = label.into();
    for &(id, value) in values {
        set(&mut doc, id, value);
    }
    doc
}
fn fence() -> Document {
    template(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/embedded/meshes/recipes/fence.mzgeometry.json"
        )),
        "Controls / Fence (Repeat)",
        &[],
    )
}
fn roof() -> Document {
    template(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/embedded/meshes/recipes/gable-roof.mzgeometry.json"
        )),
        "Controls / Roof panels",
        &[],
    )
}
fn install(project: &mut Project, mut doc: Document, blocking: bool) -> Uuid {
    let label = doc.label.clone();
    let id = mesh(project, &label, doc.save().unwrap(), blocking);
    doc.id = id.to_string();
    project
        .procedural_appearances
        .insert(id, doc.save().unwrap());
    id
}
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<[f32; 2]> {
    vec![[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
}
fn part(
    map: &mut Map,
    tile: Uuid,
    label: &str,
    anchor: [f32; 2],
    points: &[[f32; 2]],
    height: f32,
    yaw: f32,
    cutaway: bool,
) -> u32 {
    let id = sector(map, tile, label, points, height);
    let center = map.find_sector(id).unwrap().center(map).unwrap();
    let p = &mut map.find_sector_mut(id).unwrap().properties;
    p.set(
        "iso_anchor_offset",
        Value::Vec2([anchor[0] - center.x, anchor[1] - center.y]),
    );
    p.set("iso_yaw", Value::Float(yaw));
    p.set("iso_cutaway", Value::Bool(cutaway));
    id
}
fn expand(mut project: Project) -> (Project, [Uuid; 9]) {
    let walls = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/wall-segment.mzgeometry.json"
            )),
            "Controls / Wall segments",
            &[],
        ),
        true,
    );
    install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/doorway.mzgeometry.json"
            )),
            "Tutorial / Wide doorway",
            &[("width", 2.)],
        ),
        true,
    );
    let corner = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/wall-corner.mzgeometry.json"
            )),
            "Controls / Wall corners",
            &[],
        ),
        true,
    );
    let roof = install(&mut project, roof(), false);
    let fence = install(&mut project, fence(), true);
    let mut floor_doc = template(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/embedded/meshes/recipes/box.mzgeometry.json"
        )),
        "Controls / Room floor",
        &[
            ("width", 4.56),
            ("height", 0.08),
            ("depth", 3.56),
            ("color", 5.),
        ],
    );
    let height = controls(&mut floor_doc)
        .iter_mut()
        .find(|c| c.id == "height")
        .unwrap();
    height.label = "Thickness (m)".into();
    for m in &mut height.modifiers {
        if let Modifier::Parameter { target, factor, .. } = m {
            if target.operation.starts_with("floor-") && target.parameter == "position1" {
                *factor = 0.;
            }
            if target.operation == "solid-2" {
                *factor = -0.5;
            }
        }
    }
    let floor = install(&mut project, floor_doc, false);
    let tree = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/tree.mzgeometry.json"
            )),
            "Controls / Tree trunk",
            &[],
        ),
        true,
    );
    let doorway = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/doorway.mzgeometry.json"
            )),
            "Controls / Doorway",
            &[],
        ),
        true,
    );
    let posts = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/repeated-posts.mzgeometry.json"
            )),
            "Controls / Separate posts",
            &[("count", 3.), ("spacing", 2.)],
        ),
        true,
    );
    let ring = install(
        &mut project,
        template(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/embedded/meshes/recipes/ring-walkway.mzgeometry.json"
            )),
            "Controls / Ring walkway",
            &[],
        ),
        false,
    );
    let ids = [
        walls, corner, roof, fence, floor, tree, doorway, posts, ring,
    ];
    let mut collection =
        shared::project::TileCollectionAsset::new("Published controls playground".into());
    collection.description="Select a part and Edit mesh. Pitch keeps the roof ridge joined; fence length follows count × spacing. All calculations are editable MZ formulas.".into();
    let collection_id = collection.id;
    project.add_tile_collection(collection);
    for (i, id) in ids.iter().enumerate() {
        let source = rusterix::TileSource::SingleTile(*id);
        project.add_source_to_collection(&collection_id, source);
        project.set_collection_tile_board_position(&collection_id, source, Vec2::new(i as i32, 0));
    }
    let old_models = project
        .tiles
        .iter()
        .filter(|(_, t)| t.isometric.as_ref().is_some_and(|s| !s.ground))
        .map(|(id, _)| *id)
        .collect::<HashSet<_>>();
    let map = &mut project.regions[0].map;
    // Preserve the terrain/paint/stamp exercises; clear only nearby prop sectors.
    let remove: Vec<_> = map
        .sectors
        .iter()
        .filter(|s| {
            s.center(map)
                .is_some_and(|p| (p.x > 1.8 && p.x < 8.8 && p.y > 19.8 && p.y < 27.6) || (matches!(s.properties.get_default_source(), Some(rusterix::PixelSource::TileId(id)) if old_models.contains(id)) && [(9.5,23.),(9.,26.5),(0.5,26.5),(1.5,26.5)].iter().any(|&(x,z)| (p.x-x).abs()<1. && (p.y-z).abs()<1.)))
        })
        .filter(|s| s.properties.get_default_source().is_some())
        .map(|s| s.id)
        .collect();
    map.delete_elements(&[], &[], &remove);
    for (x, z, yaw) in [
        (3., 20.5, 0f32),
        (8., 20.5, 270.),
        (8., 24.5, 180.),
        (3., 24.5, 90.),
    ] {
        let points: Vec<_> = [
            [0., 0.],
            [1., 0.],
            [1., 0.22],
            [0.22, 0.22],
            [0.22, 1.],
            [0., 1.],
        ]
        .iter()
        .map(|p| {
            let (s, c) = yaw.to_radians().sin_cos();
            [x + c * p[0] + s * p[1], z - s * p[0] + c * p[1]]
        })
        .collect();
        part(
            map,
            corner,
            "Recipe room / Corner",
            [x, z],
            &points,
            0.,
            yaw,
            true,
        );
    }
    for x in [4.5, 5.5, 6.5] {
        part(
            map,
            walls,
            "Recipe room / Back wall",
            [x, 20.61],
            &rectangle(x - 0.5, 20.5, x + 0.5, 20.72),
            0.,
            0.,
            false,
        );
    }
    for x in [4.5, 6.5] {
        part(
            map,
            walls,
            "Recipe room / Door wall",
            [x, 24.39],
            &rectangle(x - 0.5, 24.28, x + 0.5, 24.5),
            0.,
            0.,
            true,
        );
    }
    for x in [3.11, 7.89] {
        for z in [22., 23.] {
            part(
                map,
                walls,
                "Recipe room / Side wall",
                [x, z],
                &rectangle(x - 0.11, z - 0.5, x + 0.11, z + 0.5),
                0.,
                90.,
                true,
            );
        }
    }
    let inside = part(
        map,
        floor,
        "Recipe room / Interior",
        [5.5, 22.5],
        &rectangle(3.22, 20.72, 7.78, 24.28),
        0.,
        0.,
        false,
    );
    let p = &mut map.find_sector_mut(inside).unwrap().properties;
    p.set("iso_interior", Value::Bool(true));
    p.set("iso_walkable", Value::Bool(true));
    part(
        map,
        doorway,
        "Recipe room / Doorway",
        [5.5, 24.39],
        &rectangle(4.8, 24.27, 6.2, 24.51),
        0.,
        0.,
        true,
    );
    let posts_id = part(
        map,
        posts,
        "Recipe yard / Separate posts",
        [0.5, 26.5],
        &rectangle(0.45, 26.44, 4.55, 26.56),
        0.,
        0.,
        false,
    );
    map.find_sector_mut(posts_id)
        .unwrap()
        .properties
        .set("iso_follow_mesh_bounds", Value::Bool(true));
    let ring_id = part(
        map,
        ring,
        "Recipe yard / Ring walkway",
        [2.5, 29.5],
        &rectangle(1., 28., 4., 31.),
        0.25,
        0.,
        false,
    );
    let props = &mut map.find_sector_mut(ring_id).unwrap().properties;
    props.set("iso_follow_mesh_bounds", Value::Bool(true));
    props.set("iso_walkable", Value::Bool(true));
    for sector in &mut map.sectors {
        if sector.name.starts_with("Recipe room /") && sector.id != inside {
            sector
                .properties
                .set("iso_follow_mesh_bounds", Value::Bool(true));
        }
    }
    for (x, z) in [(9.5, 23.), (9., 26.5)] {
        let id = part(
            map,
            tree,
            "Recipe yard / Trunk footprint tree",
            [x, z],
            &rectangle(x - 0.5, z - 0.5, x + 0.5, z + 0.5),
            0.,
            0.,
            false,
        );
        map.find_sector_mut(id)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(true));
    }
    // Each paired module shares controls with its two editable roof-slope groups.
    for z in [21., 22., 23., 24.] {
        part(
            map,
            roof,
            "Recipe room / Roof panel",
            [3., z],
            &rectangle(3., z - 0.5, 8., z + 0.5),
            2.4,
            0.,
            true,
        );
    }
    for x in [2.7, 6.2] {
        let id = part(
            map,
            fence,
            "Recipe yard / Repeat fence",
            [x, 27.],
            &rectangle(x, 26.94, x + 2.1, 27.06),
            0.,
            0.,
            false,
        );
        map.find_sector_mut(id)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(true));
    }
    // Connect the room entrance and fence gap to the existing MZ path spur.
    let road = *project
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Woodland / Winding Earth Path")
        .unwrap()
        .0;
    let mut cells = match map.properties.get("tiles") {
        Some(Value::TileOverrides(c)) => c.clone(),
        _ => Default::default(),
    };
    for x in 5..=10 {
        cells.insert((x, 25), rusterix::PixelSource::TileId(road));
    }
    for z in 24..=28 {
        cells.insert((5, z), rusterix::PixelSource::TileId(road));
    }
    map.properties.set("tiles", Value::TileOverrides(cells));
    map.grid_size = 42.;
    ServerContext::default().center_map_at_grid_pos(
        Vec2::new(1200., 850.),
        Vec2::new(5.5, 23.5),
        map,
    );
    map.changed = map.changed.wrapping_add(1);
    project.name = "MZ Published Controls Playground".into();
    for instance in project.regions[0].characters.values_mut() {
        instance.position = Vec3::new(5.5, 0., 26.5);
    }
    (project, ids)
}
/// Builds through the same placement helper the Rect tool uses. Approximate
/// grid clicks become exact joins; no authored per-part coordinate corrections.
fn connector_exercise(project: &mut Project, ids: &[Uuid; 9]) {
    let door = *project
        .procedural_appearances
        .iter()
        .find(|(_, s)| Document::parse(s).is_ok_and(|d| d.label == "Tutorial / Wide doorway"))
        .unwrap()
        .0;
    let tiles = &project.tiles;
    let map = &mut project.regions[0].map;
    map.properties
        .set("iso_brush_follow_mesh_bounds", Value::Bool(true));
    map.properties.set("iso_brush_snap", Value::Bool(true));
    map.properties.set("iso_brush_walkable", Value::Bool(false));
    map.properties.set("iso_brush_stack", Value::Bool(false));
    map.properties.set("iso_edit_height", Value::Float(0.));
    let mut placed = vec![];
    for (tile, x, z, yaw) in [
        (ids[1], 12., 21., 0.),
        (ids[0], 14., 21., 0.),
        (ids[0], 15., 21., 0.),
        (ids[0], 16., 21., 0.),
        (ids[0], 17., 21., 0.),
        (ids[1], 18., 21., 270.),
        (ids[0], 18., 23., 90.),
        (ids[0], 18., 24., 90.),
        (ids[1], 18., 25., 180.),
        (ids[0], 17., 25., 0.),
        (door, 15., 25., 0.),
        (ids[0], 14., 25., 0.),
        (ids[1], 12., 25., 90.),
        (ids[0], 12., 24., 90.),
        (ids[0], 12., 23., 90.),
    ] {
        map.properties.set("iso_brush_yaw", Value::Float(yaw));
        let vertices = [
            Vec2::new(x, z),
            Vec2::new(x + 1., z),
            Vec2::new(x + 1., z + 1.),
            Vec2::new(x, z + 1.),
        ];
        assert!(crate::iso_editor::paint_cell_with_tiles(
            map,
            vertices,
            Some(rusterix::PixelSource::TileId(tile)),
            false,
            0,
            tiles
        ));
        let id = map.selected_sectors[0];
        let sector = map.find_sector_mut(id).unwrap();
        sector.name = format!("Connector tutorial / Part {}", placed.len() + 1);
        sector.properties.set("iso_cutaway", Value::Bool(z >= 25.));
        if !placed.is_empty() {
            assert!(
                sector.properties.get_int("iso_snap_target").is_some(),
                "Part {} did not snap",
                placed.len() + 1
            );
        }
        placed.push(id);
    }
    // Closing seam: the last wall ends at the root corner's free Z connector.
    let root =
        shared::isometric::placement::anchor(map, map.find_sector(placed[0]).unwrap()).unwrap();
    let last = shared::isometric::placement::anchor(
        map,
        map.find_sector(*placed.last().unwrap()).unwrap(),
    )
    .unwrap();
    assert!(
        (last - Vec2::new(root.x + 0.11, root.y + 1.5)).magnitude() < 1e-4,
        "Closing seam: {root:?} -> {last:?}"
    );
    map.selected_sectors.clear();
    map.properties.set("iso_brush_yaw", Value::Float(0.));
}

fn render(project: &Project, out: &Path, name: &str, pos: [f32; 2]) {
    let mut assets = rusterix::Assets::default();
    assets.palette = project.palette.clone();
    assets.set_tiles(project.tiles.clone());
    let mut map = project.regions[0].map.clone();
    map.entities.clear();
    let mut actor = rusterix::Entity::new();
    actor.set_attribute(
        "source",
        Value::Source(rusterix::PixelSource::TileId(source_tile(
            project,
            "character",
        ))),
    );
    actor.set_attribute("player", Value::Bool(true));
    actor.position = Vec3::new(pos[0], 0., pos[1]);
    map.entities.push(actor);
    let mut projection = rusterix::isometric::Projection {
        origin: Vec2::zero(),
        unit: 64.,
    };
    let center = if name == "connector-room" {
        Vec2::new(15.5, 23.5)
    } else {
        Vec2::new(5.5, 23.5)
    };
    projection.origin = Vec2::new(600., 420.) - projection.screen(center, 0.);
    let mut image = TheRGBABuffer::new(TheDim::sized(1200, 850));
    rusterix::isometric::draw(&map, &assets, &mut image, projection, 0.);
    std::fs::write(out.join(format!("{name}.png")), image.to_png().unwrap()).unwrap();
}
fn bake_changed_materials(project: &mut Project) {
    let density = super::super::pixel_style::density(&project.config);
    let sources: Vec<_> = project
        .procedural_materials
        .iter()
        .filter(|(id, b)| {
            !shared::ruleset_materials::is_library_tile(&project.config, id)
                && project.procedural_material_cache_keys.get(*id)
                    != Some(&super::super::pixel_style::bake_key(b, density))
        })
        .map(|(id, b)| (*id, b.clone()))
        .collect();
    if sources.is_empty() {
        return;
    }
    let (device, queue) = pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        adapter.request_device(&Default::default()).await.unwrap()
    });
    let worker = super::super::gpu::Worker::new(device, queue);
    for (id, bundle) in sources {
        eprintln!(
            "Baking changed ground/material: {}",
            project.tiles[&id].alias
        );
        worker.send(super::super::gpu::Job {
            pixels_per_tile: density,
            tile: id,
            revision: 0,
            bundle: bundle.clone(),
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let done = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(180))
            .unwrap();
        let frame = done.frame.unwrap();
        let tile = project.tiles.get_mut(&id).unwrap();
        if let Some(pages) = frame.surface_pages {
            tile.textures = pages.textures.clone();
            tile.uv_pages = Some(pages.layout.clone());
        } else {
            tile.textures = vec![frame.tile];
            tile.uv_pages = None;
        }
        tile.baked_material_data = tile.textures.iter().map(|t| t.material_bytes()).collect();
        project
            .procedural_material_cache_keys
            .insert(id, super::super::pixel_style::bake_key(&bundle, density));
    }
}
fn runtime_review(project: &Project, out: &Path) {
    let mut running = project.clone();
    shared::rusterix_utils::insert_content_into_maps(&mut running);
    let mut runtime = rusterix::Rusterix::new_without_audio();
    runtime.set_tiles_for_maps(running.tiles.clone(), false, [&running.regions[0].map]);
    shared::rusterix_utils::start_server(&mut runtime, &mut running, false).unwrap();
    shared::rusterix_utils::sync_display_templates(&mut runtime.assets, &running, false);
    let commands = shared::rusterix_utils::setup_client(&mut runtime, &mut running);
    runtime.server.process_client_commands(commands);
    fn tick(runtime: &mut rusterix::Rusterix, p: &mut Project) {
        std::thread::sleep(std::time::Duration::from_millis(10));
        shared::rusterix_utils::warmup_runtime(runtime, p, 1);
    }
    for _ in 0..30 {
        tick(&mut runtime, &mut running);
    }
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(5.5, 28.5));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    for goal in [
        Vec2::new(5.5, 26.5),
        Vec2::new(5.5, 22.5),
        Vec2::new(3.5, 21.5),
        Vec2::new(5.5, 22.5),
        Vec2::new(5.5, 26.5),
        Vec2::new(5.5, 28.5),
    ] {
        runtime
            .server
            .local_player_action(rusterix::EntityAction::TerrainClicked(goal));
        let mut arrived = false;
        for _ in 0..350 {
            tick(&mut runtime, &mut running);
            let player = running.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap();
            assert!(
                player.position.y.abs() < 0.003,
                "Feet left the room/yard ground: {:?}",
                player.position
            );
            if (Vec2::new(player.position.x, player.position.z) - goal).magnitude() < 0.16 {
                arrived = true;
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Could not walk through door/fence gap toward {goal:?}"
        );
    }
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(9.5, 25.5));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    let goal = Vec2::new(9.5, 27.5);
    runtime
        .server
        .local_player_action(rusterix::EntityAction::TerrainClicked(goal));
    let mut arrived = false;
    for _ in 0..350 {
        tick(&mut runtime, &mut running);
        let player = running.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        if (Vec2::new(player.position.x, player.position.z) - goal).magnitude() < 0.16 {
            arrived = true;
            break;
        }
    }
    runtime
        .server
        .local_player_action(rusterix::EntityAction::Off);
    assert!(
        arrived,
        "Could not walk beneath the canopy beside its trunk"
    );
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(1.5, 25.5));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    runtime
        .server
        .local_player_action(rusterix::EntityAction::TerrainClicked(Vec2::new(1.5, 27.5)));
    let mut passed = false;
    for _ in 0..350 {
        tick(&mut runtime, &mut running);
        let e = running.regions[0]
            .map
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        if (Vec2::new(e.position.x, e.position.z) - Vec2::new(1.5, 27.5)).magnitude() < 0.16 {
            passed = true;
            break;
        }
    }
    runtime
        .server
        .local_player_action(rusterix::EntityAction::Off);
    assert!(passed, "Could not walk between disconnected posts");
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(0.5, 29.5));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    for (x, height) in [(1.5, 0.25), (2.5, 0.), (3.5, 0.25), (4.5, 0.)] {
        let goal = Vec2::new(x, 29.5);
        runtime
            .server
            .local_player_action(rusterix::EntityAction::TerrainClicked(goal));
        let mut arrived = false;
        for _ in 0..350 {
            tick(&mut runtime, &mut running);
            let e = running.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap();
            if (Vec2::new(e.position.x, e.position.z) - goal).magnitude() < 0.16 {
                assert!(
                    (e.position.y - height).abs() < 0.01,
                    "Ring/hole support wrong at {goal:?}: {:?}",
                    e.position
                );
                arrived = true;
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Could not traverse raised ring and its open center"
        );
    }
    runtime.server.local_player_teleport_pos(Vec2::new(15.5,27.5));
    for _ in 0..20 { tick(&mut runtime,&mut running); }
    for goal in [Vec2::new(15.5,23.5),Vec2::new(15.5,27.5)] {
        runtime.server.local_player_action(rusterix::EntityAction::TerrainClicked(goal));
        let mut arrived=false;
        for _ in 0..350 {
            tick(&mut runtime,&mut running);
            let e=running.regions[0].map.entities.iter().find(|e|e.is_player()).unwrap();
            if (Vec2::new(e.position.x,e.position.z)-goal).magnitude()<0.16 { arrived=true; break; }
        }
        runtime.server.local_player_action(rusterix::EntityAction::Off);
        assert!(arrived,"Could not enter/leave connector-built room at {goal:?}");
    }
    eprintln!("Verified runtime entry/exit through the connector-built room.");
    eprintln!("Verified doorway, separate-post gaps, and step up/down across the ring's hole.");
    eprintln!("Verified native movement into the concave room corner and beneath the canopy.");
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(5.5, 22.5));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    let mut game = rusterix::client::widget::game::GameWidget::new();
    game.set_camera_mode(rusterix::PlayerCamera::D2Iso);
    game.grid_size = 56.;
    game.buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
    game.graphical_draw(
        &running.regions[0].map,
        &running.time,
        0,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    std::fs::write(
        out.join("playing-inside.png"),
        game.buffer.to_png().unwrap(),
    )
    .unwrap();
    runtime.server.stop();
    eprintln!("Verified runtime entry/exit through the room door and Repeat-fence gap.");
}
fn footprint_controls_review(project: &Project, fence: Uuid) {
    let mut p = project.clone();
    let id = p.regions[0]
        .map
        .sectors
        .iter()
        .find(|s| s.name == "Recipe yard / Repeat fence")
        .unwrap()
        .id;
    p.regions[0].map.selected_sectors = vec![id];
    let mut server = ServerContext::default();
    server.curr_region = p.regions[0].id;
    server.pc = ProjectContext::Region(server.curr_region);
    server.editor_view_mode = EditorViewMode::D2;
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1000, 700, 1.);
    ui.init(&mut ctx);
    let toggle = |enabled, p: &mut Project, ctx: &mut TheContext| {
        assert!(crate::iso_editor::handle_event(
            &TheEvent::ValueChanged(
                TheId::named("Iso Placement Bounds"),
                TheValue::Bool(enabled)
            ),
            p,
            &server,
            ctx
        ));
    };
    toggle(false, &mut p, &mut ctx);
    let original = p.regions[0]
        .map
        .find_sector(id)
        .unwrap()
        .bounding_box(&p.regions[0].map);
    let mut d = Document::load(&p.procedural_appearances[&fence]).unwrap();
    set(&mut d, "count", 8.);
    set(&mut d, "spacing", 0.45);
    p.procedural_appearances.insert(fence, d.save().unwrap());
    assert!(shared::isometric::synchronize(&mut p).is_empty());
    assert_eq!(
        p.regions[0]
            .map
            .find_sector(id)
            .unwrap()
            .bounding_box(&p.regions[0].map),
        original
    );
    toggle(true, &mut p, &mut ctx);
    let width = |p: &Project| {
        let b = p.regions[0]
            .map
            .find_sector(id)
            .unwrap()
            .bounding_box(&p.regions[0].map);
        b.max.x - b.min.x
    };
    assert!((width(&p) - 3.6).abs() < 1e-4);
    crate::editor::UNDOMANAGER
        .write()
        .unwrap()
        .undo(&mut server, &mut p, &mut ui, &mut ctx);
    assert!((width(&p) - 2.1).abs() < 1e-4);
    crate::editor::UNDOMANAGER
        .write()
        .unwrap()
        .redo(&mut server, &mut p, &mut ui, &mut ctx);
    assert!((width(&p) - 3.6).abs() < 1e-4);
    eprintln!("Verified Follow recipe shape control, freeze, enable and real map undo/redo.");
}
#[test]
#[ignore = "builds the playable recipe-controls project and native runtime review; outputs stay in generated/"]
fn build_iso_published_controls_playground() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let input = root.join("generated/iso-organic-stamps/OrganicTerrain.eldiron");
    let input = if input.exists() {
        input
    } else {
        root.join("generated/mz-iso2d-terrain-playground/MZTerrainPlayground.eldiron")
    };
    let out = root.join("generated/mz-iso2d-recipe-playground");
    std::fs::create_dir_all(&out).unwrap();
    let (mut project, ids) =
        expand(shared::project_io::decode_project(&std::fs::read(input).unwrap()).unwrap());
    let errors = super::super::composition::prepare_project(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    bake_changed_materials(&mut project);
    let errors = shared::isometric::synchronize(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    connector_exercise(&mut project, &ids);
    // All shared placements point to ordinary source recipes, including rotated outputs.
    for id in ids {
        assert!(
            matches!(shared::isometric::tile_recipe(&project,id),Some(shared::isometric::TileRecipe::Geometry(r)) if r==id)
        );
    }
    let mini = project.regions[0].map.as_mini(
        &project
            .tiles
            .iter()
            .filter(|(_, t)| t.blocking)
            .map(|(id, _)| *id)
            .collect(),
    );
    for p in [
        Vec2::new(2.5, 20.5),
        Vec2::new(7.5, 24.5),
        Vec2::new(5.5, 26.5),
    ] {
        eprintln!("Room terrain at {p:?}: {}", mini.elevation.height_at(p));
    }
    for (start, delta) in [
        (Vec3::new(5.5, 0., 26.5), Vec2::new(0., -4.)),
        (Vec3::new(5.5, 0., 22.), Vec2::new(0., 6.)),
    ] {
        let (end, blocked) = mini.move_distance(Vec2::new(start.x, start.z), delta, 0.22);
        assert!(!blocked, "Door/fence gap blocked: {end:?}");
    }
    let map = &project.regions[0].map;
    let doorway_sector = map
        .sectors
        .iter()
        .find(|s| s.name == "Recipe room / Doorway")
        .unwrap();
    assert_eq!(
        doorway_sector
            .footprint_topology
            .as_ref()
            .unwrap()
            .regions
            .len(),
        2
    );
    let posts_sector = map
        .sectors
        .iter()
        .find(|s| s.name == "Recipe yard / Separate posts")
        .unwrap();
    assert_eq!(
        posts_sector
            .footprint_topology
            .as_ref()
            .unwrap()
            .regions
            .len(),
        3
    );
    let ring_sector = map
        .sectors
        .iter()
        .find(|s| s.name == "Recipe yard / Ring walkway")
        .unwrap();
    assert_eq!(
        ring_sector.footprint_topology.as_ref().unwrap().regions[0]
            .holes
            .len(),
        1
    );
    assert!(!ring_sector.is_inside(map, Vec2::new(2.5, 29.5)));
    assert!((mini.elevation.height_at(Vec2::new(1.5, 29.5)) - 0.25).abs() < 1e-4);
    assert!(mini.elevation.height_at(Vec2::new(2.5, 29.5)).abs() < 1e-4);
    render(&project, &out, "outside", [5.5, 26.5]);
    render(&project, &out, "inside", [5.5, 22.5]);
    render(&project, &out, "connector-room", [15.5, 24.]);
    // Exercise parameter edits on copies so the exported scene starts in a joined layout.
    let mut edited = project.clone();
    for (id, key, value) in [
        (ids[0], "depth", 0.35),
        (ids[1], "thickness", 0.35),
        (ids[2], "slope", 45.),
        (ids[3], "count", 8.),
        (ids[3], "spacing", 0.45),
        (ids[5], "trunk-width", 0.55),
    ] {
        let mut doc = Document::load(&edited.procedural_appearances[&id]).unwrap();
        set(&mut doc, key, value);
        edited
            .procedural_appearances
            .insert(id, doc.save().unwrap());
    }
    let errors = shared::isometric::synchronize(&mut edited);
    assert!(errors.is_empty(), "{errors:?}");
    for id in ids.iter().take(4) {
        assert_ne!(
            project.isometric_bakes[id].signature,
            edited.isometric_bakes[id].signature
        );
        assert_ne!(project.tiles[id].isometric, edited.tiles[id].isometric);
    }
    assert_eq!(
        Document::load(&edited.procedural_appearances[&ids[3]])
            .unwrap()
            .evaluate()
            .unwrap()
            .mesh
            .parts
            .len(),
        10
    );
    let fence_placements = |p: &Project| {
        p.regions[0]
            .map
            .sectors
            .iter()
            .filter(|s| s.name == "Recipe yard / Repeat fence")
            .map(|s| (s.id, s.bounding_box(&p.regions[0].map)))
            .collect::<Vec<_>>()
    };
    for ((id, old), (new_id, new)) in fence_placements(&project)
        .into_iter()
        .zip(fence_placements(&edited))
    {
        assert_eq!(id, new_id);
        assert!((old.max.x - old.min.x - 2.1).abs() < 1e-4);
        assert!((new.max.x - new.min.x - 3.6).abs() < 1e-4);
        assert!((old.min.x - new.min.x).abs() < 1e-4);
    }
    let edited_mini = edited.regions[0].map.as_mini(
        &edited
            .tiles
            .iter()
            .filter(|(_, t)| t.blocking)
            .map(|(id, _)| *id)
            .collect(),
    );
    let (_, blocked) = edited_mini.move_distance(Vec2::new(5.5, 26.5), Vec2::new(0., 1.5), 0.22);
    assert!(
        blocked,
        "Longer fence must close the old gap in collision as well as art"
    );
    render(&edited, &out, "edited-controls", [5.5, 26.5]);
    let bytes = shared::project_io::encode_project(&project).unwrap();
    let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
    assert!(shared::isometric::synchronize(&mut reopened).is_empty());
    for id in ids {
        assert_eq!(
            project.procedural_appearances[&id],
            reopened.procedural_appearances[&id]
        );
        assert_eq!(project.tiles[&id].isometric, reopened.tiles[&id].isometric);
    }
    footprint_controls_review(&project, ids[3]);
    runtime_review(&project, &out);
    std::fs::write(out.join("MZRecipePlayground.eldiron"), bytes).unwrap();
    eprintln!(
        "Published controls playground: {}",
        out.join("MZRecipePlayground.eldiron").display()
    );
}

#[test]
#[ignore = "exports scene documentation captures from the existing generated playground"]
fn materialz_documentation_scene_captures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/mz-iso2d-recipe-playground");
    let project = shared::project_io::decode_project(
        &std::fs::read(out.join("MZRecipePlayground.eldiron")).unwrap(),
    )
    .unwrap();
    render(&project, &out, "connector-room", [15.5, 26.5]);
}
