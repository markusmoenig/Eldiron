use super::*;

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
            let pos = editor.viewport.to_screen(doc.nodes[0].port_position(p));
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
    let r = doc.nodes[0].row_rect(0);
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
    let r = doc.nodes[0].row_rect(0);
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
        .to_screen(doc.nodes[1].port_position(&doc.nodes[1].ports[0]));
    let end = e
        .viewport
        .to_screen(doc.nodes[0].port_position(&doc.nodes[0].ports[0]));
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
        .to_screen(doc.nodes[0].port_position(&doc.nodes[0].ports[0]));
    let b = e
        .viewport
        .to_screen(doc.nodes[1].port_position(&doc.nodes[1].ports[0]));
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
    let old = n.port_position(&n.ports[0]);
    n.rows.swap(0, 1);
    assert_eq!(n.port_position(&n.ports[0])[1], old[1] + 62.);
    let json = serde_json::to_string(&doc).unwrap();
    assert_eq!(serde_json::from_str::<GraphDocument>(&json).unwrap(), doc);
}

fn focus_text(doc: &mut GraphDocument, editor: &mut GraphEditor) {
    let r = doc.nodes[0].row_rect(0);
    let p = editor
        .viewport
        .to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
    editor.pointer_down(doc, p, &BasicGraphControls);
    editor.pointer_up(doc, p, &AllowGraphConnections);
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
fn text_navigation_deletes_graphemes_and_escape_restores_original() {
    let mut doc = graph();
    doc.nodes[0].rows.push(GraphRow::new(
        "Area",
        GraphControlValue::Text("garden".into()),
    ));
    let before = doc.clone();
    let mut e = GraphEditor::default();
    focus_text(&mut doc, &mut e);
    e.text_input(&mut doc, GraphTextInput::Insert("a👩‍🚀e\u{301}".into()));
    e.text_input(&mut doc, GraphTextInput::Backspace);
    assert_eq!(
        doc.nodes[0].rows[0].value,
        GraphControlValue::Text("a👩‍🚀".into())
    );
    e.text_input(&mut doc, GraphTextInput::Left { extend: false });
    e.text_input(&mut doc, GraphTextInput::Delete);
    assert_eq!(
        doc.nodes[0].rows[0].value,
        GraphControlValue::Text("a".into())
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
        size: doc.nodes[0].rect().size,
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
