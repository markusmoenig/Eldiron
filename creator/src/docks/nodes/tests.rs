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

/// Showing a branch lays it out, so graphs are compared without positions.
fn without_positions(mut doc: GraphDocument) -> GraphDocument {
    for node in doc.nodes.iter_mut() {
        node.position = [0., 0.];
    }
    doc
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
            .any(|r| r.label == "event.area")
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
    assert!(!labels.contains(&"event.area"));
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
    // Activating lays the branch out, so only the graph itself is compared.
    assert_eq!(
        without_positions(dock.doc.clone()),
        without_positions(first_doc.clone())
    );
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
    assert!(
        ui.canvas
            .get_layout(Some(&"Node Graph Actions".to_string()), None)
            .is_some(),
        "the graph toolbar belongs to the dock"
    );
    assert!(
        ui.canvas
            .get_layout(Some(&"Node Branches List".to_string()), None)
            .is_some(),
        "the branch list belongs to the dock"
    );
    let dim = *ui.get_render_view(VIEW).unwrap().dim();
    assert!(dim.y > 0, "the graph view starts below the toolbar");
    assert!(dim.y < 60, "graph dimensions: {dim:?}");
    // The branch list takes a fixed left column, so the canvas is narrower.
    assert!(
        dim.width > 800 && dim.width < 1450,
        "graph dimensions: {dim:?}"
    );
    assert!(dim.x >= 180, "the branch list sits to the left: {dim:?}");
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
        let row = dock.doc.nodes[0].row_rect(0, &dock.doc.metrics());
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
        .find(|r| r.key.as_deref() == Some("offers:area"))
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

#[test]
fn a_named_area_owns_its_graph_and_offers_on_area() {
    let region = Uuid::new_v4();
    let area = Uuid::new_v4();
    let pc = ProjectContext::RegionArea(region, area);
    assert_eq!(
        NodesDock::owner(pc),
        Some(format!("behavior/region/{region}/area/{area}"))
    );
    assert!(list::node_available(pc, "on_area"));
    assert!(list::node_available(pc, "teleport"));
    assert!(list::node_available(pc, "set_attribute"));
    assert!(!list::node_available(pc, "on_enter_area"));
    assert!(!list::node_available(pc, "player_camera"));
}

#[test]
fn inventory_nodes_are_offered_to_characters_only() {
    let character = ProjectContext::CharacterCode(Uuid::new_v4());
    assert!(list::node_available(character, "add_item"));
    assert!(list::node_available(character, "drop_items"));
    assert!(!list::node_available(ProjectContext::WorldCode, "add_item"));
    assert!(!list::node_available(
        ProjectContext::RegionCode(Uuid::new_v4()),
        "drop_items"
    ));
}

#[test]
fn pasting_a_graph_gives_fresh_ids_and_follows_its_connections() {
    let mut source = GraphDocument::default();
    let mut node = GraphNode::new("On Startup", [10., 20.], [16, 112, 98, 255]);
    node.id = Uuid::new_v4();
    let source_node = node.id;
    let mut port = GraphPort::new("out", PortDirection::Output, PortSide::Right, 0.5);
    port.id = Uuid::new_v4();
    let source_port = port.id;
    node.ports.push(port);
    source.nodes.push(node);
    source.connections.push(GraphConnection {
        id: Uuid::new_v4(),
        from: source_port,
        to: source_port,
    });

    let mut document = GraphDocument::default();
    let moved = paste_graph(&mut document, &source);
    assert_eq!(document.nodes.len(), 1);
    assert_eq!(
        moved.get(&source_node).copied(),
        Some(document.nodes[0].id),
        "the caller can find where a source node landed"
    );
    let pasted = &document.nodes[0];
    assert_ne!(pasted.id, source_node, "the pasted node needs its own id");
    assert_eq!(
        pasted.position,
        [10. + PASTE_OFFSET, 20. + PASTE_OFFSET],
        "the copy is nudged off its original"
    );
    assert_ne!(pasted.ports[0].id, source_port);
    assert_eq!(
        document.connections[0].from, pasted.ports[0].id,
        "the connection follows the pasted port"
    );

    // Pasting twice keeps every id unique within the document.
    paste_graph(&mut document, &source);
    assert_ne!(document.nodes[0].id, document.nodes[1].id);
    assert_ne!(document.connections[0].id, document.connections[1].id);
    assert_eq!(document.connections[1].from, document.nodes[1].ports[0].id);
}

#[test]
fn clearing_the_graph_records_an_undo_step() {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1450, 650, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    ui.canvas = dock.setup(&mut ctx);
    let owner = Uuid::new_v4();
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(owner);
    let mut project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    add(&mut dock, "event");
    add(&mut dock, "say");
    dock.finish(&mut project);

    let key = format!("behavior/character/{owner}");
    assert!(project.node_graphs.contains_key(&key));

    dock.clear_nodes(&mut ui, &mut ctx, &mut project);

    // Clearing has to be undoable, which is what enables the Undo button.
    let history = dock.histories.get(&key).expect("undo history");
    assert!(!history.undo.is_empty(), "clearing is an undoable step");
    assert!(
        project.node_graphs[&key]["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.is_empty()),
        "the stored graph is empty"
    );
}

#[test]
fn every_registered_node_is_offered_somewhere_and_carries_help() {
    let contexts = [
        ProjectContext::WorldCode,
        ProjectContext::RegionCode(Uuid::new_v4()),
        ProjectContext::CharacterCode(Uuid::new_v4()),
        ProjectContext::RegionCharacterInstance(Uuid::new_v4(), Uuid::new_v4()),
        ProjectContext::ItemCode(Uuid::new_v4()),
        ProjectContext::RegionArea(Uuid::new_v4(), Uuid::new_v4()),
    ];
    let ids = rusterix::server::nodes::Registry::shared().ids();
    assert!(
        ids.len() > 20,
        "expected the builtin registry to be populated"
    );
    for id in ids {
        assert!(
            contexts.iter().any(|pc| list::node_available(*pc, id)),
            "node '{id}' cannot be placed in any behavior graph"
        );
        assert!(
            !list::node_help(id).is_empty(),
            "node '{id}' has no help text"
        );
    }
}

/// Fits roughly five characters per line, enough to force wrapping in tests.
struct Measure;
impl GraphPainter for Measure {
    fn round_rect(&mut self, _: GraphRect, _: f32, _: GraphColor) {}
    fn text_width(&mut self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size
    }
    fn text(&mut self, _: GraphRect, _: &str, _: f32, _: GraphColor) {}
    fn curve(&mut self, _: [Point; 4], _: f32, _: GraphColor) {}
    fn preview(&mut self, _: GraphRect, _: &str) {}
}

fn overlay_target() -> TextTarget {
    TextTarget {
        node: Uuid::new_v4(),
        row: Uuid::new_v4(),
        cell: None,
    }
}

#[test]
fn text_overlay_edits_multi_line_text() {
    let mut overlay = TextOverlay::new(
        overlay_target(),
        "hello world",
        GraphRect {
            origin: [0., 0.],
            size: [240., 200.],
        },
    );
    assert!(!overlay.changed());
    overlay.input(GraphTextInput::Insert("\nsecond".into()));
    assert_eq!(overlay.text(), "hello world\nsecond");
    assert!(overlay.changed());
    overlay.input(GraphTextInput::Backspace);
    assert_eq!(overlay.text(), "hello world\nsecon");
    // Select all then type replaces the whole draft, not the selected value.
    overlay.input(GraphTextInput::SelectAll);
    overlay.input(GraphTextInput::Insert("x".into()));
    assert_eq!(overlay.text(), "x");
    assert!(overlay.selection().is_empty());
}

#[test]
fn text_overlay_wraps_long_lines_and_scrolls() {
    let mut overlay = TextOverlay::new(
        overlay_target(),
        "one two three four five six seven eight nine",
        GraphRect {
            origin: [0., 0.],
            size: [200., 120.],
        },
    );
    let mut painter = Measure;
    overlay.paint(&mut painter, &GraphTheme::default());
    assert!(
        overlay.line_count() > 1,
        "a long line has to wrap into several rows"
    );
    let before = overlay.line_count();
    overlay.input(GraphTextInput::End { extend: false });
    overlay.paint(&mut painter, &GraphTheme::default());
    assert_eq!(overlay.line_count(), before, "layout is stable across paint");
}

#[test]
fn text_overlay_copies_cuts_and_pastes() {
    let mut overlay = TextOverlay::new(
        overlay_target(),
        "hello",
        GraphRect {
            origin: [0., 0.],
            size: [240., 200.],
        },
    );
    overlay.input(GraphTextInput::SelectAll);
    assert_eq!(overlay.copy(), "hello");
    assert_eq!(overlay.cut(), "hello");
    assert_eq!(overlay.text(), "");
    overlay.paste("world");
    assert_eq!(overlay.text(), "world");
    // Pasting replaces the current selection.
    overlay.input(GraphTextInput::SelectAll);
    overlay.paste("again");
    assert_eq!(overlay.text(), "again");
}

#[test]
fn branches_are_separate_and_focus_one_at_a_time() {
    let mut dock = NodesDock::new();
    // Each trigger is its own branch; adding one focuses it.
    add(&mut dock, "event");
    let first = dock.editor.selected.expect("first trigger");
    assert_eq!(dock.active_branch, Some(first));
    assert!(dock.editor.node_visible(first));

    add(&mut dock, "on_event");
    let second = dock.editor.selected.expect("second trigger");
    assert_eq!(dock.active_branch, Some(second));
    assert!(dock.editor.node_visible(second));
    assert!(
        !dock.editor.node_visible(first),
        "only the picked branch is shown"
    );

    // One row per trigger, and no combined entry.
    let items = dock.branch_items();
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .any(|(id, _, active)| id == &format!("Branch/{second}") && *active),
        "the picked branch is the active row"
    );
    assert!(
        items
            .iter()
            .any(|(id, _, active)| id == &format!("Branch/{first}") && !*active)
    );

    // Removing a branch deletes its node and falls back to the other branch.
    assert!(dock.remove_branch());
    assert!(dock.doc.nodes.iter().all(|node| node.id != second));
    assert!(dock.doc.nodes.iter().any(|node| node.id == first));
    assert_eq!(dock.active_branch, Some(first));
}

#[test]
fn picking_a_branch_centres_it_in_the_view() {
    let mut dock = NodesDock::new();
    add(&mut dock, "event");
    // Move the trigger far away, then select the branch again.
    if let Some(node) = dock.doc.nodes.first_mut() {
        node.position = [4000., 3000.];
    }
    let root = dock.editor.selected.expect("trigger");
    dock.set_branch(Some(root));
    dock.editor.viewport.fit_to_nodes(
        &dock.doc,
        [800., 600.],
        &dock.shown_branch().1.expect("branch nodes"),
    );
    let rect = dock.doc.nodes[0].rect(&dock.doc.metrics());
    let screen = dock.editor.viewport.to_screen([
        rect.origin[0] + rect.size[0] * 0.5,
        rect.origin[1] + rect.size[1] * 0.5,
    ]);
    assert!(
        (screen[0] - 400.).abs() < 40. && (screen[1] - 300.).abs() < 40.,
        "the branch centres in the view, got {screen:?}"
    );
}

#[test]
fn compact_style_and_folding_shrink_the_shown_branch() {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1200, 650, 1.);
    let mut ui = TheUI::new();
    ui.canvas = dock.setup(&mut ctx);
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 1200, 650), &mut ctx);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(Uuid::new_v4());
    let mut project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);

    add(&mut dock, "event");
    add(&mut dock, "dialogue");
    let tallest = |dock: &NodesDock| {
        dock.doc
            .nodes
            .iter()
            .map(|node| node.height(&dock.doc.metrics()))
            .fold(0., f32::max)
    };

    let comfortable = tallest(&dock);
    dock.doc.compact = true;
    let compact = tallest(&dock);
    assert!(
        compact < comfortable,
        "compact {compact} should be shorter than comfortable {comfortable}"
    );

    assert_eq!(
        set_folded(&mut dock, true),
        2,
        "both nodes have rows to fold"
    );
    let folded = tallest(&dock);
    assert!(folded < compact, "folded {folded} vs compact {compact}");

    // Clicking the chevron again opens the branch back up.
    assert_eq!(set_folded(&mut dock, false), 2);
    assert_eq!(tallest(&dock), compact);

    // Both the style and the folds are part of the graph document, so they come
    // back with it when the dock is re-activated.
    set_folded(&mut dock, true);
    let folded: Vec<_> = dock
        .doc
        .nodes
        .iter()
        .filter(|node| node.folded)
        .map(|node| node.id)
        .collect();
    assert!(!folded.is_empty());
    dock.store(&mut project, true);
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    assert!(dock.doc.compact, "the node style travels with the graph");
    assert_eq!(
        dock.doc
            .nodes
            .iter()
            .filter(|node| node.folded)
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        folded,
        "the folds travel with the graph"
    );

    // Optional visual check, matching the other snapshot hooks in this module.
    // The picture shows the folded shape, whatever the state was before.
    if let Ok(path) = std::env::var("ELDIRON_COMPACT_SNAPSHOT") {
        set_folded(&mut dock, true);
        dock.tidy_branch();
        dock.store(&mut project, true);
        let mut pixels = vec![0; 1200 * 650 * 4];
        ui.draw(&mut pixels, &mut ctx);
        dock.render(&mut ui, &mut ctx);
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            path,
            TheRGBABuffer::from(pixels, 1200, 650).to_png().unwrap(),
        )
        .unwrap();
    }
}

