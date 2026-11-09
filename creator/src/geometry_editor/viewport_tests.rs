use super::*;
/// Same central stack/sidebar split as Creator, without starting the game renderer.
pub(super) fn fixture(width: i32, height: i32) -> (TheUI, TheContext) {
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(width as usize, height as usize, 1.);
    ui.style.set_theme(Box::new(TheBlackBlueTheme::new()));
    crate::procedural_material::register_preview_icons(&mut ctx, ui.style.theme().as_ref());
    let mut pages = TheStackLayout::new(TheId::named("Sidebar Page Stack"));
    for _ in 0..crate::procedural_material::SIDEBAR_PAGE {
        pages.add_canvas(TheCanvas::new());
    }
    pages.add_canvas(workspace(TheCanvas::new()));
    pages.set_index(crate::procedural_material::SIDEBAR_PAGE);
    let mut side = TheCanvas::new();
    side.set_layout(pages);
    let mut mini = TheCanvas::new();
    mini.set_widget(TheRenderView::new(TheId::named("MiniMap")));
    let mut shared = TheSharedVLayout::new(TheId::named("Sidebar Map Shared"));
    shared.limiter_mut().set_max_width(380);
    shared.set_shared_ratio(0.78);
    shared.set_mode(TheSharedVLayoutMode::Shared);
    shared.add_canvas(side);
    shared.add_canvas(crate::procedural_material::minimap_canvas(mini));
    let mut sidebar = TheCanvas::new();
    sidebar.set_layout(shared);
    ui.canvas.set_right(sidebar);
    let mut map = TheCanvas::new();
    map.set_widget(TheRenderView::new(TheId::named("PolyView")));
    let mut editor = TheStackLayout::new(TheId::named("Editor Stack"));
    editor.add_canvas(map);
    editor.add_canvas(TheCanvas::new());
    let mut normal = TheCanvas::new();
    normal.set_layout(editor);
    let mut docks = TheCanvas::new();
    let mut title = TheText::new(TheId::named("Map dock marker"));
    title.set_text("Tilepicker".into());
    title.limiter_mut().set_max_height(160);
    docks.set_widget(title);
    normal.set_bottom(docks);
    ui.canvas.set_center(main_canvas(normal));
    (ui, ctx)
}
fn human(p: &mut Project) -> (Uuid, Document) {
    let d = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
    let id = Uuid::new_v4();
    p.procedural_appearances.insert(id, d.save().unwrap());
    (id, d)
}

