use super::*;

fn add(dock: &mut NodesDock, id: &str) {
    dock.pick(
        GraphPickerItem {
            id: id.into(),
            label: id.into(),
        },
        None,
    );
}

#[test]
fn event_schema_rows_are_stable_and_show_all_offered_fields() {
    let mut dock = NodesDock::new();
    add(&mut dock, "event");
    let event = dock.doc.nodes[0].id;
    let ports = dock.doc.nodes[0].ports.clone();
    assert!(
        dock.doc.nodes[0]
            .rows
            .iter()
            .any(|r| r.label == "event.sector")
    );
    let before = dock.doc.clone();
    catalog::sync_fields(&mut dock.doc, &dock.definitions);
    assert_eq!(before, dock.doc);
    dock.pick(
        GraphPickerItem {
            id: "damaged".into(),
            label: String::new(),
        },
        Some(event),
    );
    assert_eq!(ports, dock.doc.nodes[0].ports);
    let labels: Vec<_> = dock.doc.nodes[0]
        .rows
        .iter()
        .map(|r| r.label.as_str())
        .collect();
    assert!(
        labels.contains(&"event.amount")
            && labels.contains(&"event.attacker")
            && labels.contains(&"event.kind")
    );
    assert!(!labels.contains(&"event.sector"));
    assert_eq!(
        AuthoringContext(&dock.definitions)
            .observe(&dock.doc.nodes[0])
            .execution,
        GraphExecution::Idle
    );
}

#[test]
fn direct_event_to_say_and_filter_branches_need_no_binding_nodes() {
    let mut dock = NodesDock::new();
    add(&mut dock, "event");
    add(&mut dock, "say");
    add(&mut dock, "filter");
    let out = dock.doc.nodes[0].ports[0].id;
    let input = dock.doc.nodes[1].ports[0].id;
    assert!(dock.doc.validate_connection(out, input).is_ok());
    let filter = &dock.doc.nodes[2];
    assert_eq!(
        filter
            .ports
            .iter()
            .filter(|p| p.direction == PortDirection::Output)
            .map(|p| p.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Match", "No Match"]
    );
    assert!(dock.doc.nodes[1].rows.iter().all(|r| r.binding.is_none()));
}

#[test]
fn project_storage_roundtrips_and_old_projects_default_to_empty() {
    let mut project = Project::new();
    let mut dock = NodesDock::new();
    dock.owner = Some("behavior/character/test".into());
    dock.store(&mut project, false);
    assert!(project.node_graphs.is_empty());
    assert!(!dock.dirty);
    add(&mut dock, "event");
    dock.store(&mut project, true);
    let json = serde_json::to_value(&project).unwrap();
    let restored: Project = serde_json::from_value(json.clone()).unwrap();
    let graph: GraphDocument =
        serde_json::from_value(restored.node_graphs["behavior/character/test"].clone()).unwrap();
    assert_eq!(graph, dock.doc);
    let mut old = json;
    old.as_object_mut().unwrap().remove("node_graphs");
    assert!(
        serde_json::from_value::<Project>(old)
            .unwrap()
            .node_graphs
            .is_empty()
    );
}

#[test]
fn creator_dock_edits_undo_and_owner_switching() {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1200, 650, 1.);
    let mut ui = TheUI::new();
    ui.canvas = dock.setup(&mut ctx);
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 1200, 650), &mut ctx);
    let mut server = ServerContext::default();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    server.pc = ProjectContext::CharacterCode(first);
    let mut project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    add(&mut dock, "event");
    add(&mut dock, "filter");
    add(&mut dock, "say");
    for (from_node, from_key, to_node) in [(0, "out", 1), (1, "match", 2)] {
        let from = dock.doc.nodes[from_node]
            .ports
            .iter()
            .find(|p| p.key.as_deref() == Some(from_key))
            .unwrap()
            .id;
        let to = dock.doc.nodes[to_node]
            .ports
            .iter()
            .find(|p| p.direction == PortDirection::Input)
            .unwrap()
            .id;
        dock.doc.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from,
            to,
        });
    }
    dock.store(&mut project, true);
    let first_doc = dock.doc.clone();
    dock.render(&mut ui, &mut ctx);
    if let Ok(path) = std::env::var("ELDIRON_NODES_SNAPSHOT") {
        let png = ui
            .get_render_view(VIEW)
            .unwrap()
            .render_buffer_mut()
            .to_png()
            .unwrap();
        std::fs::write(path, png).unwrap();
    }
    dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
    assert!(dock.doc.nodes.is_empty());
    dock.redo(&mut ui, &mut ctx, &mut project, &mut server);
    assert_eq!(dock.doc, first_doc);
    server.pc = ProjectContext::CharacterCode(second);
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    assert!(dock.doc.nodes.is_empty());
    add(&mut dock, "say");
    dock.store(&mut project, true);
    server.pc = ProjectContext::CharacterCode(first);
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    assert_eq!(dock.doc, first_doc);
    assert_eq!(project.node_graphs.len(), 2);
}