/// A dock with a live owner, ready for branch operations.
fn live_dock() -> (NodesDock, TheUI, TheContext, Project, ServerContext) {
    let mut dock = NodesDock::new();
    let mut ctx = TheContext::new(1200, 650, 1.);
    let mut ui = TheUI::new();
    ui.canvas = dock.setup(&mut ctx);
    let mut server = ServerContext::default();
    server.pc = ProjectContext::CharacterCode(Uuid::new_v4());
    let project = Project::new();
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    (dock, ui, ctx, project, server)
}

/// Click a node's title bar chevron, which is the fold affordance.
fn click_fold_handle(dock: &mut NodesDock, id: GraphId) {
    let metrics = dock.doc.metrics();
    let handle = dock
        .doc
        .nodes
        .iter()
        .find(|node| node.id == id)
        .map(|node| node.fold_handle(&metrics))
        .expect("node");
    let point = dock.editor.viewport.to_screen([
        handle.origin[0] + handle.size[0] * 0.5,
        handle.origin[1] + handle.size[1] * 0.5,
    ]);
    dock.editor
        .pointer_down(&mut dock.doc, point, &BasicGraphControls);
}

/// Fold or unfold every node with rows through its chevron.
fn set_folded(dock: &mut NodesDock, folded: bool) -> usize {
    let targets: Vec<GraphId> = dock
        .doc
        .nodes
        .iter()
        .filter(|node| !node.rows.is_empty() && node.folded != folded)
        .map(|node| node.id)
        .collect();
    for id in &targets {
        click_fold_handle(dock, *id);
    }
    targets.len()
}

