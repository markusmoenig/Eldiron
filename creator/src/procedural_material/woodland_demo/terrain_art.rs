//! Repair and review the maintained terrain example through the normal MZ
//! composition/bake pipeline. No image or material cache is an authored input.
use super::*;
use rusterix::PixelSource;

const ROCK: &str = "38af3bce-0b84-45a0-a080-100000000001";
const MASKS: &str = "38af3bce-0b84-45a0-a080-100000000002";
const TERRAIN: &str = "38af3bce-0b84-45a0-a080-100000000003";
fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}
fn install(project: &mut Project) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let assets = root.join("crates/shared/examples/iso_2d/uo_art");
    let rock = id(ROCK);
    let masks = id(MASKS);
    let terrain = id(TERRAIN);
    let grass = id("e8f4509f-086c-4ab3-b8c3-a502259ba436");
    let earth = id("5eae73af-197b-46ba-9425-f7e9403498b1");
    let colors: Vec<_> = project
        .palette
        .colors
        .iter()
        .map(|c| c.clone().unwrap())
        .collect();
    let nearest =
        |hex: &str| shared::palette::closest(&TheColor::from_hex(hex), &colors).unwrap() as u16;
    material_asset(
        project,
        rock,
        &std::fs::read_to_string(assets.join("hill-rock.material")).unwrap(),
        &[
            ("colors", "color0", nearest("#393e47")),
            ("colors", "color1", nearest("#555953")),
            ("colors", "color2", nearest("#73776a")),
            ("colors", "color3", nearest("#958d73")),
        ],
    );
    // Discard the earlier experimental mask asset entirely.
    project.procedural_materials.shift_remove(&masks);
    project.procedural_material_cache_keys.shift_remove(&masks);
    project.tiles.shift_remove(&masks);
    project.tile_board_tiles.shift_remove(&masks);
    project.remove_source_from_collections(rusterix::TileSource::SingleTile(masks));
    for (uuid, name) in [
        (rock, "Terrain / Weathered hill rock"),
        (terrain, "Terrain / Rocky hills and winding trail"),
    ] {
        let tile = project
            .tiles
            .entry(uuid)
            .or_insert_with(rusterix::Tile::empty);
        tile.id = uuid;
        tile.alias = name.into();
    }
    // Geometry TOML is the authored source; the region derives its own MZ outputs.
    project.material_compositions.shift_remove(&terrain);
    project.procedural_materials.shift_remove(&terrain);
    project
        .procedural_material_cache_keys
        .shift_remove(&terrain);
    project.tiles.shift_remove(&terrain);
    project.tile_board_tiles.shift_remove(&terrain);
    project.remove_source_from_collections(rusterix::TileSource::SingleTile(terrain));
    let map = &mut project.regions[0].map;
    for sector in &mut map.sectors {
        if !sector.properties.get_bool_default("terrain_example", false) {
            continue;
        }
        let p = &mut sector.properties;
        p.set(
            "terrain_material_source",
            Value::Source(PixelSource::TileId(rock)),
        );
        for (key, value) in [
            ("inset", 0.5),
            ("soft_edge", 1.0),
            ("noisy_edge", 0.28),
            ("noise_scale", 2.7),
        ] {
            p.set(&format!("terrain_material_{key}"), Value::Float(value));
        }
    }
    for line in &mut map.linedefs {
        if !line.properties.get_bool_default("terrain_example", false) {
            continue;
        }
        let p = &mut line.properties;
        p.set(
            "terrain_material_source",
            Value::Source(PixelSource::TileId(earth)),
        );
        p.set(
            "terrain_material_verge_source",
            Value::Source(PixelSource::TileId(grass)),
        );
        for (key, value) in [
            ("width", 1.35),
            ("verge_width", 3.2),
            ("soft_edge", 0.45),
            ("noisy_edge", 0.13),
            ("noise_scale", 2.7),
        ] {
            p.set(&format!("terrain_material_{key}"), Value::Float(value));
        }
        p.set("terrain_material_order", Value::Int(10));
    }
    let mut tiles = match map.properties.get("tiles") {
        Some(Value::TileOverrides(t)) => t.clone(),
        _ => Default::default(),
    };
    let mut blends = match map.properties.get("blend_tiles") {
        Some(Value::BlendOverrides(t)) => t.clone(),
        _ => Default::default(),
    };
    // Remove only the earlier demo's coarse Earth paint, preserving the inn,
    // original AutoRoad placements, terrain guides and user-authored geometry.
    for z in 10..30 {
        for x in 11..30 {
            if tiles.get(&(x, z)) == Some(&PixelSource::TileId(earth)) {
                tiles.remove(&(x, z));
            }
            if blends
                .get(&(x, z))
                .is_some_and(|(_, s)| *s == PixelSource::TileId(earth))
            {
                blends.remove(&(x, z));
            }
            if x >= 12
                && tiles.get(&(x, z)).is_none_or(|s| {
                    *s == PixelSource::TileId(grass) || *s == PixelSource::TileId(terrain)
                })
            {
                tiles.insert((x, z), PixelSource::TileId(grass));
            }
        }
    }
    map.properties.set("tiles", Value::TileOverrides(tiles));
    map.properties
        .set("blend_tiles", Value::BlendOverrides(blends));
    map.changed = map.changed.wrapping_add(1);
    let collection = project
        .tile_collections
        .values()
        .find(|c| c.name == "Wayfarer's Rest / MZ parts")
        .unwrap()
        .id;
    for tile in [rock] {
        project.add_source_to_collection(&collection, rusterix::TileSource::SingleTile(tile));
    }
}

