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
fn geometry_owns_the_central_viewport_and_restores_map_and_texture_layouts() {
    let (mut ui, mut ctx) = fixture(1400, 900);
    let mut p = Project::default();
    let mut host = GeometryEditor::default();
    ui.get_stack_layout("Editor Stack").unwrap().set_index(1); // A dock editor was already open.
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
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
        1
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
    let mut host = GeometryEditor::default();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
        host.update(&p, &mut ui, &mut ctx);
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
    assert!(is_visible(&mut ui));
    assert_eq!(host.scene.tool, 2);
    assert_eq!(p.procedural_appearances.len(), 1);
    assert_eq!(
        ui.get_sharedvlayout("Sidebar Map Shared")
            .unwrap()
            .get_shared_ratio(),
        0.78
    );
    host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert_eq!(host.scene.tool, 1);
    assert!(host.wants_realtime());
    assert_eq!(host.group_path, ["body-op-0"]);
    assert_eq!(host.selected_operation.as_deref(), Some("skeleton-op"));
    assert!(!host.playback.playing);
    assert!(ui.get_widget("MZ Edit Texture").is_none());
    crate::sidebar::Sidebar::set_navigation_page(0, &mut ui, &mut ctx);
    host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
        host.update(&p, &mut ui, &mut ctx);
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
        host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
    host.update(&p, &mut ui, &mut ctx);
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
        host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
            host.update(&p, &mut ui, &mut ctx);
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
        host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
    assert!(host.playback.show_rig);
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Rig Overlay"), 0),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
    host.update(&p, &mut ui, &mut ctx);
    host.playback.phase = 0.37;
    host.playback.playing = false;
    host.inspector_dirty = true;
    host.update(&p, &mut ui, &mut ctx);
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
    host.update(&p, &mut ui, &mut ctx);
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
    host.update(&p, &mut ui, &mut ctx);
    let before = p.procedural_appearances[&id].clone();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("MZ Geometry Tool"), 4),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
        &TheEvent::IndexChanged(TheId::named(MODE), 1),
        &mut p,
        &mut ui,
        &mut ctx,
    );
    host.update(&p, &mut ui, &mut ctx);
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
    host.update(&p, &mut ui, &mut ctx);
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