#[test]
#[ignore = "requires native GPU for the actual editor texture publication path"]
fn isometric_model_recipe_edits_refresh_all_tiles_and_the_scene_automatically() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    p.config = "[ruleset]\nsource='project'\n".into();
    let recipe = Uuid::new_v4();
    p.procedural_appearances.insert(recipe,r#"{"version":1,"id":"wall","label":"Wall","rows":[{"id":"wall","label":"Wall","tiles":[{"id":"box","op":{"type":"box","size":[1,0.5,0.22]}}]}],"output":"wall"}"#.into());
    let outputs = [Uuid::new_v4(), Uuid::new_v4()];
    let map = &mut p.regions[0].map;
    *map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.grid_size = 40.;
    for (i, output) in outputs.iter().enumerate() {
        let x = i as f32 * 2.;
        map.possible_polygon.clear();
        let v =
            [[x, 0.], [x + 1., 0.], [x + 1., 1.], [x, 1.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for k in 0..4 {
            map.create_linedef_manual(v[k], v[(k + 1) % 4]);
        }
        let sector = map.close_polygon_manual().unwrap();
        map.find_sector_mut(sector).unwrap().properties.set(
            "source",
            rusterix::Value::Source(rusterix::PixelSource::TileId(*output)),
        );
        let mut tile = rusterix::Tile::empty();
        tile.id = *output;
        tile.alias = format!("Wall {i}");
        tile.blocking = true;
        p.tiles.insert(*output, tile);
        p.isometric_bakes.insert(
            *output,
            shared::isometric::Bake {
                recipe,
                ground: false,
                yaw: i as f32 * 90.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
    }
    let mut server = ServerContext::default();
    server.curr_region = p.regions[0].id;
    let mut geometry = GeometryEditor::default();
    let mut publisher = crate::procedural_material::ProceduralMaterialEditor::default();
    let picked = TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(outputs[1]));
    geometry.handle_event(&picked, &mut p, &mut ui, &mut ctx);
    publisher.handle_event(&picked, &mut ui, &mut ctx, &mut p, &mut server);
    publisher.update(&mut ui, &mut ctx, &mut p, &mut server);
    let old = outputs.map(|id| p.tiles[&id].clone());
    let draw = |p: &Project| {
        let mut pixels = vec![0; 400 * 300 * 4];
        RUSTERIX
            .write()
            .unwrap()
            .draw_d2(&p.regions[0].map, &mut pixels, 400, 300);
        pixels
    };
    let before = draw(&p);
    geometry
        .apply_command(
            MaterialEditorCommand::SetNumber {
                operation: "box".into(),
                parameter: "size1".into(),
                value: 2.4,
            },
            &mut p,
            &mut ctx,
        )
        .unwrap();
    // The normal UI update performs the rebake and publishes its textures.
    publisher.update(&mut ui, &mut ctx, &mut p, &mut server);
    for (i, id) in outputs.iter().enumerate() {
        assert_ne!(p.tiles[id].textures, old[i].textures);
        assert_eq!(p.tiles[id].alias, old[i].alias);
        assert!(p.tiles[id].blocking);
        assert_eq!(
            RUSTERIX.read().unwrap().assets.tiles[id].textures,
            p.tiles[id].textures
        );
    }
    assert_ne!(draw(&p), before);
    UNDOMANAGER
        .write()
        .unwrap()
        .undo(&mut server, &mut p, &mut ui, &mut ctx);
    publisher.update(&mut ui, &mut ctx, &mut p, &mut server);
    assert_eq!(draw(&p), before);
    UNDOMANAGER
        .write()
        .unwrap()
        .redo(&mut server, &mut p, &mut ui, &mut ctx);
    publisher.update(&mut ui, &mut ctx, &mut p, &mut server);
    assert_ne!(draw(&p), before);
}

#[test]
fn picking_isometric_model_tiles_keeps_map_visible_until_explicit_mesh_edit() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    let (recipe, _) = human(&mut p);
    let mut host = GeometryEditor::default();
    for yaw in [0., 90.] {
        let output = Uuid::new_v4();
        p.isometric_bakes.insert(
            output,
            shared::isometric::Bake {
                recipe,
                ground: false,
                yaw,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        let event = TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(output));
        assert!(!host.handle_event(&event, &mut p, &mut ui, &mut ctx));
        // The pixel editor delegates the same pick on a second queued event.
        host.handle_event(
            &TheEvent::Custom(TheId::named("Open Procedural Tile"), TheValue::Id(output)),
            &mut p,
            &mut ui,
            &mut ctx,
        );
        assert!(!host.active);
        assert!(!is_visible(&mut ui));
        assert_eq!(ui.get_stack_layout(STACK).unwrap().index(), 2);
        assert!(ui.get_widget(ASSETS).is_none());
        assert_eq!(host.selected, Some(recipe));
        assert_eq!(host.iso_output, Some(output));
        assert_eq!(host.camera, 4);
        assert!(host.group_path.is_empty());
        assert_eq!(
            ui.get_stack_layout("Sidebar Page Stack").unwrap().index(),
            crate::procedural_material::SIDEBAR_PAGE
        );
        host.handle_event(
            &TheEvent::StateChanged(
                TheId::named("MZ Edit Selected Mesh"),
                TheWidgetState::Clicked,
            ),
            &mut p,
            &mut ui,
            &mut ctx,
        );
        assert!(host.active);
        assert!(is_visible(&mut ui));
        assert_eq!(host.iso_output, Some(output));
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        host.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ Close Geometry"), TheWidgetState::Clicked),
            &mut p,
            &mut ui,
            &mut ctx,
        );
        assert!(!is_visible(&mut ui));
        assert!(receiver.try_iter().any(|event|matches!(event,TheEvent::Custom(id,TheValue::List(values)) if id.name=="Pick Tile Source" && values==vec![TheValue::Text("single".into()),TheValue::Id(output)])));
        for name in ["Tile Picked", "Open Procedural Tile"] {
            host.handle_event(
                &TheEvent::Custom(TheId::named(name), TheValue::Id(output)),
                &mut p,
                &mut ui,
                &mut ctx,
            );
            host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
            assert!(!host.active);
            assert!(!is_visible(&mut ui));
        }
    }
    host.handle_event(
        &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(Uuid::new_v4())),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(!host.active);
    assert!(host.iso_output.is_none());
}

#[test]
fn isometric_map_selects_tile_preview_and_new_models_keep_it() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    p.regions[0].map.camera = rusterix::MapCamera::TwoDIso;
    let mut server = ServerContext::default();
    server.curr_region = p.regions[0].id;
    let mut host = GeometryEditor::default();
    host.update(&p, &mut ui, &mut ctx, &server);
    assert_eq!(host.camera, 4);
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert_eq!(host.camera, 4);
    p.regions[0].map.camera = rusterix::MapCamera::TwoD;
    host.update(&p, &mut ui, &mut ctx, &server);
    assert_eq!(host.camera, 0);
}
#[test]
fn geometry_owns_the_central_viewport_and_restores_map_and_texture_layouts() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let (selected, _) = human(&mut p);
    let mut host = GeometryEditor::default();
    host.selected = Some(selected);
    ui.get_stack_layout("Editor Stack").unwrap().set_index(1); // A dock editor was already open.
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(is_visible(&mut ui));
    assert_eq!(
        ui.get_stack_layout(crate::procedural_material::PREVIEW_STACK)
            .unwrap()
            .index(),
        0
    );
    assert_eq!(
        ui.get_sharedvlayout("Sidebar Map Shared").unwrap().mode(),
        TheSharedVLayoutMode::Top
    );
    ui.canvas.set_dim(TheDim::new(0, 0, 1400, 900), &mut ctx);
    let dim = *ui.get_widget(PREVIEW).unwrap().dim();
    assert!(dim.width >= 1000 && dim.height >= 780, "{dim:?}");
    assert!(ui.get_widget(EDITOR).unwrap().dim().height > 780);
    let point = Vec2::new(dim.x + dim.width / 2, dim.y + dim.height / 2);
    assert_eq!(
        ui.canvas.get_widget_at_coord(point).unwrap().id().name,
        PREVIEW
    );
    assert!(ui.get_widget("MZ New Geometry").unwrap().dim().width > 0);
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ Close Geometry"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(!is_visible(&mut ui));
    assert_eq!(ui.get_stack_layout("Editor Stack").unwrap().index(), 1);
    assert_eq!(
        ui.get_stack_layout(crate::procedural_material::PREVIEW_STACK)
            .unwrap()
            .index(),
        0
    );
    assert_eq!(
        ui.get_sharedvlayout("Sidebar Map Shared").unwrap().mode(),
        TheSharedVLayoutMode::Shared
    );
    assert_eq!(
        ui.get_sharedvlayout("Sidebar Map Shared")
            .unwrap()
            .get_shared_ratio(),
        0.78
    );
}
#[test]
fn navigation_restores_the_map_immediately_and_returns_to_geometry() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let (selected, _) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(selected),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    crate::sidebar::Sidebar::set_navigation_page(0, &mut ui, &mut ctx);
    assert!(!is_visible(&mut ui));
    crate::sidebar::Sidebar::set_navigation_page(
        crate::procedural_material::SIDEBAR_PAGE,
        &mut ui,
        &mut ctx,
    );
    assert!(is_visible(&mut ui));
    host.reset(&mut ui, &mut ctx);
    assert!(!is_visible(&mut ui));
}
#[test]
fn toolbars_show_context_actions_without_mutating_the_model() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let source = p.procedural_appearances[&id].clone();
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    for (tool, context, action) in [
        (1, 1, "MZ Mirror Branch"),
        (3, 2, "MZ Delete Profile"),
        (4, 0, "MZ Motion Phase"),
        (5, 3, ""),
    ] {
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), tool),
            &mut p,
            &mut ui,
            &mut ctx,
        );
        // Layout/interaction regression: the explicit native review tests rendering.
        ui.get_widget(PREVIEW)
            .unwrap()
            .set_dim(TheDim::zero(), &mut ctx);
        host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
        layout::sync_controls(&host, &mut ui, &mut ctx);
        ui.canvas.set_dim(TheDim::new(0, 0, 1100, 800), &mut ctx);
        assert_eq!(
            ui.get_stack_layout("Geometry Context Tools")
                .unwrap()
                .index(),
            context
        );
        if !action.is_empty() {
            let d = *ui.get_widget(action).unwrap().dim();
            assert!(d.width > 0 && d.x + d.width <= 720, "{action}: {d:?}");
        }
    }
    for id in [
        "MZ Geometry Tool",
        "MZ Geometry Camera",
        "MZ Geometry Target",
        "MZ Rig Overlay",
        "MZ Close Geometry",
    ] {
        let d = ui.get_widget(id).unwrap().dim();
        assert!(d.width > 0 && d.x + d.width <= 720, "{id}: {d:?}");
    }
    assert_eq!(p.procedural_appearances[&id], source);
}
#[test]
fn creation_menu_enters_the_large_editor_without_resizing_the_minimap() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let mut host = GeometryEditor::default();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ New Geometry"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(!is_visible(&mut ui));
    assert_eq!(ui.dialog_text, "Mesh recipes");
    assert!(p.procedural_appearances.is_empty());
    ui.clear_dialog();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ New Geometry"), 2),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(is_visible(&mut ui));
    assert_eq!(host.scene.tool, 2);
    assert_eq!(p.procedural_appearances.len(), 1);
    assert_eq!(
        ui.get_sharedvlayout("Sidebar Map Shared")
            .unwrap()
            .get_shared_ratio(),
        0.78
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(
        ui.get_stack_layout("Geometry Context Tools")
            .unwrap()
            .index(),
        1
    );
}
#[test]
#[ignore = "requires a native GPU; exports the full-size rigging workspace"]
fn full_viewport_gpu_review() {
    let (mut ui, mut ctx) = fixture(1500, 1000);
    let mut p = Project::default();
    p.sync_ruleset_items().unwrap();
    let id = Uuid::parse_str(shared::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    ui.canvas.set_dim(TheDim::new(0, 0, 1500, 1000), &mut ctx);
    let dim = *ui.get_widget(PREVIEW).unwrap().dim();
    let e = host.evaluation.as_ref().unwrap();
    let mesh = e.mesh.clone();
    let rig = mesh.rig.as_ref().unwrap();
    let rest = rig.rest();
    let bones = rig
        .joints
        .iter()
        .enumerate()
        .filter_map(|(i, j)| j.parent.map(|p| (rest[p].translation, rest[i].translation)))
        .collect();
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let renderer = preview::Renderer::new(&device);
    let bounds = mesh.bounds();
    host.scene.projection = Some(scene::Projection::new(
        bounds, dim.width, dim.height, -25., 8.,
    ));
    let mut image = renderer
        .render(
            &device,
            &queue,
            &preview::Job {
                surface_seconds: 0.,
                revision: 0,
                mesh,
                palette: p.palette.clone(),
                width: dim.width as u32,
                height: dim.height as u32,
                yaw: -25.,
                pitch: 8.,
                background: *ui.style.theme().color(MaterialCanvas),
                grip: None,
                axes: std::array::from_fn(|i| {
                    ui.style
                        .theme()
                        .palette_color(TheThemePalettes::MaterialCategories, i)
                }),
                framing: Some(bounds),
                bones,
                attachment_frames: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        (image.dim().width, image.dim().height),
        (dim.width, dim.height)
    );
    host.scene.paint(
        &mut image,
        host.document.as_ref().unwrap(),
        e,
        &host.inspector,
        ui.style.theme().as_ref(),
    );
    *ui.get_widget(PREVIEW)
        .unwrap()
        .as_render_view()
        .unwrap()
        .render_buffer_mut() = image;
    let mut pixels = vec![0; 1500 * 1000 * 4];
    ui.draw(&mut pixels, &mut ctx);
    std::fs::write(
        "/tmp/eldiron-full-rigging-workspace.png",
        TheRGBABuffer::from(pixels, 1500, 1000).to_png().unwrap(),
    )
    .unwrap();
    eprintln!("Main rigging viewport: {} x {}", dim.width, dim.height);
}

#[test]
fn rigs_open_visible_and_keep_their_own_realtime_schedule() {
    let (mut ui, mut ctx) = fixture(1500, 1000);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(host.scene.tool, 1);
    assert!(host.wants_realtime());
    assert_eq!(host.group_path, ["body-op-0"]);
    assert_eq!(host.selected_operation.as_deref(), Some("skeleton-op"));
    assert!(!host.playback.playing);
    assert!(ui.get_widget("MZ Edit Texture").is_none());
    crate::sidebar::Sidebar::set_navigation_page(0, &mut ui, &mut ctx);
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert!(!host.wants_realtime());
}
#[test]
fn newer_camera_requests_do_not_hide_completed_compatible_frames() {
    let host = GeometryEditor {
        preview_epoch: 4,
        revision: 20,
        displayed_revision: 5,
        ..Default::default()
    };
    assert!(host.accepts_frame(4, 6));
    assert!(host.accepts_frame(4, 19));
    assert!(!host.accepts_frame(3, 19));
    assert!(!host.accepts_frame(4, 5));
}
#[test]
#[ignore = "requires a native GPU; measures continuous orbit through the actual UI/worker path"]
fn orbit_preview_benchmark() {
    use std::time::{Duration, Instant};
    let (mut ui, mut ctx) = fixture(1500, 1000);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    ui.canvas.set_dim(TheDim::new(0, 0, 1500, 1000), &mut ctx);
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    host.worker = Some(preview::Worker::new(device, queue));
    let walk = host
        .playback
        .clips
        .iter()
        .position(|(id, _)| id == "walk")
        .unwrap()
        + 1;
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Motion"), walk),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    let viewport = *ui.get_widget(PREVIEW).unwrap().dim();
    let warm = Instant::now();
    while host.displayed_revision == 0 && warm.elapsed() < Duration::from_secs(10) {
        host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(host.displayed_revision > 0);
    let mut pixels = vec![0; 1500 * 1000 * 4];
    let mut presented = 0;
    let mut cost = vec![];
    let start = Instant::now();
    for _ in 0..90 {
        let tick = Instant::now();
        let previous = host.displayed_revision;
        host.yaw += 0.75;
        host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
        ui.draw(&mut pixels, &mut ctx);
        if host.displayed_revision > previous {
            presented += 1;
        }
        cost.push(tick.elapsed().as_secs_f64() * 1000.);
        if let Some(left) = Duration::from_secs_f64(1. / 60.).checked_sub(tick.elapsed()) {
            std::thread::sleep(left);
        }
    }
    cost.sort_by(f64::total_cmp);
    let fps = presented as f64 / start.elapsed().as_secs_f64();
    let report = format!(
        "Rig playback + orbit at {}x{}: {presented}/90 updates presented, {fps:.1} presented FPS; UI median {:.2}ms, p95 {:.2}ms",
        viewport.width, viewport.height, cost[45], cost[85]
    );
    eprintln!("{report}");
    std::fs::write("/tmp/eldiron-rig-orbit-performance.txt", report).unwrap();
    assert!(
        presented > 10,
        "Continuous input must still display completed frames"
    );
    std::fs::write(
        "/tmp/eldiron-rig-orbit-workspace.png",
        TheRGBABuffer::from(pixels, 1500, 1000).to_png().unwrap(),
    )
    .unwrap();
}

#[test]
fn rig_mode_exposes_parent_clips_and_can_play_pause_and_scrub_without_navigation() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let source = p.procedural_appearances[&id].clone();
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(host.group_path, ["body-op-0"]);
    assert_eq!(host.scene.tool, 1);
    assert_eq!(
        host.playback
            .clips
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["idle", "walk", "attack"]
    );
    ui.canvas.set_dim(TheDim::new(0, 0, 1400, 900), &mut ctx);
    for id in [
        "MZ Motion",
        "MZ Motion Play",
        "MZ Motion Phase",
        "MZ Mirror Branch",
    ] {
        let d = *ui.get_widget(id).unwrap().dim();
        assert!(d.width > 0 && d.height > 0, "{id}: {d:?}");
        assert_eq!(
            ui.canvas
                .get_widget_at_coord(Vec2::new(d.x + d.width / 2, d.y + d.height / 2))
                .unwrap()
                .id()
                .name,
            id
        );
    }
    // Keep unit tests GPU-free; the native benchmark exercises the full worker path.
    ui.get_widget(PREVIEW)
        .unwrap()
        .set_dim(TheDim::zero(), &mut ctx);
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Motion"), 2),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert!(host.playback.playing);
    assert_eq!(host.scene.tool, 1);
    assert_eq!(host.group_path, ["body-op-0"]);
    host.handle_event(
        &TheEvent::ValueChanged(TheId::named("MZ Motion Phase"), TheValue::Float(0.25)),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(!host.playback.playing);
    assert_eq!(host.playback.phase, 0.25);
    let e = host.evaluation.as_ref().unwrap();
    let motion = host.playback.motion(e).unwrap();
    let posed = e.mesh.posed(Some(motion), motion.duration * 0.25).unwrap();
    assert_ne!(posed.mesh.parts, e.mesh.parts);
    host.scene.posed = posed.joints.clone();
    let index = e.mesh.rig.as_ref().unwrap().index("left-elbow").unwrap();
    let point = host
        .scene
        .handles(host.document.as_ref().unwrap(), e, &host.inspector)
        .into_iter()
        .find(|(h, _, _)| matches!(h,scene::Handle::Joint(id) if id=="left-elbow"))
        .unwrap()
        .1;
    assert_eq!(point, posed.joints[index].translation);
    let rest = e.mesh.rig.as_ref().unwrap().rest();
    assert!(
        (0..3)
            .map(|k| (point[k] - rest[index].translation[k]).abs())
            .sum::<f32>()
            > 0.001
    );
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Motion Play"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(host.playback.playing);
    for tool in [0, 1, 4, 1] {
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), tool),
            &mut p,
            &mut ui,
            &mut ctx,
        );
        host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
        assert_eq!(host.playback.clips[host.playback.selected - 1].0, "walk");
        assert!(host.playback.playing);
        assert_eq!(host.playback.phase, 0.25);
    }
    let e = host.evaluation.as_ref().unwrap();
    let m = host.playback.motion(e).unwrap();
    host.scene.posed = e
        .mesh
        .posed(Some(m), m.duration * host.playback.phase)
        .unwrap()
        .joints;
    let frame = scene::Projection::new(([-1., 0., -1.], [1., 2., 1.]), 400, 400, 0., 0.);
    let point = host
        .scene
        .handles(host.document.as_ref().unwrap(), e, &host.inspector)
        .into_iter()
        .find(|(h, _, _)| matches!(h,scene::Handle::Joint(id) if id=="left-elbow"))
        .unwrap()
        .1;
    let q = frame.project(point);
    host.scene.projection = Some(frame);
    host.handle_event(
        &TheEvent::RenderViewClicked(
            TheId::named(PREVIEW),
            Vec2::new(q[0].round() as i32, q[1].round() as i32),
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert_eq!(host.playback.selected, 0);
    assert!(!host.playback.playing);
    assert!(host.scene.drag.is_none());
    assert_eq!(p.procedural_appearances[&id], source);
}

/// Exercise native dropdown gestures across frame updates, rather than only
/// sending IndexChanged (which bypasses the menu's uncommitted hover state).
#[test]
fn open_dropdowns_keep_hovered_choices_until_mouse_up() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    ui.init(&mut ctx);
    let rx = ui.add_state_listener("rig audit".into());
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    ui.canvas.set_dim(TheDim::new(0, 0, 1400, 900), &mut ctx);
    ui.get_widget(PREVIEW)
        .unwrap()
        .set_dim(TheDim::zero(), &mut ctx);
    for (name, choice) in [
        ("MZ Geometry Camera", 2),
        ("MZ Geometry Tool", 3),
        ("MZ Motion", 2),
    ] {
        ui.get_widget(name)
            .unwrap()
            .on_event(&TheEvent::MouseDown(Vec2::new(5, 5)), &mut ctx);
        ui.get_widget(name).unwrap().on_event(
            &TheEvent::MouseDragged(Vec2::new(5, 20 + choice * 21 + 5)),
            &mut ctx,
        );
        for _ in 0..3 {
            host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
            host.playback
                .tick(host.evaluation.as_ref().unwrap(), &mut ui, &mut ctx);
            assert_eq!(
                ui.get_widget(name)
                    .unwrap()
                    .as_drop_down_menu()
                    .unwrap()
                    .selected_index(),
                choice as usize,
                "{name}"
            );
        }
        ui.get_widget(name)
            .unwrap()
            .on_event(&TheEvent::MouseUp(Vec2::new(5, 5)), &mut ctx);
        ui.process_events(&mut ctx);
        while let Ok(event) = rx.try_recv() {
            host.handle_event(&event, &mut p, &mut ui, &mut ctx);
        }
        host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
        assert_eq!(
            ui.get_widget(name)
                .unwrap()
                .as_drop_down_menu()
                .unwrap()
                .selected_index(),
            choice as usize,
            "{name}"
        );
    }
    assert_eq!(host.camera, 2, "changing tools must retain Side view");
    assert_eq!(host.scene.tool, 3);
    assert_eq!(host.playback.selected, 2);
    assert!(host.playback.playing);
}

#[test]
fn rig_overlay_and_rest_transport_have_stable_actionable_states() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let source = p.procedural_appearances[&id].clone();
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert!(host.playback.show_rig);
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Rig Overlay"), 0),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert!(!host.playback.show_rig);
    assert_eq!(
        ui.get_widget("MZ Rig Overlay")
            .unwrap()
            .as_group_button()
            .unwrap()
            .index(),
        0
    );
    assert!(!ui.get_widget("MZ Rig Overlay").unwrap().disabled());
    host.playback.rest();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Motion Play"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.playback
        .tick(host.evaluation.as_ref().unwrap(), &mut ui, &mut ctx);
    assert!(host.playback.playing);
    assert_eq!(host.playback.selected, 1);
    assert_eq!(
        ui.get_widget("MZ Motion Play")
            .unwrap()
            .as_group_button()
            .unwrap()
            .index(),
        1
    );
    assert_eq!(p.procedural_appearances[&id], source);
    host.inspector.joint = 4;
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Camera"), 2),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 4),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    host.playback.phase = 0.37;
    host.playback.playing = false;
    host.inspector_dirty = true;
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(
        host.playback.phase, 0.37,
        "inspector refresh must not seek a paused clip"
    );
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(
        host.inspector.joint, 4,
        "Rig–Pose–Rig retains the selected joint"
    );
    assert_eq!(host.camera, 2);
    // An empty skeleton has no clip: entering Pose must never author one implicitly.
    let empty = scene::new_document();
    p.procedural_appearances.insert(id, empty.save().unwrap());
    host.group_path.clear();
    host.key = None;
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    let before = p.procedural_appearances[&id].clone();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 4),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    host.playback
        .tick(host.evaluation.as_ref().unwrap(), &mut ui, &mut ctx);
    assert_eq!(p.procedural_appearances[&id], before);
    assert!(ui.get_widget("MZ Motion Play").unwrap().disabled());
    ui.get_widget("MZ Motion Play")
        .unwrap()
        .on_event(&TheEvent::MouseDown(Vec2::new(35, 5)), &mut ctx);
    assert_eq!(
        ui.get_widget("MZ Motion Play")
            .unwrap()
            .as_group_button()
            .unwrap()
            .index(),
        0
    );
}