#[test]
#[ignore = "updates the maintained project using native MZ GPU bakes; captures stay in generated/"]
fn update_unending_night_terrain_art() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let file = root.join("test_projects/UnendingNight.eldiron");
    let out = root.join("generated/unending-night/terrain-art");
    std::fs::create_dir_all(&out).unwrap();
    let input = if std::env::var_os("ELDIRON_TERRAIN_REVIEW_CANDIDATE").is_some() {
        out.join("UnendingNight.terrain-art-candidate.eldiron")
    } else {
        file.clone()
    };
    let bytes = std::fs::read(&input).unwrap();
    let backup = out.join("UnendingNight.before-terrain-art.eldiron");
    if !backup.exists() {
        std::fs::write(backup, &bytes).unwrap();
    }
    let mut project = shared::project_io::decode_project(&bytes).unwrap();
    install(&mut project);
    let errors = super::super::composition::prepare_project(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    let (device, queue) = pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        adapter.request_device(&Default::default()).await.unwrap()
    });
    let worker = super::super::gpu::Worker::new(device, queue);
    // Refresh all composition world pages after paint changes; the source
    // grass/earth pixel density remains the same 64 texels per map unit.
    let sources: Vec<_> = project
        .procedural_materials
        .iter()
        .map(|(id, s)| (*id, s.clone()))
        .collect();
    for (id, bundle) in sources {
        if project.procedural_material_cache_keys.get(&id)
            == Some(&super::super::pixel_style::bake_key(&bundle, 64))
        {
            continue;
        }
        eprintln!("Baking {}", project.tiles[&id].alias);
        worker.send(super::super::gpu::Job {
            pixels_per_tile: 64,
            tile: id,
            revision: 0,
            bundle: bundle.clone(),
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(180))
            .unwrap()
            .frame
            .unwrap();
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
            .insert(id, super::super::pixel_style::bake_key(&bundle, 64));
    }
    let bytes = shared::project_io::encode_project(&project).unwrap();
    let project = shared::project_io::decode_project(&bytes).unwrap();
    std::fs::write(
        out.join("UnendingNight.terrain-art-candidate.eldiron"),
        &bytes,
    )
    .unwrap();
    check_surfaces(&project);
    let character = source_tile(&project, "character");
    if std::env::var_os("ELDIRON_TERRAIN_REVIEW_CANDIDATE").is_none() {
        for (name, unit, center, hour) in [
            ("terrain-day", 36., [15., 15.], 11.),
            ("hill-day", 62., [23., 20.], 11.),
            ("hill-evening", 62., [23., 20.], 17.),
        ] {
            uo_art::render_at_hour(
                &project,
                character,
                &out,
                name,
                Vec3::new(12., 0., 12.3),
                unit,
                Vec2::from(center),
                hour,
            );
        }
    }
    std::fs::write(file, bytes).unwrap();
}
fn check_surfaces(project: &Project) {
    assert!(!project.tiles.contains_key(&id(MASKS)));
    assert!(!project.procedural_materials.contains_key(&id(MASKS)));
    assert!(!project.tiles.contains_key(&id(TERRAIN)));
    let map = &project.regions[0].map;
    let base = id("e8f4509f-086c-4ab3-b8c3-a502259ba436");
    let output = rusterix::chunkbuilder::terrain_material::outputs(map)[&base];
    assert!(
        project.material_compositions[&output]
            .layers
            .iter()
            .all(|l| l.mask.is_none() && l.geometry.is_some())
    );
    // Check actual persisted world pages, not source-only recipe assertions.
    let tile = &project.tiles[&output];
    let asset = EditableMaterialAsset::reopen(&project.procedural_materials[&output]).unwrap();
    let program = denrim_materials::compile(&format::write(
        &asset.resolved(&InstanceRecord::new("continuity")).unwrap(),
    ))
    .unwrap();
    let eval = denrim_materials::cpu::Evaluator::new(&program);
    let mut red = 0u64;
    let mut green = 0u64;
    let map = &project.regions[0].map;
    let geometry = project.material_compositions[&output]
        .layers
        .last()
        .unwrap()
        .geometry
        .as_ref()
        .unwrap();
    for line in &geometry.linedefs {
        let line = map.find_linedef(*line).unwrap();
        let a = map.get_vertex(line.start_vertex).unwrap();
        let b = map.get_vertex(line.end_vertex).unwrap();
        for i in 0..=100 {
            let p = a + (b - a) * (i as f32 / 100.);
            let (color, material) = tile.sample_surface_2d(p.into_array(), 0);
            assert!(
                material[0] > 0.8 && color[3] == 255,
                "broken trail at {p:?}: {color:?} {material:?}"
            );
            assert!(
                eval.sample_tile(&asset.selected_material, "shape", 0, p.into_array(), 0.001)
                    .unwrap()
                    .color[0]
                    > 0.99,
                "shape gap at {p:?}"
            );
            red += color[0] as u64;
            green += color[1] as u64;
        }
    }
    // Palette dithering may use an olive texel within dark earth. Check its
    // overall brown colour rather than forbidding individual palette entries.
    assert!(
        red > green + 5000,
        "the trail must remain earth-coloured overall"
    );
    let (_, rock) = tile.sample_surface_2d([22., 23.], 0);
    assert!(rock[0] > 0.25 && rock[0] < 0.65, "rock roughness: {rock:?}");
    assert_eq!(rock[1], 0., "stone must remain nonmetallic");
    // Fine mineral/grass grain intentionally changes between neighbouring
    // texels. Test page address continuity separately from albedo noise.
    assert!(
        tile.uv_pages
            .as_ref()
            .is_some_and(|p| p.size[0] > 1 && p.size[1] > 1)
    );
}