#[test]
fn catalog_drop_uses_viewport_and_global_dock_history() {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1200, 650, 1.);
    let mut ui = TheUI::new();
    ui.canvas = dock.setup(&mut ctx);
    assert!(ui.get_widget("Nodes Undo").is_none());
    assert!(ui.get_widget("Nodes Add").is_none());
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 1200, 650), &mut ctx);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(Uuid::new_v4());
    let mut project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    dock.editor.viewport.pan = [75., 40.];
    dock.editor.viewport.zoom_at([0., 0.], 0.75);
    let point = Vec2::new(300, 200);
    let expected = dock.editor.viewport.to_graph([300., 200.]);
    let mut drop = TheDrop::new(TheId::named("Node Catalog/filter"));
    drop.data = "filter".into();
    assert!(dock.handle_event(
        &TheEvent::RenderViewDrop(TheId::named(VIEW), point, drop),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server
    ));
    assert_eq!(dock.doc.nodes[0].position, expected);
    assert!(dock.supports_undo());
    dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
    assert!(dock.doc.nodes.is_empty());
    dock.redo(&mut ui, &mut ctx, &mut project, &mut server);
    assert_eq!(dock.doc.nodes.len(), 1);
    // Delete is a graph operation when no text control has focus.
    dock.editor.selected = Some(dock.doc.nodes[0].id);
    let id = ui.get_render_view(VIEW).unwrap().id().clone();
    ctx.ui.set_focus(&id);
    dock.handle_event(
        &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(dock.doc.nodes.is_empty());
}

#[test]
fn catalog_filters_by_owner_and_rejects_unrelated_drops() {
    let character = ProjectContext::CharacterCode(Uuid::new_v4());
    assert!(node_available(character, "say"));
    assert!(!node_available(ProjectContext::WorldCode, "say"));
    assert!(node_available(ProjectContext::WorldCode, "event"));
    assert!(!node_available(
        ProjectContext::Screen(Uuid::new_v4()),
        "event"
    ));
    assert!(!node_available(character, "invented"));
    assert!(!list::node_help("filter").is_empty());
}