#[test]
fn use_in_game_copies_the_whole_root_and_keeps_the_editing_scope() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let (id, original) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    assert_eq!(host.group_path, ["body-op-0"]);
    let selected = host.selected_operation.clone();
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Use Ruleset Humanoid"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx, &ServerContext::default());
    let target = Uuid::parse_str(shared::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
    assert_eq!(host.selected, Some(target));
    assert_eq!(host.group_path, ["body-op-0"]);
    assert_eq!(host.selected_operation, selected);
    let copy = Document::load(&p.procedural_appearances[&target]).unwrap();
    assert_eq!(
        copy.rows, original.rows,
        "use in game must copy the root, including gear and clips"
    );
    let saved = shared::project_io::encode_project(&p).unwrap();
    let reopened = shared::project_io::decode_project(&saved).unwrap();
    assert_eq!(
        reopened.procedural_appearances[&target],
        p.procedural_appearances[&target]
    );
    let atom = ProjectUndoAtom::ProceduralAppearanceEdit(
        target,
        None,
        Some(p.procedural_appearances[&target].clone()),
    );
    let mut server = ServerContext::default();
    atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
    assert!(!p.procedural_appearances.contains_key(&target));
    atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
    assert!(p.procedural_appearances.contains_key(&target));
}

#[test]
fn mesh_selection_panel_keeps_map_and_exposes_clickable_edit_and_height_controls() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = std::env::var("ELDIRON_ISO_REVIEW_PROJECT")
        .ok()
        .map(|path| shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap())
        .unwrap_or_default();
    let (recipe, output) = if let Some((output, bake)) = p
        .isometric_bakes
        .iter()
        .find(|(_, b)| !b.ground && b.motion.is_none())
    {
        (bake.recipe, *output)
    } else {
        let (id, _) = human(&mut p);
        let output = Uuid::new_v4();
        p.isometric_bakes.insert(
            output,
            shared::isometric::Bake {
                recipe: id,
                ground: false,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        (id, output)
    };
    let mut host = GeometryEditor::default();
    let offset = p.regions[0].map.offset;
    let zoom = p.regions[0].map.grid_size;
    host.handle_event(
        &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(output)),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    if p.regions[0].map.camera == rusterix::MapCamera::TwoDIso {
        let mut server=ServerContext::default();
        server.curr_region=p.regions[0].id;
        crate::iso_editor::sync(&mut ui,&p,&server,&mut ctx);
    }
    ui.canvas.set_dim(TheDim::new(0, 0, 1400, 900), &mut ctx);
    assert!(!is_visible(&mut ui));
    assert_eq!(host.selected, Some(recipe));
    assert_eq!(p.regions[0].map.offset, offset);
    assert_eq!(p.regions[0].map.grid_size, zoom);
    for name in [
        "MZ Edit Selected Mesh",
        "MZ New Box",
        "Iso Placement Height",
        "Iso Placement Step",
        "Iso Placement Walkable",
        "Iso Placement Stack",
        "Iso Placement Bounds",
        "Iso Placement Snap",
        "Iso Detach Connection",
        "Iso Pick Surface",
    ] {
        let d = *ui.get_widget(name).unwrap().dim();
        assert!(d.width > 0 && d.height > 0, "{name}: {d:?}");
        assert_eq!(
            ui.canvas
                .get_widget_at_coord(Vec2::new(d.x + d.width / 2, d.y + d.height / 2))
                .unwrap()
                .id()
                .name,
            name,
            "{name} is covered"
        );
    }
    if let Ok(dir) = std::env::var("ELDIRON_UI_REVIEW") {
        let view = ui.get_render_view("PolyView").unwrap();
        let (w, h) = (view.dim().width as usize, view.dim().height as usize);
        let mut image = TheRGBABuffer::new(TheDim::sized(w as i32, h as i32));
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(p.tiles.clone());
        let map = &p.regions[0].map;
        rusterix::isometric::draw(
            map,
            &assets,
            &mut image,
            rusterix::isometric::Projection::editor(map, Vec2::new(w as f32, h as f32)),
            0.,
        );
        *ui.get_render_view("PolyView").unwrap().render_buffer_mut() = image;
        let mut pixels = vec![0; 1400 * 900 * 4];
        ui.draw(&mut pixels, &mut ctx);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join("iso-placement-sidebar.png"),
            TheRGBABuffer::from(pixels, 1400, 900).to_png().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn mesh_texture_opens_shared_material_and_returns_to_same_recipe_operation() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    let material = Uuid::new_v4();
    let recipe = Uuid::new_v4();
    let asset = denrim_materials::integration::EditableMaterialAsset::parse(
        &material.to_string(),
        include_str!("../../embedded/materials/05-walls.material"),
        "surface",
    )
    .unwrap();
    p.procedural_materials
        .insert(material, asset.save().unwrap());
    let source = format!(
        r#"{{"version":1,"id":"wall","label":"Wall","rows":[{{"id":"wall","label":"Wall","tiles":[{{"id":"box","op":{{"type":"box","size":[1,1,1]}}}},{{"id":"bricks","op":{{"type":"texture","source":"{material}","size":64,"axes":[0,1],"origin":[0,0],"span":[1,1]}}}}]}}],"output":"wall"}}"#
    );
    let doc = Document::load(&source).unwrap();
    p.procedural_appearances.insert(recipe, source);
    let mut host = GeometryEditor {
        active: true,
        selected: Some(recipe),
        document: Some(doc),
        selected_operation: Some("bricks".into()),
        ..Default::default()
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(sender);
    assert!(host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Texture Material"),
            TheWidgetState::Clicked
        ),
        &mut p,
        &mut ui,
        &mut ctx
    ));
    assert!(!host.active);
    assert!(!is_visible(&mut ui));
    let event=receiver.try_iter().find(|e|matches!(e,TheEvent::Custom(id,TheValue::Id(id2)) if id.name=="Open Mesh Material" && *id2==material)).unwrap();
    let mut materials = crate::procedural_material::ProceduralMaterialEditor::default();
    let mut server = ServerContext::default();
    assert!(materials.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server));
    assert!(p.tiles.contains_key(&material));
    let back = TheEvent::Custom(TheId::named("Return to Mesh Recipe"), TheValue::Empty);
    host.handle_event(&back, &mut p, &mut ui, &mut ctx);
    materials.handle_event(&back, &mut ui, &mut ctx, &mut p, &mut server);
    assert!(is_visible(&mut ui));
    assert_eq!(host.selected, Some(recipe));
    assert_eq!(host.selected_operation.as_deref(), Some("bricks"));
}