/// Wire `from`'s `key` output to the first input of `to`.
fn connect(dock: &mut NodesDock, from: GraphId, key: &str, to: GraphId) {
    let output = dock
        .doc
        .nodes
        .iter()
        .find(|node| node.id == from)
        .and_then(|node| {
            node.ports
                .iter()
                .find(|port| port.key.as_deref() == Some(key))
        })
        .expect("output port")
        .id;
    let input = dock
        .doc
        .nodes
        .iter()
        .find(|node| node.id == to)
        .and_then(|node| {
            node.ports
                .iter()
                .find(|port| port.direction == PortDirection::Input)
        })
        .expect("input port")
        .id;
    dock.doc.connections.push(GraphConnection {
        id: Uuid::new_v4(),
        from: output,
        to: input,
    });
}

#[test]
fn copy_and_paste_work_on_one_branch() {
    let (mut dock, mut ui, mut ctx, mut project, _server) = live_dock();

    // Two branches, each a trigger plus one action.
    add(&mut dock, "event");
    let first = dock.editor.selected.expect("first trigger");
    add(&mut dock, "say");
    let first_say = dock.editor.selected.expect("first action");
    connect(&mut dock, first, "out", first_say);
    add(&mut dock, "on_event");
    let second = dock.editor.selected.expect("second trigger");
    add(&mut dock, "say");
    let second_say = dock.editor.selected.expect("second action");
    connect(&mut dock, second, "out", second_say);

    // Copy only what is shown.
    dock.set_branch(Some(first));
    assert!(dock.copy_branch(&mut project));
    let clipboard = dock.clipboard.clone().expect("clipboard");
    let copied: Vec<_> = clipboard.nodes.iter().map(|node| node.id).collect();
    assert!(copied.contains(&first) && copied.contains(&first_say));
    assert!(
        !copied.contains(&second) && !copied.contains(&second_say),
        "the other branch stays out of the clipboard"
    );
    assert_eq!(
        clipboard.connections.len(),
        1,
        "the wiring inside the branch comes along"
    );

    // Pasting lands it as its own branch, with new ids, and shows it.
    let nodes_before = dock.doc.nodes.len();
    let connections_before = dock.doc.connections.len();
    assert!(dock.paste_branch(&mut ui, &mut ctx, &mut project));
    assert_eq!(dock.doc.nodes.len(), nodes_before + 2);
    assert_eq!(dock.doc.connections.len(), connections_before + 1);
    assert_eq!(dock.branch_items().len(), 3, "a third branch appears");
    let pasted_root = dock.active_branch.expect("the pasted branch is focused");
    assert!(pasted_root != first && pasted_root != second);
    assert!(dock.editor.node_visible(pasted_root));
    assert!(dock.doc.nodes.iter().any(|node| node.id == first));
}

