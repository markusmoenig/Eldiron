use super::*;

/// Geometry in the tests uses the standard node metrics.
fn m() -> GraphMetrics {
    GraphMetrics::STANDARD
}

fn graph() -> GraphDocument {
    let mut a = GraphNode::new("A", [0., 0.], [0; 4]);
    let mut b = GraphNode::new("B", [400., 0.], [0; 4]);
    a.ports.push(GraphPort::new(
        "Out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    b.ports.push(GraphPort::new(
        "In",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    GraphDocument {
        nodes: vec![a, b],
        ..Default::default()
    }
}
#[test]
fn zoom_preserves_cursor_anchor_and_round_trip() {
    let mut view = GraphViewport::default();
    let cursor = [372., 219.];
    let point = view.to_graph(cursor);
    for factor in [1.17, 0.41, 8., 0.001, 1.4] {
        view.zoom_at(cursor, factor);
        let actual = view.to_screen(point);
        assert!((actual[0] - cursor[0]).abs() < 0.001);
        assert!((actual[1] - cursor[1]).abs() < 0.001);
    }
    let before = view.zoom();
    view.zoom_at(cursor, f32::NAN);
    assert_eq!(before, view.zoom());
}
#[test]
fn ports_all_sides_use_shared_hit_geometry_at_every_zoom() {
    let mut doc = graph();
    doc.nodes[0].ports.clear();
    for side in [
        PortSide::Left,
        PortSide::Right,
        PortSide::Top,
        PortSide::Bottom,
    ] {
        doc.nodes[0]
            .ports
            .push(GraphPort::new("", PortDirection::Output, side, 0.5));
    }
    let mut editor = GraphEditor::default();
    for factor in [0.25, 3., 4.] {
        editor.viewport.zoom_at([0., 0.], factor);
        for p in &doc.nodes[0].ports {
            let pos = editor
                .viewport
                .to_screen(doc.nodes[0].port_position(p, &m()));
            assert_eq!(editor.port_at(&doc, pos), Some(p.id));
        }
    }
}
#[test]
fn controls_capture_drag_without_moving_node_and_undo_as_one_edit() {
    let mut doc = graph();
    doc.nodes[0].rows.push(GraphRow::new(
        "Speed",
        GraphControlValue::Number {
            value: 0.,
            min: 0.,
            max: 10.,
            step: 1.,
        },
    ));
    let before = doc.clone();
    let mut editor = GraphEditor::default();
    editor.viewport.zoom_at([0., 0.], 0.5);
    let r = doc.nodes[0].row_rect(0, &m());
    let start = editor
        .viewport
        .to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
    let end = editor
        .viewport
        .to_screen([r.origin[0] + r.size[0], r.origin[1] + 10.]);
    editor.pointer_down(&mut doc, start, &BasicGraphControls);
    editor.pointer_move(&mut doc, end, &BasicGraphControls);
    editor.pointer_up(&mut doc, end, &AllowGraphConnections);
    assert_eq!(doc.nodes[0].position, before.nodes[0].position);
    let edits = editor.take_edits();
    assert_eq!(edits.len(), 1);
    let after = doc.clone();
    edits[0].apply(&mut doc, false);
    assert_eq!(doc, before);
    edits[0].apply(&mut doc, true);
    assert_eq!(doc, after);
}
#[test]
fn cancellation_restores_control_value() {
    let mut doc = graph();
    doc.nodes[0]
        .rows
        .push(GraphRow::new("Enabled", GraphControlValue::Toggle(false)));
    let before = doc.clone();
    let mut e = GraphEditor::default();
    let r = doc.nodes[0].row_rect(0, &m());
    let p = e.viewport.to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
    e.pointer_down(&mut doc, p, &BasicGraphControls);
    assert_ne!(doc, before);
    e.cancel(&mut doc);
    assert_eq!(doc, before);
    assert!(e.take_edits().is_empty());
}
#[test]
fn connect_in_reverse_then_delete_and_undo() {
    let mut doc = graph();
    let mut e = GraphEditor::default();
    let start = e
        .viewport
        .to_screen(doc.nodes[1].port_position(&doc.nodes[1].ports[0], &m()));
    let end = e
        .viewport
        .to_screen(doc.nodes[0].port_position(&doc.nodes[0].ports[0], &m()));
    e.pointer_down(&mut doc, start, &BasicGraphControls);
    e.pointer_up(&mut doc, end, &AllowGraphConnections);
    assert_eq!(doc.connections.len(), 1);
    let c = doc.connections[0].clone();
    assert!(doc.validate_connection(c.from, c.to).is_err());
    e.selected_connection = Some(c.id);
    e.delete_connection(&mut doc);
    assert!(doc.connections.is_empty());
    let edits = e.take_edits();
    assert_eq!(edits.len(), 2);
    edits[1].apply(&mut doc, false);
    assert_eq!(doc.connections[0], c);
}
#[test]
fn host_validation_can_reject_structurally_valid_wire() {
    struct Reject;
    impl GraphConnectionPolicy for Reject {
        fn validate(&self, _: &GraphDocument, _: GraphId, _: GraphId) -> Result<(), String> {
            Err("Cycle rejected".into())
        }
    }
    let mut doc = graph();
    let mut e = GraphEditor::default();
    let a = e
        .viewport
        .to_screen(doc.nodes[0].port_position(&doc.nodes[0].ports[0], &m()));
    let b = e
        .viewport
        .to_screen(doc.nodes[1].port_position(&doc.nodes[1].ports[0], &m()));
    e.pointer_down(&mut doc, a, &BasicGraphControls);
    e.pointer_up(&mut doc, b, &Reject);
    assert!(doc.connections.is_empty());
    assert_eq!(e.error.as_deref(), Some("Cycle rejected"));
}
#[test]
fn row_ports_follow_stable_identity_after_reorder_and_serialization() {
    let mut doc = graph();
    let n = &mut doc.nodes[0];
    n.rows
        .push(GraphRow::new("One", GraphControlValue::Label("one".into())));
    n.rows
        .push(GraphRow::new("Two", GraphControlValue::Label("two".into())));
    n.ports[0].row = Some(n.rows[0].id);
    let old = n.port_position(&n.ports[0], &m());
    n.rows.swap(0, 1);
    assert_eq!(
        n.port_position(&n.ports[0], &m())[1],
        old[1] + MIN_ROW_PITCH
    );
    let json = serde_json::to_string(&doc).unwrap();
    assert_eq!(serde_json::from_str::<GraphDocument>(&json).unwrap(), doc);
}

fn focus_text_at(doc: &mut GraphDocument, editor: &mut GraphEditor, x: f32) {
    let r = doc.nodes[0].row_rect(0, &m());
    let p = editor
        .viewport
        .to_screen([r.origin[0] + x, r.origin[1] + 10.]);
    editor.pointer_down(doc, p, &BasicGraphControls);
    editor.pointer_up(doc, p, &AllowGraphConnections);
}
fn focus_text(doc: &mut GraphDocument, editor: &mut GraphEditor) {
    focus_text_at(doc, editor, 10.);
}
#[test]
fn clicking_a_text_row_places_the_caret_instead_of_selecting_all() {
    let mut doc = graph();
    doc.nodes[0].rows.push(GraphRow::new(
        "Area",
        GraphControlValue::Text("garden".into()),
    ));
    let mut e = GraphEditor::default();
    e.viewport.zoom_at([0., 0.], 0.6);
    focus_text_at(&mut doc, &mut e, 40.);
    let focus = e.text_focus().expect("focus");
    assert!(
        focus.selection().is_empty(),
        "clicking must not select the whole value"
    );
    assert!(focus.caret() > 0 && focus.caret() < "garden".len());
}
#[test]
fn text_focus_consumes_edits_and_commits_one_undo_on_blur() {
    let mut doc = graph();
    doc.nodes[0].rows.push(GraphRow::new(
        "Area",
        GraphControlValue::Text("garden".into()),
    ));
    let before = doc.clone();
    let mut e = GraphEditor::default();
    e.viewport.zoom_at([0., 0.], 0.6);
    focus_text(&mut doc, &mut e);
    assert!(e.text_input(&mut doc, GraphTextInput::Insert("office".into())));
    e.text_input(&mut doc, GraphTextInput::Insert(" courtyard".into()));
    assert!(e.take_edits().is_empty());
    e.pointer_down(&mut doc, [900., 700.], &BasicGraphControls);
    assert!(e.text_focus().is_none());
    let edits = e.take_edits();
    assert_eq!(edits.len(), 1);
    assert_eq!(doc.nodes[0].position, before.nodes[0].position);
    edits[0].apply(&mut doc, false);
    assert_eq!(doc, before);
}
#[test]
fn text_focus_types_into_a_list_cell_and_undoes_the_whole_row() {
    let mut doc = graph();
    doc.nodes[0].rows.push(list_row(vec![vec![
        GraphControlValue::Text("mode".into()),
        GraphControlValue::Toggle(true),
    ]]));
    let before = doc.clone();
    let mut e = GraphEditor::default();
    let r = doc.nodes[0].row_rect(0, &m());
    let point = [
        r.origin[0] + 7.,
        r.origin[1] + LIST_HEADER_PITCH + LIST_ROW_PITCH * 0.5,
    ];
    let p = e.viewport.to_screen(point);
    e.pointer_down(&mut doc, p, &BasicGraphControls);
    e.pointer_up(&mut doc, p, &AllowGraphConnections);
    assert_eq!(e.text_focus().and_then(|focus| focus.cell), Some((0, 0)));
    assert_eq!(e.text_focus().map(|focus| focus.caret()), Some(0));

    assert!(e.text_input(&mut doc, GraphTextInput::Insert("alive".into())));
    let GraphControlValue::List { rows, .. } = &doc.nodes[0].rows[0].value else {
        panic!("list expected");
    };
    assert_eq!(rows[0][0], GraphControlValue::Text("alivemode".into()));

    e.pointer_down(&mut doc, [900., 700.], &BasicGraphControls);
    let edits = e.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut doc, false);
    assert_eq!(doc, before);
}
#[test]
fn text_navigation_deletes_graphemes_and_escape_restores_original() {
    let mut doc = graph();
    doc.nodes[0].rows.push(GraphRow::new(
        "Area",
        GraphControlValue::Text("garden".into()),
    ));
    let before = doc.clone();
    let mut e = GraphEditor::default();
    // Caret past the end, so inserts append instead of replacing the value.
    focus_text_at(&mut doc, &mut e, 200.);
    e.text_input(&mut doc, GraphTextInput::Insert("a👩‍🚀e\u{301}".into()));
    e.text_input(&mut doc, GraphTextInput::Backspace);
    assert_eq!(
        doc.nodes[0].rows[0].value,
        GraphControlValue::Text("gardena👩‍🚀".into())
    );
    e.text_input(&mut doc, GraphTextInput::Left { extend: false });
    e.text_input(&mut doc, GraphTextInput::Delete);
    assert_eq!(
        doc.nodes[0].rows[0].value,
        GraphControlValue::Text("gardena".into())
    );
    e.text_input(&mut doc, GraphTextInput::Cancel);
    assert_eq!(doc, before);
    assert!(e.take_edits().is_empty());
    assert!(!e.text_input(&mut doc, GraphTextInput::Backspace));
}
#[test]
fn context_border_is_preserved_when_executing_or_selected() {
    struct Paint(Vec<(GraphRect, GraphColor)>);
    impl GraphPainter for Paint {
        fn round_rect(&mut self, r: GraphRect, _: f32, c: GraphColor) {
            self.0.push((r, c));
        }
        fn text(&mut self, _: GraphRect, _: &str, _: f32, _: GraphColor) {}
        fn curve(&mut self, _: [Point; 4], _: f32, _: GraphColor) {}
        fn preview(&mut self, _: GraphRect, _: &str) {}
    }
    struct Context(GraphCondition);
    impl GraphContext for Context {
        fn observe(&self, _: &GraphNode) -> GraphObservation {
            GraphObservation {
                condition: self.0,
                execution: GraphExecution::Running,
                text: "Status retained".into(),
            }
        }
    }
    let doc = graph();
    let mut e = GraphEditor::default();
    e.selected = Some(doc.nodes[0].id);
    let theme = GraphTheme::default();
    let r = GraphRect {
        origin: e.viewport.to_screen(doc.nodes[0].position),
        size: doc.nodes[0].rect(&m()).size,
    };
    for (condition, color) in [
        (GraphCondition::True, theme.yes),
        (GraphCondition::False, theme.no),
    ] {
        let mut painter = Paint(vec![]);
        e.paint(
            &doc,
            &Context(condition),
            &BasicGraphControls,
            &mut painter,
            [1000., 600.],
            &theme,
        );
        assert!(painter.0.contains(&(r, color)));
    }
}
#[test]
fn fast_fill_clips_and_covers_subpixel_rectangles_once() {
    let bytes = crate::Embedded::get("fonts/Roboto-Bold.ttf").unwrap();
    let font =
        fontdue::Font::from_bytes(bytes.data.as_ref(), fontdue::FontSettings::default()).unwrap();
    let mut resources = GraphRasterResources::new(font);
    let mut pixels = vec![0; 4 * 4 * 4 + 16];
    pixels[64..].fill(123);
    let mut painter = RasterGraphPainter::new(&mut pixels, 4, 4, 1., &mut resources, &());
    painter.round_rect(
        GraphRect {
            origin: [-2., -2.],
            size: [8., 8.],
        },
        0.,
        [0, 0, 0, 255],
    );
    painter.round_rect(
        GraphRect {
            origin: [1.25, 1.],
            size: [0.5, 1.],
        },
        0.,
        [200, 100, 0, 255],
    );
    assert_eq!(&pixels[20..24], &[100, 50, 0, 255]);
    assert!(pixels[64..].iter().all(|b| *b == 123));
}

#[test]
fn compact_picker_scrolls_smoothly_and_tracks_hover_after_filtering() {
    let mut picker = GraphPicker::compact(
        GraphRect {
            origin: [0., 0.],
            size: [200., 24.],
        },
        [220., 120.],
        (0..20)
            .map(|i| GraphPickerItem {
                id: i.to_string(),
                label: format!("Event {i}"),
            })
            .collect(),
    );
    assert!(picker.hover(Some([10., picker.origin[1] + 5.])));
    assert_eq!(picker.hovered.as_deref(), Some("0"));
    picker.scroll_pixels(12.);
    assert_eq!(picker.scroll, 0);
    picker.scroll_pixels(12.);
    assert_eq!(picker.scroll, 1);
    assert_eq!(picker.hovered.as_deref(), Some("1"));
    picker.scroll_by(1000);
    assert_eq!(picker.scroll, 17);
    picker.scroll_by(-1000);
    assert_eq!(picker.scroll, 0);
    picker.type_char('1');
    picker.type_char('9');
    picker.hover(Some([10., picker.origin[1] + 29.]));
    assert_eq!(picker.hovered.as_deref(), Some("19"));
    assert_eq!(
        picker.pick([10., picker.origin[1] + 29.]),
        Some("19".into())
    );
    assert!(picker.hover(None));
    assert!(picker.hovered.is_none());
}

fn list_row(rows: Vec<Vec<GraphControlValue>>) -> GraphRow {
    GraphRow::new(
        "Attributes",
        GraphControlValue::List {
            columns: vec![
                GraphListColumn {
                    id: "attribute".into(),
                    label: "Attribute".into(),
                    control: GraphControlValue::Text(String::new()),
                },
                GraphListColumn {
                    id: "value".into(),
                    label: "Value".into(),
                    control: GraphControlValue::Toggle(false),
                },
            ],
            rows,
        },
    )
}
#[test]
fn list_rows_grow_the_node_and_round_trip() {
    let mut doc = graph();
    doc.nodes[0].rows.push(list_row(vec![]));
    let empty = doc.nodes[0].height(&m());
    doc.nodes[0].rows[0].value = GraphControlValue::List {
        columns: match &doc.nodes[0].rows[0].value {
            GraphControlValue::List { columns, .. } => columns.clone(),
            _ => unreachable!(),
        },
        rows: vec![vec![
            GraphControlValue::Text("mode".into()),
            GraphControlValue::Toggle(true),
        ]],
    };
    let one = doc.nodes[0].height(&m());
    assert!((one - empty - LIST_ROW_PITCH).abs() < 0.001);

    let text = serde_json::to_string(&doc).unwrap();
    let restored: GraphDocument = serde_json::from_str(&text).unwrap();
    assert_eq!(restored, doc);
}
#[test]
fn list_control_adds_cycles_and_deletes_entries() {
    let mut doc = graph();
    doc.nodes[0].rows.push(list_row(vec![vec![
        GraphControlValue::Text("mode".into()),
        GraphControlValue::Toggle(true),
    ]]));
    let mut editor = GraphEditor::default();
    let rect = doc.nodes[0].row_rect(0, &m());
    let cell = |column: usize, row: usize| {
        let width = rect.size[0] * LIST_DELETE_FRACTION / 2.;
        [
            rect.origin[0] + width * (column as f32 + 0.5),
            rect.origin[1] + LIST_HEADER_PITCH + LIST_ROW_PITCH * (row as f32 + 0.5),
        ]
    };
    let click = |editor: &mut GraphEditor, doc: &mut GraphDocument, point: [f32; 2]| {
        let screen = editor.viewport.to_screen(point);
        editor.pointer_down(doc, screen, &BasicGraphControls);
        editor.pointer_up(doc, screen, &AllowGraphConnections);
        let _ = editor.take_edits();
    };

    // Adding: click the trailing add-row.
    click(&mut editor, &mut doc, [rect.origin[0] + 10., cell(0, 1)[1]]);
    let GraphControlValue::List { rows, .. } = &doc.nodes[0].rows[0].value else {
        panic!("list expected");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][0], GraphControlValue::Text(String::new()));
    assert_eq!(rows[1][1], GraphControlValue::Toggle(false));

    // Editing: clicking a toggle cell flips it.
    click(&mut editor, &mut doc, cell(1, 0));
    let GraphControlValue::List { rows, .. } = &doc.nodes[0].rows[0].value else {
        panic!("list expected");
    };
    assert_eq!(rows[0][1], GraphControlValue::Toggle(false));

    // Deleting: click the far right of a row.
    click(
        &mut editor,
        &mut doc,
        [
            rect.origin[0] + rect.size[0] * (LIST_DELETE_FRACTION + 0.05),
            cell(0, 0)[1],
        ],
    );
    let GraphControlValue::List { rows, .. } = &doc.nodes[0].rows[0].value else {
        panic!("list expected");
    };
    assert_eq!(rows.len(), 1);
}