#[test]
fn embedded_mesh_material_becomes_a_named_picker_asset_and_relinks_the_recipe() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    let mut p = Project::default();
    let recipe = Uuid::new_v4();
    let a = denrim_materials::integration::EditableMaterialAsset::parse(
        "portable-wall",
        include_str!("../../embedded/materials/05-walls.material"),
        "surface",
    )
    .unwrap();
    let mut doc = Document::load(include_str!(
        "../../../crates/shared/examples/iso_2d/wall.mzgeometry.json"
    ))
    .unwrap();
    for op in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
        if let Operation::Texture { bundle, .. } = &mut op.op {
            *bundle = Some(a.save().unwrap());
        }
    }
    p.procedural_appearances.insert(recipe, doc.save().unwrap());
    let before = p.clone();
    let mut host = GeometryEditor {
        selected: Some(recipe),
        document: Some(doc),
        selected_operation: Some("masonry-2".into()),
        ..Default::default()
    };
    let material = host.texture_material_for_edit(&mut p, &mut ctx).unwrap();
    assert_eq!(p.tiles[&material].alias, "Walls");
    assert_eq!(
        denrim_materials::integration::EditableMaterialAsset::reopen(
            &p.procedural_materials[&material]
        )
        .unwrap()
        .root
        .id,
        material.to_string()
    );
    let d = Document::load(&p.procedural_appearances[&recipe]).unwrap();
    let op = &d
        .rows
        .iter()
        .flat_map(|r| &r.tiles)
        .find(|t| t.id == "masonry-2")
        .unwrap()
        .op;
    assert!(
        matches!(op,Operation::Texture{source,bundle:None,..} if source==&material.to_string())
    );
    let after = p.clone();
    let atom = ProjectUndoAtom::TilePickerEdit(Box::new(before.clone()), Box::new(after));
    let mut server = ServerContext::default();
    atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
    assert_eq!(p.procedural_materials, before.procedural_materials);
    assert_eq!(p.procedural_appearances, before.procedural_appearances);
    atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
    assert!(p.procedural_materials.contains_key(&material));
    assert!(p.procedural_appearances[&recipe].contains(&material.to_string()));
}