#[test]
fn a_new_trigger_node_creates_and_focuses_its_branch() {
    let (mut dock, _ui, _ctx, _project, _server) = live_dock();

    assert!(dock.branch_items().is_empty());
    // An action on its own is a stray, not a branch.
    add(&mut dock, "say");
    assert!(dock.branch_items().is_empty(), "a stray node is not a branch");
    assert!(dock.editor.node_visible(dock.editor.selected.unwrap()));

    // A left sided node starts a branch, and it is shown straight away.
    add(&mut dock, "on_event");
    let trigger = dock.editor.selected.expect("trigger");
    assert_eq!(dock.active_branch, Some(trigger));
    assert_eq!(dock.branch_items().len(), 1);
    assert!(dock.branch_items()[0].2, "the new branch is the active row");
    assert!(dock.editor.node_visible(trigger));
}

#[test]
fn the_conversation_editor_edits_a_talk_node() {
    let (mut dock, mut ui, mut ctx, mut project, mut server) = live_dock();
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");

    // A dropped Talk node ships with a starter conversation document, so the
    // panel opens on something valid.
    let conversation = NodesDock::conversation_of(
        dock.doc.nodes.iter().find(|node| node.id == talk).unwrap(),
    )
    .expect("a conversation");
    assert_eq!(conversation.steps.len(), 1);
    assert_eq!(conversation.steps[0].choices.len(), 1);

    // Open the panel on the node, then click the line field to edit it.
    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [900., 600.]), "the panel opens");
    let line = ConversationField::StepLine(0);
    let field = dock.conversation.as_ref().unwrap().field_rect(line);
    let click = dock
        .conversation
        .as_mut()
        .unwrap()
        .pointer_down([field.origin[0] + 6., field.origin[1] + 6.]);
    assert_eq!(
        click,
        ConversationClick::Edit(line),
        "clicking a field asks for it to be edited"
    );

    dock.open_conversation_field(line, &mut ui, &mut ctx);
    assert!(dock.text_overlay.is_some(), "the shared overlay edits the field");
    if let Some(overlay) = dock.text_overlay.as_mut() {
        overlay.input(GraphTextInput::SelectAll);
        overlay.paste("Well met, traveller.");
    }
    dock.close_conversation(&mut project, true);

    // The node carries one conversation document holding the edit.
    let node = dock.doc.nodes.iter().find(|node| node.id == talk).unwrap();
    assert_eq!(node.rows.len(), 1, "the document is the node's only row");
    assert_eq!(node.rows[0].key.as_deref(), Some("conversation"));
    let written = NodesDock::conversation_of(node).expect("conversation");
    assert_eq!(written.steps[0].text, "Well met, traveller.");
    assert_eq!(written.steps[0].name, "start");

    // The document survives saving and reopening the dock.
    dock.store(&mut project, true);
    dock.activate(&mut ui, &mut ctx, &project, &mut server);
    let node = dock.doc.nodes.iter().find(|node| node.id == talk).unwrap();
    let reopened = NodesDock::conversation_of(node).expect("conversation");
    assert_eq!(reopened.steps[0].text, "Well met, traveller.");

    // Optional visual check, matching the other snapshot hooks in this module.
    if let Ok(path) = std::env::var("ELDIRON_TALK_SNAPSHOT") {
        let mut pixels = vec![0; 1200 * 650 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let view = *ui.get_render_view(VIEW).unwrap().dim();
        let point = dock.editor.viewport.to_screen([
            dock.doc.nodes[0].position[0] + 20.,
            dock.doc.nodes[0].position[1] + 20.,
        ]);
        dock.open_conversation(point, [view.width as f32, view.height as f32]);
        // Show the output form with a resume step, the richest shape.
        if let Some(editor) = dock.conversation.as_mut() {
            for _ in 0..2 {
                if let Some(rect) = editor.then_rect(0, 0) {
                    editor.pointer_down([rect.origin[0] + 4., rect.origin[1] + 4.]);
                }
            }
            if let Some(rect) = editor.then_resume_rect(0, 0) {
                editor.pointer_down([rect.origin[0] + 4., rect.origin[1] + 4.]);
            }
        }
        let mut pixels = vec![0; 1200 * 650 * 4];
        ui.draw(&mut pixels, &mut ctx);
        dock.render(&mut ui, &mut ctx);
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            path,
            TheRGBABuffer::from(pixels, 1200, 650).to_png().unwrap(),
        )
        .unwrap();
    }
}