#[test]
fn hill_rock_is_an_editable_mz_material() {
    let rock = EditableMaterialAsset::parse(
        "hill-rock",
        include_str!("../../../../crates/shared/examples/iso_2d/uo_art/hill-rock.material"),
        "surface",
    )
    .unwrap();
    rock.validate().unwrap();
    let view = super::super::view(&rock).unwrap();
    assert!(view.rows.iter().any(|r| r.id == "roughness"));
}

#[test]
#[ignore = "captures native geometry TOML widgets; images stay in generated/"]
fn capture_geometry_terrain_toml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/unending-night/terrain-art");
    let mut project = shared::project_io::decode_project(&std::fs::read(root.join("test_projects/UnendingNight.eldiron")).unwrap()).unwrap();
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1500, 960, 1.);
    ui.set_theme(Box::new(TheBrownTheme::new()), &mut ctx);
    ui.init(&mut ctx);
    let mut server = ServerContext::default();
    server.curr_region = project.regions[0].id;
    server.pc = ProjectContext::Region(server.curr_region);
    server.curr_map_tool_type = rusterix::MapToolType::Linedef;
    let line = project.regions[0].map.linedefs.iter().find(|l| l.properties.get_bool_default("terrain_example", false)).unwrap().id;
    project.regions[0].map.selected_linedefs = vec![line];
    project.regions[0].map.selected_sectors.clear();
    let mut sidebar = crate::sidebar::Sidebar::new();
    sidebar.init_ui(&mut ui, &mut ctx, &mut server);
    server.curr_action_id = Some(Uuid::parse_str(crate::actions::edit_linedef::EDIT_LINEDEF_ACTION_ID).unwrap());
    {
        let mut actions = crate::editor::ACTIONLIST.write().unwrap();
        actions.get_action_by_id_mut(server.curr_action_id.unwrap()).unwrap().load_params(&project.regions[0].map);
    }
    sidebar.handle_event(&TheEvent::Custom(TheId::named("Refresh Action Parameters"), TheValue::Empty), &mut ui, &mut ctx, &mut project, &mut server);
    ui.get_stack_layout("Sidebar Page Stack").unwrap().set_index(1);
    let shared = ui.get_sharedvlayout("Sidebar Map Shared").unwrap();
    shared.limiter_mut().set_max_width(580);
    shared.set_mode(TheSharedVLayoutMode::Top);
    ui.get_sharedvlayout("Sidebar Actions Shared").unwrap().set_mode(TheSharedVLayoutMode::Bottom);
    ctx.ui.relayout = true;
    let mut center = TheCanvas::new();
    center.set_widget(TheRenderView::new(TheId::named("Terrain Documentation View")));
    ui.canvas.set_center(center);
    let mut pixels = vec![0; 1500 * 960 * 4];
    ui.draw(&mut pixels, &mut ctx);
    let data = std::fs::read(out.join("hill-day.png")).unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(&data)).read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    let image = TheRGBABuffer::from(rgba[..info.buffer_size()].to_vec(), info.width, info.height);
    let view = ui.get_render_view("Terrain Documentation View").unwrap();
    let dim = *view.dim();
    *view.render_buffer_mut() = image.scaled(dim.width, dim.height);
    ui.draw(&mut pixels, &mut ctx);
    std::fs::write(out.join("geometry-terrain-toml.png"), TheRGBABuffer::from(pixels, 1500, 960).to_png().unwrap()).unwrap();
}