#[test]
fn modified_shortcuts_pass_through_mesh_editor_and_hidden_rows_lose_focus() {
    let (mut ui, mut ctx) = fixture(1100, 800);
    ui.init(&mut ctx);
    let mut p = Project::default();
    let (id, _) = human(&mut p);
    let mut host = GeometryEditor {
        selected: Some(id),
        ..Default::default()
    };
    host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("MZ Edit Selected Mesh"),
            TheWidgetState::Clicked,
        ),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    ctx.ui.set_focus(&TheId::named(PREVIEW));
    ui.logo = true;
    assert!(!host.handle_event(
        &TheEvent::KeyDown(TheValue::Char('t')),
        &mut p,
        &mut ui,
        &mut ctx
    ));
    ui.logo = false;
    ui.ctrl = true;
    assert!(!host.handle_event(
        &TheEvent::KeyDown(TheValue::Char('t')),
        &mut p,
        &mut ui,
        &mut ctx
    ));
    ui.ctrl = false;
    assert!(host.handle_event(
        &TheEvent::KeyDown(TheValue::Char('t')),
        &mut p,
        &mut ui,
        &mut ctx
    ));
    host.handle_event(
        &TheEvent::StateChanged(TheId::named("MZ Close Geometry"), TheWidgetState::Clicked),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    assert!(ctx.ui.focus.is_none());
    assert!(ctx.ui.keyboard_focus.is_none());
    assert!(!host.handle_event(
        &TheEvent::KeyDown(TheValue::Char('t')),
        &mut p,
        &mut ui,
        &mut ctx
    ));
}