/// Put a conversation document on a Talk node: one step per entry, named
/// `stepN`, each with the given number of choices that simply end.
fn set_conversation(dock: &mut NodesDock, talk: GraphId, choices_per_step: &[usize]) {
    use rusterix::server::nodes::{Choice, Step, Then};

    let steps = choices_per_step
        .iter()
        .enumerate()
        .map(|(step, count)| Step {
            name: format!("step{step}"),
            text: format!("Line {step}"),
            choices: (0..*count)
                .map(|choice| Choice {
                    label: format!("choice{choice}"),
                    condition: String::new(),
                    then: Then::End,
                })
                .collect(),
        })
        .collect();
    let conversation = Conversation {
        entry: "step0".into(),
        steps,
    };
    let data = serde_json::to_value(&conversation).unwrap();
    let mut row = GraphRow::new(
        "Conversation",
        GraphControlValue::Custom {
            kind: "conversation".into(),
            data,
        },
    );
    row.key = Some("conversation".into());
    dock.doc
        .nodes
        .iter_mut()
        .find(|node| node.id == talk)
        .unwrap()
        .rows = vec![row];
}

/// A conversation taller than its panel still has to be editable: the columns
/// scroll, the consequence row is reachable, and a click that misses a clipped
/// row can never close the editor.
#[test]
fn a_small_dock_scrolls_and_edits_every_conversation_field() {
    let (mut dock, mut ui, mut ctx, mut project, mut server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 760, 460), &mut ctx);
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");

    // Five steps, the first with six choices: the shipped Hideout2D shape.
    set_conversation(&mut dock, talk, &[6, 1, 1, 1, 1]);

    let (w, h) = (760., 460.);
    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [w, h]), "panel opens");

    // The right column overflows. Clicking where the clipped consequence row
    // sits must be handled by the panel, never treated as a click away.
    assert_ne!(
        dock.conversation
            .as_mut()
            .unwrap()
            .pointer_down([376., 431.]),
        ConversationClick::Outside,
        "a click inside the panel must not close the editor"
    );

    // The wheel over the panel scrolls its fields, not the graph behind it.
    dock.editor.cursor = [500., 300.];
    let view_id = ui.get_render_view(VIEW).unwrap().id().clone();
    let pan_before = dock.editor.viewport.pan;
    dock.handle_event(
        &TheEvent::RenderViewScrollBy(view_id.clone(), Vec2::new(0, 1000)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert_eq!(dock.editor.viewport.pan, pan_before, "the graph stays put");

    // The deepest control is the choice's "then" button. After the wheel it is
    // inside the panel, and clicking it cycles the outcome instead of a script.
    let then_rect = dock
        .conversation
        .as_ref()
        .unwrap()
        .then_rect(0, 0)
        .expect("the then row");
    assert_eq!(
        dock.conversation
            .as_mut()
            .unwrap()
            .pointer_down([then_rect.origin[0] + 4., then_rect.origin[1] + 4.]),
        ConversationClick::Handled
    );
    assert!(
        matches!(
            dock.conversation.as_ref().unwrap().conversation().steps[0].choices[0].then,
            rusterix::server::nodes::Then::Go { .. }
        ),
        "the first click turns End into a jump"
    );

    // Through the dock, a text field opens the shared overlay and keeps the panel.
    let condition = ConversationField::ChoiceCondition(0, 0);
    let rect = dock.conversation.as_ref().unwrap().field_rect(condition);
    dock.handle_event(
        &TheEvent::RenderViewClicked(
            view_id,
            Vec2::new(
                (rect.origin[0] + 6.) as i32,
                (rect.origin[1] + 6.) as i32,
            ),
        ),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(dock.conversation.is_some(), "the panel stays open");
    let overlay = dock.text_overlay.as_mut().expect("the field opens");
    assert!(
        overlay.rect.size[1] >= 100.,
        "the editor has room for several lines, got {:?}",
        overlay.rect
    );
    overlay.input(GraphTextInput::SelectAll);
    overlay.paste("self.flag");

    if let Ok(path) = std::env::var("ELDIRON_TALK_SNAPSHOT") {
        let mut pixels = vec![0; 1200 * 650 * 4];
        ui.draw(&mut pixels, &mut ctx);
        dock.render(&mut ui, &mut ctx);
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            path,
            TheRGBABuffer::from(pixels, 1200, 650).to_png().unwrap(),
        )
        .unwrap();
    }

    // Committing writes the edit and the graph keeps its node.
    dock.close_conversation(&mut project, true);
    assert_eq!(dock.doc.nodes.len(), 1, "the node survives the edit");
    let node = dock.doc.nodes.iter().find(|node| node.id == talk).unwrap();
    let written = NodesDock::conversation_of(node).expect("conversation");
    assert_eq!(written.steps[0].choices[0].condition, "self.flag");
    assert!(matches!(
        written.steps[0].choices[0].then,
        rusterix::server::nodes::Then::Go { .. }
    ));
}

/// Even in a dock smaller than the conversation, the panel stays inside the
/// view and every field can still be scrolled to.
#[test]
fn a_tiny_dock_still_scrolls_to_the_consequence() {
    let (mut dock, mut ui, mut ctx, mut project, mut server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 420, 300), &mut ctx);
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");
    set_conversation(&mut dock, talk, &[6, 1, 1, 1, 1]);

    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [420., 300.]), "panel opens");

    // The panel never spills past the view, so the far corner is away from it.
    assert_eq!(
        dock.conversation
            .as_mut()
            .unwrap()
            .pointer_down([419., 299.]),
        ConversationClick::Outside
    );

    dock.conversation
        .as_mut()
        .unwrap()
        .scroll_at([320., 150.], 10000.);
    let then_rect = dock
        .conversation
        .as_ref()
        .unwrap()
        .then_rect(0, 0)
        .expect("the then row is reachable in a tiny dock");
    assert_eq!(
        dock.conversation
            .as_mut()
            .unwrap()
            .pointer_down([then_rect.origin[0] + 4., then_rect.origin[1] + 4.]),
        ConversationClick::Handled,
        "the consequence control is reachable in a tiny dock"
    );
    assert_eq!(dock.doc.nodes.len(), 1);

    // Clicking away closes the panel but keeps the graph: there is always a way
    // back in by double-clicking the node.
    let view_id = ui.get_render_view(VIEW).unwrap().id().clone();
    dock.handle_event(
        &TheEvent::RenderViewClicked(view_id.clone(), Vec2::new(4, 4)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(dock.conversation.is_none(), "clicking away closes the panel");
    assert_eq!(dock.doc.nodes.len(), 1, "the graph is untouched");
    dock.handle_event(
        &TheEvent::RenderViewClicked(view_id, Vec2::new(4, 4)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert_eq!(dock.doc.nodes.len(), 1);
}

/// A long step list scrolls on its own, and the last step stays reachable.
#[test]
fn a_long_step_list_scrolls_on_the_left() {
    let (mut dock, mut ui, mut ctx, _project, _server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 760, 460), &mut ctx);
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");
    let count = 16;
    set_conversation(&mut dock, talk, &vec![1; count]);

    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [760., 460.]), "panel opens");

    // Before scrolling, the same spot belongs to a middle step.
    dock.conversation
        .as_mut()
        .unwrap()
        .pointer_down([120., 370.]);
    assert_ne!(dock.conversation.as_ref().unwrap().selected_step(), count - 1);

    // The wheel over the steps column scrolls it, and the last step is there.
    dock.conversation
        .as_mut()
        .unwrap()
        .scroll_at([120., 300.], 1000.);
    dock.conversation
        .as_mut()
        .unwrap()
        .pointer_down([120., 370.]);
    assert_eq!(
        dock.conversation.as_ref().unwrap().selected_step(),
        count - 1,
        "the last step is reachable after scrolling"
    );
}