#[test]
fn terminals_on_a_parameter_less_node_clear_the_title_bar() {
    // A trigger such as On Area has no parameters, only terminals.
    let mut node = GraphNode::new("On Area", [0., 0.], [16, 112, 98, 255]);
    node.width = 240.;
    for position in [0.2, 0.4, 0.6, 0.8] {
        node.ports.push(GraphPort::new(
            "out",
            PortDirection::Output,
            PortSide::Right,
            position,
        ));
    }

    assert!(
        node.height(&m()) > HEADER_PITCH,
        "the node must have room for its terminals"
    );
    for port in &node.ports {
        let y = node.port_position(port, &m())[1] - node.position[1];
        assert!(
            y > HEADER_PITCH,
            "terminal at {y} overlaps the {} header",
            HEADER_PITCH
        );
        assert!(
            y < node.height(&m()),
            "terminal at {y} falls outside the node"
        );
    }
    // Order is preserved: the first terminal still sits above the last.
    let first = node.port_position(&node.ports[0], &m())[1];
    let last = node.port_position(&node.ports[3], &m())[1];
    assert!(first < last);
}

fn branch_definitions() -> GraphDefinitions {
    let mut definitions = GraphDefinitions::default();
    let mut trigger = GraphNode::new("Trigger", [0., 0.], [0; 4]);
    trigger.ports.push(GraphPort::new(
        "out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    definitions
        .register_node(GraphNodeDefinition::from_template(
            "event", "Events", &trigger,
        ))
        .unwrap();
    let mut action = GraphNode::new("Action", [0., 0.], [0; 4]);
    action.ports.push(GraphPort::new(
        "in",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    action.ports.push(GraphPort::new(
        "out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    definitions
        .register_node(GraphNodeDefinition::from_template(
            "say", "Actions", &action,
        ))
        .unwrap();
    definitions
}

/// `Trigger -> Say` plus a loose Say with no trigger, and a shared node.
fn branch_document() -> (GraphDocument, GraphDefinitions, GraphId, GraphId, GraphId) {
    let definitions = branch_definitions();
    let mut trigger = GraphNode::new("Trigger", [0., 0.], [0; 4]);
    trigger.definition = Some("event".into());
    let trigger_out = GraphPort::new("out", PortDirection::Output, PortSide::Right, 0.5);
    let trigger_out_id = trigger_out.id;
    trigger.ports.push(trigger_out);

    let mut say = GraphNode::new("Say", [400., 0.], [0; 4]);
    say.definition = Some("say".into());
    let say_in = GraphPort::new("in", PortDirection::Input, PortSide::Left, 0.5);
    let say_in_id = say_in.id;
    say.ports.push(say_in);

    let mut loose = GraphNode::new("Loose", [800., 0.], [0; 4]);
    loose.definition = Some("say".into());
    loose.ports.push(GraphPort::new(
        "in",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));

    let doc = GraphDocument {
        version: 1,
        nodes: vec![trigger.clone(), say.clone(), loose.clone()],
        connections: vec![GraphConnection {
            id: uuid::Uuid::new_v4(),
            from: trigger_out_id,
            to: say_in_id,
        }],
    };
    (doc, definitions, trigger.id, say.id, loose.id)
}

#[test]
fn branches_follow_reachability_from_triggers() {
    let (doc, definitions, trigger, say, loose) = branch_document();
    let branches = graph_branches(&doc, &definitions);
    assert_eq!(branches.branches.len(), 1);
    assert_eq!(branches.branches[0].root, trigger);
    assert!(branches.branches[0].nodes.contains(&say));
    assert!(!branches.branches[0].nodes.contains(&loose));
    assert!(branches.detached.contains(&loose));
    assert!(branches.shared_nodes().is_empty());
}

#[test]
fn hiding_a_branch_hides_its_nodes_and_picking() {
    let (doc, definitions, _trigger, say, loose) = branch_document();
    let branches = graph_branches(&doc, &definitions);
    let mut editor = GraphEditor::default();
    editor.set_visible(Some(branches.branches[0].nodes.clone()));
    assert!(editor.node_visible(say));
    assert!(!editor.node_visible(loose));
    let connection = doc.connections[0].id;
    assert!(editor.connection_visible(&doc, connection));
    // A node picked while hidden stays unselectable.
    let loose_rect = doc.nodes[2].rect(&m());
    let point = editor
        .viewport
        .to_screen([loose_rect.origin[0] + 10., loose_rect.origin[1] + 10.]);
    editor.pointer_down(&mut doc.clone(), point, &BasicGraphControls);
    assert!(editor.selected.is_none());
    // Showing everything again makes it pickable.
    editor.set_visible(None);
    assert!(editor.node_visible(loose));
}

#[test]
fn fit_to_nodes_centres_and_zooms_the_view() {
    let (mut doc, _definitions, trigger, say, _loose) = branch_document();
    let nodes: std::collections::HashSet<GraphId> = [trigger, say].into_iter().collect();
    // Move the branch far away; the fit has to bring it back to centre.
    for node in doc.nodes.iter_mut() {
        node.position[0] += 5000.;
        node.position[1] += 5000.;
    }
    let mut view = GraphViewport::default();
    view.fit_to_nodes(&doc, [1000., 800.], &nodes);

    let mut min = [f32::MAX; 2];
    let mut max = [f32::MIN; 2];
    for node in doc.nodes.iter().filter(|node| nodes.contains(&node.id)) {
        let rect = node.rect(&m());
        min[0] = min[0].min(rect.origin[0]);
        min[1] = min[1].min(rect.origin[1]);
        max[0] = max[0].max(rect.origin[0] + rect.size[0]);
        max[1] = max[1].max(rect.origin[1] + rect.size[1]);
    }
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let screen = view.to_screen(center);
    assert!(
        (screen[0] - 500.).abs() < 1. && (screen[1] - 400.).abs() < 1.,
        "the branch centres in the view, got {screen:?}"
    );
    assert!(view.zoom() > 0.1 && view.zoom() <= 1.5);
}

#[test]
fn layout_branch_places_steps_left_to_right_without_overlap() {
    let (mut doc, _definitions, trigger, say, loose) = branch_document();
    // Pile the two branch nodes on top of each other first.
    if let Some(node) = doc.nodes.iter_mut().find(|node| node.id == say) {
        node.position = [0., 0.];
    }
    let nodes: std::collections::HashSet<GraphId> = [trigger, say].into_iter().collect();
    assert!(layout_branch(&mut doc, &nodes));

    let trigger_node = doc.nodes.iter().find(|node| node.id == trigger).unwrap();
    let say_node = doc.nodes.iter().find(|node| node.id == say).unwrap();
    assert_eq!(
        trigger_node.position[0], 0.,
        "the trigger is the first column"
    );
    assert!(
        say_node.position[0] > trigger_node.position[0] + trigger_node.width,
        "the next step is to the right, not on top"
    );
    // Nodes outside the branch are left alone.
    let loose_node = doc.nodes.iter().find(|node| node.id == loose).unwrap();
    assert_eq!(loose_node.position, [800., 0.]);
    // Laying the same branch out twice changes nothing.
    assert!(!layout_branch(&mut doc, &nodes));
}

/// An action node with one input and one output, for layout tests.
fn step_node(name: &str) -> GraphNode {
    let mut node = GraphNode::new(name, [700., 700.], [0; 4]);
    node.definition = Some("say".into());
    node.ports.push(GraphPort::new(
        "in",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    node.ports.push(GraphPort::new(
        "out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    node
}

/// A trigger node with one output, for layout tests.
fn trigger_node() -> GraphNode {
    let mut trigger = GraphNode::new("Trigger", [700., 700.], [0; 4]);
    trigger.definition = Some("event".into());
    trigger.ports.push(GraphPort::new(
        "out",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    trigger
}

fn link(from: GraphId, to: GraphId) -> GraphConnection {
    GraphConnection {
        id: uuid::Uuid::new_v4(),
        from,
        to,
    }
}

#[test]
fn layout_branch_stacks_parallel_steps_without_overlap() {
    let trigger = trigger_node();
    let root_out_id = trigger.ports[0].id;

    // Root splits into A and B, which merge into C.
    let (a, b, c) = (step_node("A"), step_node("B"), step_node("C"));
    let doc = GraphDocument {
        version: 1,
        nodes: vec![trigger.clone(), a.clone(), b.clone(), c.clone()],
        connections: vec![
            link(root_out_id, a.ports[0].id),
            link(root_out_id, b.ports[0].id),
            link(a.ports[1].id, c.ports[0].id),
            link(b.ports[1].id, c.ports[0].id),
        ],
    };
    let mut doc = doc;
    let nodes: std::collections::HashSet<GraphId> =
        [trigger.id, a.id, b.id, c.id].into_iter().collect();
    assert!(layout_branch(&mut doc, &nodes));

    let position = |id: GraphId| doc.nodes.iter().find(|n| n.id == id).unwrap().position;
    assert_eq!(position(trigger.id)[0], 0., "the trigger heads the branch");
    assert_eq!(
        position(a.id)[0],
        position(b.id)[0],
        "A and B share a column"
    );
    assert!(
        position(c.id)[0] > position(a.id)[0],
        "the merge comes after"
    );
    assert!(
        position(a.id)[1] != position(b.id)[1],
        "stacked, not on top of each other"
    );
    for (i, one) in doc.nodes.iter().enumerate() {
        for other in doc.nodes.iter().skip(i + 1) {
            let a = one.rect(&m());
            let b = other.rect(&m());
            let apart = a.origin[0] + a.size[0] <= b.origin[0]
                || b.origin[0] + b.size[0] <= a.origin[0]
                || a.origin[1] + a.size[1] <= b.origin[1]
                || b.origin[1] + b.size[1] <= a.origin[1];
            assert!(apart, "{} overlaps {}", one.id, other.id);
        }
    }
}

#[test]
fn layout_branch_does_not_stretch_a_loop_back() {
    // Trigger -> Ask -> Reply -> Ask (a dialogue choice that re-opens the menu).
    // Following the loop would push Ask and Reply rightwards forever.
    let trigger = trigger_node();
    let root_out = trigger.ports[0].id;
    let (ask, reply, extra) = (step_node("Ask"), step_node("Reply"), step_node("Extra"));
    let mut doc = GraphDocument {
        version: 1,
        nodes: vec![trigger.clone(), ask.clone(), reply.clone(), extra.clone()],
        connections: vec![
            link(root_out, ask.ports[0].id),
            link(ask.ports[1].id, reply.ports[0].id),
            link(reply.ports[1].id, ask.ports[0].id),
            link(ask.ports[1].id, extra.ports[0].id),
        ],
    };
    let nodes: std::collections::HashSet<GraphId> = [trigger.id, ask.id, reply.id, extra.id]
        .into_iter()
        .collect();
    assert!(layout_branch(&mut doc, &nodes));

    let x = |id: GraphId| doc.nodes.iter().find(|n| n.id == id).unwrap().position[0];
    assert!(
        x(ask.id) > x(trigger.id),
        "the question comes after the event"
    );
    assert!(
        x(reply.id) > x(ask.id),
        "the reply comes after the question"
    );
    // Three execution columns, not one per loop turn.
    let rightmost = doc
        .nodes
        .iter()
        .filter(|n| nodes.contains(&n.id))
        .map(|n| n.position[0])
        .fold(f32::MIN, f32::max);
    assert!(
        rightmost <= 2. * (trigger.width + 300.),
        "the loop stays in three columns, rightmost was {rightmost}"
    );
    // Laying out again changes nothing: the loop does not creep.
    assert!(!layout_branch(&mut doc, &nodes));
}

fn list_column(label: &str) -> GraphListColumn {
    GraphListColumn {
        id: label.to_lowercase(),
        label: label.into(),
        control: GraphControlValue::Text(String::new()),
    }
}

/// A node shaped like a dialogue step: a line plus a table of choices.
fn dialogue_node() -> GraphNode {
    let mut node = step_node("Dialogue");
    node.rows.push(GraphRow::new(
        "text",
        GraphControlValue::Text("A long line of dialogue".into()),
    ));
    node.rows.push(GraphRow::new(
        "choices",
        GraphControlValue::List {
            columns: vec![list_column("Choice"), list_column("Condition")],
            rows: (0..6)
                .map(|i| {
                    vec![
                        GraphControlValue::Text(format!("choice {i}")),
                        GraphControlValue::Text(String::new()),
                    ]
                })
                .collect(),
        },
    ));
    node
}

#[test]
fn folding_shrinks_a_node() {
    let mut node = dialogue_node();
    let standard = node.height(&GraphMetrics::STANDARD);
    node.folded = true;
    let folded = node.height(&GraphMetrics::STANDARD);
    assert!(folded < standard, "folded {folded} vs {standard}");
    assert_eq!(node.rows.len(), 2, "folding keeps the parameters");

    // Terminals stay on the folded body so they can still be wired.
    let bottom = node.position[1] + folded;
    for port in &node.ports {
        let y = node.port_position(port, &GraphMetrics::STANDARD)[1];
        assert!(
            y >= node.position[1] - 0.01 && y <= bottom + 0.01,
            "terminal at {y} outside a folded node ({})",
            node.position[1]
        );
    }
}

#[test]
fn folds_survive_a_round_trip() {
    let mut doc = GraphDocument {
        version: 1,
        nodes: vec![dialogue_node()],
        connections: vec![],
    };
    doc.nodes[0].folded = true;
    let json = serde_json::to_string(&doc).unwrap();
    assert_eq!(serde_json::from_str::<GraphDocument>(&json).unwrap(), doc);

    // A graph saved before folding existed still loads with open nodes.
    let old = r#"{
        "version": 1,
        "nodes": [{
            "id": "6f1a1a1a-0000-4000-8000-000000000000",
            "title": "Old",
            "position": [0.0, 0.0],
            "width": 240.0,
            "color": [0, 0, 0, 255],
            "rows": [],
            "ports": []
        }],
        "connections": []
    }"#;
    let old: GraphDocument = serde_json::from_str(old).unwrap();
    assert!(!old.nodes[0].folded);
    assert_eq!(old.metrics(), GraphMetrics::STANDARD);

    // Saving an untouched graph does not add a folding key.
    let plain = serde_json::to_string(&old).unwrap();
    assert!(!plain.contains("\"folded\""), "{plain}");

    let mut legacy: serde_json::Value = serde_json::from_str(&plain).unwrap();
    legacy["compact"] = serde_json::Value::Bool(true);
    let migrated: GraphDocument = serde_json::from_value(legacy).unwrap();
    assert_eq!(migrated.metrics(), GraphMetrics::STANDARD);
    assert!(
        !serde_json::to_string(&migrated)
            .unwrap()
            .contains("\"compact\"")
    );
}

#[test]
fn clicking_the_title_chevron_folds_a_node() {
    let mut doc = GraphDocument {
        version: 1,
        nodes: vec![dialogue_node()],
        connections: vec![],
    };
    let mut editor = GraphEditor::default();
    let metrics = doc.metrics();
    let handle = doc.nodes[0].fold_handle(&metrics);
    let point = editor.viewport.to_screen([
        handle.origin[0] + handle.size[0] * 0.5,
        handle.origin[1] + handle.size[1] * 0.5,
    ]);
    editor.pointer_down(&mut doc, point, &BasicGraphControls);
    assert!(doc.nodes[0].folded, "the title chevron folds the node");
    editor.pointer_down(&mut doc, point, &BasicGraphControls);
    assert!(!doc.nodes[0].folded, "and unfolds it again");
}

#[test]
fn searchable_choice_targets_use_list_geometry_at_every_zoom() {
    let mut node = GraphNode::new("Input", [30., 50.], [0, 0, 0, 255]);
    let choices = GraphControlValue::Choice {
        options: vec!["one".into(), "two".into()],
        selected: 1,
    };
    node.rows.push(GraphRow::new(
        "Bindings",
        GraphControlValue::List {
            columns: vec![GraphListColumn {
                id: "command".into(),
                label: "Command".into(),
                control: choices.clone(),
            }],
            rows: vec![vec![choices]],
        },
    ));
    let doc = GraphDocument {
        version: 1,
        nodes: vec![node],
        connections: vec![],
    };
    let metrics = doc.metrics();
    let rect = doc.nodes[0].row_rect(0, &metrics);
    let point = [
        rect.origin[0] + 10.,
        rect.origin[1] + metrics.list_header + metrics.list_row * 0.5,
    ];
    for zoom in [0.25, 0.9, 2.] {
        let mut editor = GraphEditor::default();
        editor.viewport.zoom_at([0., 0.], zoom);
        let target = editor
            .choice_at(&doc, editor.viewport.to_screen(point))
            .unwrap();
        assert_eq!(target.cell, Some((0, 0)));
        assert_eq!(target.selected, 1);
        assert_eq!(target.options, vec!["one", "two"]);
    }
}