#[test]
fn node_list_and_dock_layout_snapshot() {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1450, 650, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    ui.canvas = dock.setup(&mut ctx);
    let mut sidebar = node_list_canvas();
    sidebar
        .get_layout(Some(&"Node Catalog List".to_string()), None)
        .unwrap()
        .limiter_mut()
        .set_max_width(300);
    ui.canvas.set_right(sidebar);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(Uuid::new_v4());
    let project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    sync_node_list(&mut ui, &mut ctx, server.pc);
    add(&mut dock, "event");
    add(&mut dock, "filter");
    add(&mut dock, "say");
    let mut pixels = vec![0; 1450 * 650 * 4];
    ui.draw(&mut pixels, &mut ctx);
    dock.render(&mut ui, &mut ctx);
    ui.draw(&mut pixels, &mut ctx);
    let dim = *ui.get_render_view(VIEW).unwrap().dim();
    assert_eq!(dim.y, 0);
    assert!(
        dim.width > 1000 && dim.width < 1450,
        "graph dimensions: {dim:?}"
    );
    if let Ok(path) = std::env::var("ELDIRON_NODE_LIST_SNAPSHOT") {
        std::fs::write(
            path,
            TheRGBABuffer::from(pixels.clone(), 1450, 650)
                .to_png()
                .unwrap(),
        )
        .unwrap();
    }
    if let Ok(path) = std::env::var("ELDIRON_EVENT_PICKER_SNAPSHOT") {
        let row = dock.doc.nodes[0].row_rect(0);
        let anchor = GraphRect {
            origin: dock.editor.viewport.to_screen(row.origin),
            size: row.size,
        };
        let picker = GraphPicker::compact(
            anchor,
            [dim.width as f32, dim.height as f32],
            dock.definitions
                .events()
                .map(|e| GraphPickerItem {
                    id: e.id.clone(),
                    label: e.label.clone(),
                })
                .collect(),
        );
        dock.popup = Some((picker, Some(dock.doc.nodes[0].id)));
        dock.render(&mut ui, &mut ctx);
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            path,
            TheRGBABuffer::from(pixels, 1450, 650).to_png().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn compact_event_picker_fits_a_small_dock() {
    let defs = catalog::definitions();
    let items = defs
        .events()
        .map(|e| GraphPickerItem {
            id: e.id.clone(),
            label: e.label.clone(),
        })
        .collect();
    let picker = GraphPicker::compact(
        GraphRect {
            origin: [500., 120.],
            size: [204., 28.],
        },
        [720., 180.],
        items,
    );
    let size = picker.size();
    assert!(size[0] <= 260. && size[1] <= 180.);
    assert!(picker.origin[0] + size[0] <= 720. && picker.origin[1] + size[1] <= 180.);
    assert!(
        picker
            .pick([picker.origin[0] + 12., picker.origin[1] + 12.])
            .is_some()
    );
}

#[test]
fn touchpad_pan_corrects_both_widget_inverted_axes() {
    let mut dock = NodesDock::new();
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(700, 200, 1.);
    ui.canvas = dock.setup(&mut ctx);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(Uuid::new_v4());
    let mut project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    dock.handle_event(
        &TheEvent::RenderViewPreciseScrollBy(TheId::named(VIEW), Vec2::new(12, -8)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert_eq!(dock.editor.viewport.pan, [28., 48.]);
    assert!(project.node_graphs.is_empty());
}

#[test]
fn dispatched_events_reach_selected_owner_without_source_debugging() {
    use rusterix::server::{
        event_observation::EventOwner, region_host::run_server_named_fn, regionctx::RegionCtx,
    };
    use rusterix::vm::{Execution, Program, VMValue};
    let mut project = Project::new();
    let region = shared::region::Region::new();
    let region_id = region.id;
    let map_id = region.map.id;
    project.regions.push(region);
    let mut entity = rusterix::Entity::new();
    entity.id = 17;
    let instance = entity.creator_id;
    let mut runtime_ctx = RegionCtx::default();
    runtime_ctx.map.id = map_id;
    runtime_ctx.map.entities.push(entity);
    runtime_ctx.curr_entity_id = 17;
    assert!(!runtime_ctx.debug_mode);
    // Delivery is observable even when no legacy handler is defined.
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &[
            VMValue::from_string("entered"),
            VMValue::from_string("garden"),
        ],
        &Program::new(),
        &mut runtime_ctx,
    );
    let event = runtime_ctx.event_observations.pop_front().unwrap();
    assert_eq!(event.owner, EventOwner::Entity(instance));
    let mut server = rusterix::server::Server::new();
    server.state = rusterix::ServerState::Running;
    server.event_observations.push_back(event);
    let mut context = ServerContext::default();
    context.pc = ProjectContext::RegionCharacterInstance(region_id, instance);
    let mut dock = NodesDock::new();
    add(&mut dock, "event");
    let authored = dock.doc.clone();
    assert!(dock.update_live(&server, &project, &context));
    let row = dock.doc.nodes[0]
        .rows
        .iter()
        .find(|r| r.key.as_deref() == Some("offers:sector"))
        .unwrap();
    assert_eq!(
        live::LiveContext {
            definitions: &dock.definitions,
            live: &dock.live
        }
        .row_label(&dock.doc.nodes[0], row),
        Some("garden".into())
    );
    assert_eq!(dock.doc, authored); // Runtime values are not saved into graph parameters.
    if let Ok(path) = std::env::var("ELDIRON_LIVE_NODE_SNAPSHOT") {
        let mut ctx = TheContext::new(620, 320, 1.);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        ui.get_render_view(VIEW)
            .unwrap()
            .set_dim(TheDim::new(0, 0, 620, 320), &mut ctx);
        dock.render(&mut ui, &mut ctx);
        std::fs::write(
            path,
            ui.get_render_view(VIEW)
                .unwrap()
                .render_buffer_mut()
                .to_png()
                .unwrap(),
        )
        .unwrap();
    }

    context.pc = ProjectContext::RegionCharacterInstance(region_id, Uuid::new_v4());
    dock.update_live(&server, &project, &context);
    assert!(dock.live.latest.is_empty());
    assert!(!live::matches_owner(
        server.event_observations.front().unwrap(),
        context.pc,
        &project,
        region_id
    ));
    server.event_session = Uuid::new_v4();
    server.event_observations.clear();
    dock.update_live(&server, &project, &context);
    assert!(dock.live.latest.is_empty());
    server.state = rusterix::ServerState::Off;
    dock.update_live(&server, &project, &context);
    assert!(!dock.live.active);
}

#[test]
fn say_defaults_are_context_neutral_and_character_behavior_nodes_are_available() {
    let mut dock = NodesDock::new();
    add(&mut dock, "say");
    assert_eq!(
        dock.doc.nodes[0].rows[0].value,
        GraphControlValue::Text(String::new())
    );
    let character = ProjectContext::CharacterCode(Uuid::new_v4());
    for key in ["routine", "random_walk", "resume_routine"] {
        assert!(list::node_available(character, key));
        assert!(!list::node_available(ProjectContext::WorldCode, key));
        assert!(dock.definitions.node(key).is_some());
    }
}