/// A conversation edit is a normal dock change: undo restores the previous
/// conversation and redo brings the edit back, so a mistake is recoverable.
#[test]
fn a_conversation_edit_is_undoable() {
    let (mut dock, mut ui, mut ctx, mut project, mut server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 900, 620), &mut ctx);
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");
    set_conversation(&mut dock, talk, &[3]);
    dock.store(&mut project, true);
    let before = dock.doc.clone();

    let view_id = ui.get_render_view(VIEW).unwrap().id().clone();
    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [900., 620.]), "panel opens");

    let condition = ConversationField::ChoiceCondition(0, 0);
    let rect = dock.conversation.as_ref().unwrap().field_rect(condition);
    dock.handle_event(
        &TheEvent::RenderViewClicked(
            view_id,
            Vec2::new((rect.origin[0] + 6.) as i32, (rect.origin[1] + 6.) as i32),
        ),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    let overlay = dock.text_overlay.as_mut().expect("the field opens");
    overlay.input(GraphTextInput::SelectAll);
    overlay.paste("self.flag");
    dock.close_conversation(&mut project, true);

    assert_ne!(dock.doc, before, "the edit landed");
    assert!(dock.supports_undo());
    dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
    assert_eq!(dock.doc, before, "undo restores the conversation");
    dock.redo(&mut ui, &mut ctx, &mut project, &mut server);
    assert_ne!(dock.doc, before, "redo brings the edit back");
}

/// A choice can be turned into "leave through a port and continue at a step",
/// which is how the old `set ...; go ...` reads as graph.
#[test]
fn a_choice_can_leave_through_a_port_and_continue_at_a_step() {
    use rusterix::server::nodes::Then;

    let (mut dock, mut ui, mut ctx, _project, _server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 900, 620), &mut ctx);
    add(&mut dock, "talk");
    let talk = dock.editor.selected.expect("talk node");
    set_conversation(&mut dock, talk, &[2, 1]);

    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [900., 620.]), "panel opens");

    let then = dock.conversation.as_ref().unwrap().then_rect(0, 0).unwrap();
    // End -> Go to step -> Run output.
    for _ in 0..2 {
        dock.conversation
            .as_mut()
            .unwrap()
            .pointer_down([then.origin[0] + 4., then.origin[1] + 4.]);
    }
    assert!(
        matches!(
            dock.conversation.as_ref().unwrap().conversation().steps[0].choices[0].then,
            Then::Out { slot: 0, .. }
        ),
        "two clicks reach the output form"
    );

    let resume = dock
        .conversation
        .as_ref()
        .unwrap()
        .then_resume_rect(0, 0)
        .expect("the continue-at row");
    dock.conversation
        .as_mut()
        .unwrap()
        .pointer_down([resume.origin[0] + 4., resume.origin[1] + 4.]);
    assert_eq!(
        dock.conversation.as_ref().unwrap().conversation().steps[0].choices[0].then,
        Then::Out {
            slot: 0,
            resume: "step0".into()
        },
        "the continue-at control cycles through the steps"
    );
}

/// Escape closes the conversation panel, like the other overlays.
#[test]
fn escape_closes_the_conversation_panel() {
    let (mut dock, mut ui, mut ctx, mut project, mut server) = live_dock();
    add(&mut dock, "talk");
    let point = dock.editor.viewport.to_screen([
        dock.doc.nodes[0].position[0] + 20.,
        dock.doc.nodes[0].position[1] + 20.,
    ]);
    assert!(dock.open_conversation(point, [900., 600.]), "panel opens");
    let view_id = ui.get_render_view(VIEW).unwrap().id().clone();
    ctx.ui.set_focus(&view_id);
    dock.handle_event(
        &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)),
        &mut ui,
        &mut ctx,
        &mut project,
        &mut server,
    );
    assert!(dock.conversation.is_none(), "Escape has to close the panel");
    assert_eq!(dock.doc.nodes.len(), 1, "the node stays");
}

/// Six consequence ports on a one-row node have to be spread over the whole
/// right side, so the labels do not pile up.
#[test]
fn a_talk_node_spreads_its_consequence_ports() {
    use rusterix::server::nodes::CONSEQUENCE_SLOTS;

    let (mut dock, mut ui, mut ctx, _project, _server) = live_dock();
    ui.get_render_view(VIEW)
        .unwrap()
        .set_dim(TheDim::new(0, 0, 900, 600), &mut ctx);
    add(&mut dock, "talk");
    let position = |key: &str| {
        dock.doc.nodes[0]
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some(key))
            .map(|port| port.position)
    };
    let outs: Vec<f32> = (0..CONSEQUENCE_SLOTS)
        .map(|slot| position(&format!("out:{slot}")).expect("consequence port"))
        .collect();
    assert!(
        outs.windows(2).all(|pair| pair[1] > pair[0]),
        "ports are ordered, got {outs:?}"
    );
    assert!(
        outs[0] <= 0.2 && *outs.last().unwrap() >= 0.7,
        "ports are spread over the side, got {outs:?}"
    );
    assert!(
        position("done").unwrap() > *outs.last().unwrap(),
        "the done port sits past the last consequence port"
    );

    dock.render(&mut ui, &mut ctx);
    if let Ok(path) = std::env::var("ELDIRON_TALK_PORTS_SNAPSHOT") {
        let png = ui
            .get_render_view(VIEW)
            .unwrap()
            .render_buffer_mut()
            .to_png()
            .unwrap();
        std::fs::write(path, png).unwrap();
    }
}
