#![cfg(all(feature = "ui", feature = "text"))]
use theframework::{
    paint::{PaintList, PaintOp},
    platform::*,
    text::TextSystem,
    text_edit::{EditorAction, EditorClipboard, EditorCommand, EditorResponse},
    thegraph::*,
};
use unicode_segmentation::UnicodeSegmentation;
fn key(physical: PhysicalKey) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    }
}
struct Fixture {
    doc: GraphDocument,
    graph: GraphEditor,
    fonts: TextSystem,
    session: GraphTextSession,
}
impl Fixture {
    fn new(text: &str) -> Self {
        let mut node = GraphNode::new("Draft", [24., 32.], [50, 90, 110, 255]);
        node.rows
            .push(GraphRow::new("Text", GraphControlValue::Text(text.into())));
        let mut doc = GraphDocument {
            nodes: vec![node],
            ..Default::default()
        };
        let mut graph = GraphEditor::default();
        let mut fonts = TextSystem::default();
        fonts.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        let mut session = GraphTextSession::default();
        let rect = doc.nodes[0].row_rect(0, &doc.metrics());
        let point = graph
            .viewport
            .to_screen([rect.origin[0] + 12., rect.origin[1] + 10.]);
        session
            .pointer_down(&mut graph, &mut doc, &mut fonts, point, &BasicGraphControls)
            .unwrap();
        session.input(
            &mut graph,
            &mut doc,
            &mut fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: point,
            },
            true,
        );
        Self {
            doc,
            graph,
            fonts,
            session,
        }
    }
    fn text(&self) -> &str {
        self.graph.text_focus().unwrap().value(&self.doc).unwrap()
    }
    fn input(&mut self, event: InputEvent) -> EditorResponse {
        self.session.input(
            &mut self.graph,
            &mut self.doc,
            &mut self.fonts,
            &event,
            true,
        )
    }
    fn command(&mut self, command: EditorCommand) -> EditorResponse {
        self.session.command(
            &mut self.graph,
            &mut self.doc,
            &mut self.fonts,
            command,
            true,
        )
    }
    fn state(&mut self) -> TextInputState {
        self.session
            .input_state(&mut self.graph, &self.doc, &mut self.fonts, true)
            .unwrap()
    }
    fn paint(&mut self) -> PaintList {
        let mut list = PaintList::default();
        self.session.paint(
            &mut self.graph,
            &self.doc,
            &NoContext,
            &BasicGraphControls,
            PaintGraphPainter {
                paint: &mut list,
                text: &mut self.fonts,
                images: &(),
                family: "Roboto",
            },
            [640., 400.],
            &GraphTheme::default(),
            true,
        );
        list.validate().unwrap();
        list
    }
}
struct NoContext;
impl GraphContext for NoContext {
    fn observe(&self, _: &GraphNode) -> GraphObservation {
        GraphObservation::default()
    }
}

#[test]
fn native_graph_preedit_undo_and_commit_keep_one_reversible_document_edit() {
    let mut f = Fixture::new("original");
    let original = f.doc.clone();
    f.command(EditorCommand::SelectAll);
    let target = f.state().target;
    f.input(InputEvent::CompositionFor {
        target,
        text: "日本".into(),
        cursor: Some((0, 6)),
    });
    assert_eq!(f.doc, original);
    assert!(f.graph.text_focus().unwrap().caret() <= "original".len());
    let paint = f.paint();
    assert!(
        paint
            .ops
            .iter()
            .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
    );
    assert!(
        !paint
            .ops
            .iter()
            .any(|op| matches!(op, PaintOp::Image { .. }))
    );
    assert!(
        f.input(key(PhysicalKey::Enter)).action.is_none(),
        "IME owns Enter during preedit"
    );
    assert!(f.graph.text_focus().is_some());
    assert!(
        f.input(InputEvent::TextFor {
            target,
            text: "e\u{301}東京".into()
        })
        .changed
    );
    f.input(key(PhysicalKey::Backspace));
    assert_eq!(f.text(), "e\u{301}東");
    f.command(EditorCommand::Undo);
    assert_eq!(f.text(), "e\u{301}東京");
    assert!(f.graph.take_edits().is_empty());
    assert_eq!(
        f.input(key(PhysicalKey::Enter)).action,
        Some(EditorAction::Submit)
    );
    assert!(f.graph.text_focus().is_none());
    let after = f.doc.clone();
    let edits = f.graph.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut f.doc, false);
    assert_eq!(f.doc, original);
    edits[0].apply(&mut f.doc, true);
    assert_eq!(f.doc, after);
    let row = f.doc.nodes[0].row_rect(0, &f.doc.metrics());
    let point = f
        .graph
        .viewport
        .to_screen([row.origin[0] + 12., row.origin[1] + 10.]);
    f.session
        .pointer_down(
            &mut f.graph,
            &mut f.doc,
            &mut f.fonts,
            point,
            &BasicGraphControls,
        )
        .unwrap();
    assert_ne!(f.state().target, target);
    assert!(
        !f.input(InputEvent::TextFor {
            target,
            text: "stale".into()
        })
        .changed
    );
    assert_eq!(f.doc, after);
}

#[test]
fn graph_scope_suspension_rejects_pointer_ime_and_pending_clipboard() {
    let mut f = Fixture::new("A東京Z");
    f.command(EditorCommand::SelectAll);
    assert!(matches!(f.command(EditorCommand::Copy).clipboard,
        Some(EditorClipboard::Write { text, cut: None }) if text == "A東京Z"));
    let cut = f.command(EditorCommand::Cut);
    let Some(EditorClipboard::Write {
        cut: Some(request), ..
    }) = cut.clipboard
    else {
        panic!("cut request");
    };
    assert_eq!(f.text(), "A東京Z");
    assert!(
        !f.input(InputEvent::Clipboard {
            request,
            result: ClipboardResult::Written(false)
        })
        .changed
    );
    let cut = f.command(EditorCommand::Cut);
    let Some(EditorClipboard::Write {
        cut: Some(request), ..
    }) = cut.clipboard
    else {
        panic!("cut request");
    };
    assert!(
        f.input(InputEvent::Clipboard {
            request,
            result: ClipboardResult::Written(true)
        })
        .changed
    );
    assert_eq!(f.text(), "");
    f.command(EditorCommand::Undo);
    let target = f.state().target;
    let Some(EditorClipboard::Read { request }) = f.command(EditorCommand::Paste).clipboard else {
        panic!("paste request");
    };
    let before = f.doc.clone();
    let bounds = f.session.bounds().unwrap();
    let event = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: [bounds.x + 5., bounds.y + 5.],
    };
    assert!(
        !f.session
            .input(&mut f.graph, &mut f.doc, &mut f.fonts, &event, false)
            .input
            .consumed
    );
    assert!(
        f.session
            .input_state(&mut f.graph, &f.doc, &mut f.fonts, false)
            .is_none()
    );
    let resumed = f.state().target;
    assert_ne!(resumed, target);
    assert!(
        !f.input(InputEvent::TextFor {
            target,
            text: "stale".into()
        })
        .changed
    );
    assert!(
        !f.input(InputEvent::Clipboard {
            request,
            result: ClipboardResult::Text(Some("stale".into()))
        })
        .changed
    );
    assert_eq!(f.doc, before);
    assert!(
        f.session
            .command(
                &mut f.graph,
                &mut f.doc,
                &mut f.fonts,
                EditorCommand::Cut,
                false
            )
            .clipboard
            .is_none()
    );
    assert_eq!(f.doc, before);
}

#[test]
fn graph_cancel_reverts_the_draft_and_external_model_replacement_rebases_it() {
    let mut f = Fixture::new("first");
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Text("draft".into()));
    let stale = f.state().target;
    f.doc.nodes[0].rows[0].value = GraphControlValue::Text("external replacement".into());
    assert_ne!(f.state().target, stale);
    assert!(
        !f.input(InputEvent::CompositionFor {
            target: stale,
            text: "old".into(),
            cursor: None
        })
        .input
        .consumed
    );
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Text("new draft".into()));
    f.input(InputEvent::Composition {
        text: "仮".into(),
        cursor: None,
    });
    assert!(f.input(key(PhysicalKey::Escape)).action.is_none());
    assert_eq!(f.text(), "new draft");
    let cancel = f.input(key(PhysicalKey::Escape));
    assert_eq!(cancel.action, Some(EditorAction::Cancel));
    assert!(cancel.changed);
    assert_eq!(
        f.doc.nodes[0].rows[0].value,
        GraphControlValue::Text("external replacement".into())
    );
    assert!(f.graph.take_edits().is_empty());
}

#[test]
fn graph_hit_testing_stays_on_extended_graphemes_at_each_zoom_and_reuses_its_epoch() {
    let text = "Ae\u{301}👨‍👩‍👧‍👦אבZ";
    let mut f = Fixture::new(text);
    let mut boundaries: Vec<_> = text.grapheme_indices(true).map(|(at, _)| at).collect();
    boundaries.push(text.len());
    let target = f.state().target;
    for zoom in [0.5, 1., 2.25] {
        f.graph
            .viewport
            .zoom_at([0., 0.], zoom / f.graph.viewport.zoom());
        f.paint();
        let bounds = f.session.bounds().unwrap();
        use theframework::text_edit::{
            NativeTextAnswer as A, NativeTextQuery as Q, NativeTextRequest,
        };
        let source = f
            .session
            .native_text_snapshot(&mut f.graph, &f.doc, &mut f.fonts, true)
            .unwrap();
        let query = |query| NativeTextRequest {
            target: source.target,
            revision: source.revision,
            query,
        };
        let area = f.state().area;
        let A::Geometry { caret, .. } = f
            .session
            .native_text_query(
                &mut f.graph,
                &f.doc,
                &mut f.fonts,
                true,
                &query(Q::Geometry(source.selection.1..source.selection.1)),
            )
            .unwrap()
        else {
            panic!("geometry")
        };
        // Candidate metadata is clipped to the visible cell; the query carries
        // the full painted caret, whose fallback-font metrics may exceed it.
        let visible = caret.intersection(bounds);
        assert!(
            visible.x >= area.x
                && visible.x <= area.x + area.width
                && (visible.y - area.y).abs() < 0.001
        );

        for index in 0..80 {
            let point = [
                bounds.x + (index as f32 + 0.5) * bounds.width / 80.,
                bounds.y + bounds.height * 0.5,
            ];
            let hit = f
                .session
                .native_text_query(
                    &mut f.graph,
                    &f.doc,
                    &mut f.fonts,
                    true,
                    &query(Q::Hit(point)),
                )
                .unwrap();
            f.session
                .pointer_down(
                    &mut f.graph,
                    &mut f.doc,
                    &mut f.fonts,
                    point,
                    &BasicGraphControls,
                )
                .unwrap();
            f.input(InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: point,
            });
            let focus = f.graph.text_focus().unwrap();
            assert_eq!(hit, A::Hit(focus.caret()));
            assert!(
                boundaries.contains(&focus.caret()),
                "zoom {zoom}, caret {}",
                focus.caret()
            );
            assert_eq!(f.state().target, target);
            assert!(f.graph.take_edits().is_empty());
        }
    }
    assert_eq!(f.text(), text);
}

#[test]
fn inline_graph_preserves_authored_multiline_text_and_history_across_layout_changes() {
    let original = "first\r\nsecond\t\u{c}e\u{301}";
    let mut f = Fixture::new(original);
    let source = f.doc.clone();
    f.paint();
    assert_eq!(f.doc, source);
    let target = f.state().target;
    f.command(EditorCommand::SelectAll);
    let Some(EditorClipboard::Read { request }) = f.command(EditorCommand::Paste).clipboard else {
        panic!("paste");
    };
    f.input(InputEvent::Clipboard {
        request,
        result: ClipboardResult::Text(Some("replacement\n東京\t".into())),
    });
    f.graph.viewport.pan = [91., 53.];
    f.graph.viewport.zoom_at([0., 0.], 1.5);
    f.fonts.register_font_named(
        "graph-revision",
        include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec(),
    );
    f.paint();
    assert_eq!(f.state().target, target);
    f.command(EditorCommand::Undo);
    assert_eq!(f.text(), original);
    f.command(EditorCommand::Redo);
    assert_eq!(f.text(), "replacement東京");
    let submit = f.input(key(PhysicalKey::Enter));
    assert_eq!(submit.action, Some(EditorAction::Submit));
    let edits = f.graph.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut f.doc, false);
    assert_eq!(f.doc, source);
}

#[test]
fn list_text_cells_share_native_editing_and_the_whole_row_transaction() {
    let mut f = Fixture::new("unused");
    f.graph.finish_text(&mut f.doc, false);
    f.doc.nodes[0].rows[0].value = GraphControlValue::List {
        columns: vec![
            GraphListColumn {
                id: "text".into(),
                label: "Text".into(),
                control: GraphControlValue::Text(String::new()),
            },
            GraphListColumn {
                id: "enabled".into(),
                label: "Enabled".into(),
                control: GraphControlValue::Toggle(false),
            },
        ],
        row_ids: vec![uuid::Uuid::new_v4()],
        rows: vec![vec![
            GraphControlValue::Text("cell".into()),
            GraphControlValue::Toggle(true),
        ]],
    };
    let before = f.doc.clone();
    let rect = f.doc.nodes[0].row_rect(0, &f.doc.metrics());
    let point = f.graph.viewport.to_screen([
        rect.origin[0] + 12.,
        rect.origin[1] + f.doc.metrics().list_header + 10.,
    ]);
    f.session
        .pointer_down(
            &mut f.graph,
            &mut f.doc,
            &mut f.fonts,
            point,
            &BasicGraphControls,
        )
        .unwrap();
    assert_eq!(f.graph.text_focus().unwrap().cell, Some((0, 0)));
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Text("e\u{301}日本".into()));
    f.paint();
    assert_eq!(f.text(), "e\u{301}日本");
    f.input(key(PhysicalKey::Enter));
    let after = f.doc.clone();
    let edits = f.graph.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut f.doc, false);
    assert_eq!(f.doc, before);
    edits[0].apply(&mut f.doc, true);
    assert_eq!(f.doc, after);
}

#[test]
fn graph_label_fitting_uses_display_measurement_and_keeps_whole_graphemes() {
    struct Display {
        fonts: TextSystem,
        drawn: Vec<(GraphRect, String, f32)>,
    }
    impl GraphPainter for Display {
        fn round_rect(&mut self, _: GraphRect, _: f32, _: GraphColor) {}
        fn text_width(&mut self, text: &str, size: f32) -> f32 {
            self.fonts.layout(text, "Roboto", size, None).width()
        }
        fn text(&mut self, rect: GraphRect, text: &str, size: f32, _: GraphColor) {
            self.drawn.push((rect, text.into(), size));
        }
        fn curve(&mut self, _: [Point; 4], _: f32, _: GraphColor) {}
        fn preview(&mut self, _: GraphRect, _: &str) {}
    }
    struct Controls;
    impl GraphControls for Controls {
        fn text_width(&self, _: &str, _: f32) -> f32 {
            panic!("display fitting must use its own painter");
        }
        fn label(&self, value: &GraphControlValue) -> String {
            BasicGraphControls.label(value)
        }
        fn interact(
            &self,
            value: &GraphControlValue,
            input: GraphControlInput,
        ) -> Option<GraphControlValue> {
            BasicGraphControls.interact(value, input)
        }
    }
    let long = "e\u{301}👨‍👩‍👧‍👦日本 ".repeat(30);
    let mut f = Fixture::new(&long);
    f.graph.finish_text(&mut f.doc, false);
    f.doc.nodes[0].title = long.clone();
    f.doc.nodes[0].rows[0].label = long.clone();
    let mut painter = Display {
        fonts: f.fonts,
        drawn: Vec::new(),
    };
    f.graph.paint(
        &f.doc,
        &NoContext,
        &Controls,
        &mut painter,
        [640., 400.],
        &GraphTheme::default(),
    );
    assert!(
        painter
            .drawn
            .iter()
            .filter(|(_, text, _)| text.ends_with('…'))
            .count()
            >= 3
    );
    let ends: Vec<_> = long
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain([long.len()])
        .collect();
    let drawn = std::mem::take(&mut painter.drawn);
    for (rect, text, size) in drawn {
        if let Some(prefix) = text.strip_suffix('…') {
            assert!(long.starts_with(prefix));
            assert!(ends.contains(&prefix.len()));
            assert!(painter.text_width(&text, size) <= rect.size[0]);
        }
    }
    let rect = GraphRect {
        origin: [0.; 2],
        size: [1.; 2],
    };
    painter.fitted_text(rect, &long, 0., 14., [255; 4]);
    assert_eq!(painter.drawn.last().unwrap().1, "");
    painter.fitted_text(rect, &long, 1., 14., [255; 4]);
    assert_eq!(painter.drawn.last().unwrap().1, "");
}

#[test]
fn graph_hidden_scope_and_modifier_selection_preserve_committed_values() {
    let mut f = Fixture::new("abcdef");
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Modifiers(Modifiers {
        shift: true,
        ..Default::default()
    }));
    f.input(key(PhysicalKey::Left));
    assert_eq!(f.graph.text_focus().unwrap().selection(), 0..5);
    let previous = f.state().target;
    let before = f.doc.clone();
    f.doc.nodes[0].folded = true;
    assert!(
        f.session
            .input_state(&mut f.graph, &f.doc, &mut f.fonts, true)
            .is_none()
    );
    f.doc.nodes[0].folded = false;
    assert_ne!(f.state().target, previous);
    assert!(
        !f.input(InputEvent::TextFor {
            target: previous,
            text: "stale".into()
        })
        .changed
    );
    assert_eq!(f.doc, before);
    assert!(f.graph.take_edits().is_empty());
}

#[cfg(feature = "render-wgpu")]
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_graph_draft_and_paint_survive_gpu_replacement_without_panel_pixels() {
    use renderkit::*;
    use std::sync::Arc;
    use theframework::{
        paint::{ImageHandle, Rect},
        prelude::*,
        render_wgpu::{PaintRenderer, UiImages},
        theui::paint::WidgetPainter,
    };
    struct Preview(ImageHandle);
    impl GraphImageHandles for Preview {
        fn image_handle(&self, key: &str) -> Option<ImageHandle> {
            (key == "preview").then_some(self.0)
        }
    }
    let mut f = Fixture::new("Native · e\u{301} · العربية");
    f.doc.nodes[0].title = "Message".into();
    f.doc.nodes[0].rows.push(GraphRow::new(
        "Shared image",
        GraphControlValue::Preview {
            asset: "preview".into(),
            caption: "GPU image".into(),
        },
    ));
    f.doc.nodes[0].ports.push(GraphPort::new(
        "Next",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    let mut other = GraphNode::new("Action", [326., 110.], [75, 115, 80, 255]);
    other
        .rows
        .push(GraphRow::new("Enabled", GraphControlValue::Toggle(true)));
    other.ports.push(GraphPort::new(
        "Input",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    f.doc.connections.push(GraphConnection {
        id: uuid::Uuid::new_v4(),
        from: f.doc.nodes[0].ports[0].id,
        to: other.ports[0].id,
    });
    f.doc.nodes.push(other);
    f.graph.viewport.pan = [18., 12.];
    f.input(key(PhysicalKey::End));
    let target = f.state().target;
    f.input(InputEvent::CompositionFor {
        target,
        text: "日本".into(),
        cursor: Some((0, 6)),
    });
    let before = f.doc.clone();
    let mut captures = Vec::new();
    for _ in 0..2 {
        let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let asset = ImageAsset::rgba(2, 1, vec![32, 80, 230, 255, 230, 150, 32, 255]).unwrap();
        let preview = Preview(images.registry.upload_linear(&asset).unwrap());
        let mut local = PaintList::default();
        f.session.paint(
            &mut f.graph,
            &f.doc,
            &NoContext,
            &BasicGraphControls,
            PaintGraphPainter {
                paint: &mut local,
                text: &mut f.fonts,
                images: &preview,
                family: "Roboto",
            },
            [590., 350.],
            &GraphTheme::default(),
            true,
        );
        assert!(
            local
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::Image { .. }))
        );
        let mut context = TheContext::new(640, 400, 1.5);
        let mut view = TheRenderView::new(TheId::named("graph-view"));
        view.set_dim(TheDim::new(25, 20, 590, 350), &mut context);
        view.set_paint_list(Some(Arc::new(local))).unwrap();
        let mut list = PaintList::default();
        let mut style = TheClassicStyle::new();
        WidgetPainter {
            list: &mut list,
            text: &mut context.text,
            images: &mut images,
            ui: &context.ui,
            theme: style.theme().as_mut(),
        }
        .widget(&mut view)
        .unwrap();
        images.finish_frame(&list).unwrap();
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 600)).unwrap();
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (960, 600),
                1.5,
                [0.015, 0.02, 0.025, 1.],
            )
            .unwrap();
        let uploads = gpu.transfers().uploaded_bytes;
        let rasterized = renderer.glyphs.rasterizations;
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (960, 600),
                1.5,
                [0.015, 0.02, 0.025, 1.],
            )
            .unwrap();
        assert_eq!(gpu.transfers().uploaded_bytes, uploads);
        assert_eq!(renderer.glyphs.rasterizations, rasterized);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        assert_eq!(f.doc, before);
        assert_eq!(f.state().target, target);
        let area = f.state().area;
        assert!(Rect::new(0., 0., 590., 350.).contains([area.x, area.y]));
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let capture = ticket.try_finish(&gpu).unwrap().unwrap();
        captures.push(capture.pixels().to_vec());
    }
    assert_eq!(captures[0], captures[1]);
    let file = std::fs::File::create("/private/tmp/eldiron-native-graph-text.png").unwrap();
    let mut encoder = png::Encoder::new(file, 960, 600);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&captures[0])
        .unwrap();
    assert!(
        f.input(InputEvent::TextFor {
            target,
            text: "日本".into()
        })
        .changed
    );
    assert!(f.text().ends_with("日本"));
}

#[test]
fn recreating_graph_editor_with_identical_authored_ids_invalidates_the_old_native_session() {
    let mut f = Fixture::new("same authored text");
    let old = f.state().target;
    let before = f.doc.clone();
    // The application installs a new document/controller without an intervening
    // paint. Node/row IDs and text can be identical in two project snapshots.
    f.graph = GraphEditor::default();
    let rect = f.doc.nodes[0].row_rect(0, &f.doc.metrics());
    let point = f
        .graph
        .viewport
        .to_screen([rect.origin[0] + 12., rect.origin[1] + 10.]);
    f.graph.pointer_down(&mut f.doc, point, &BasicGraphControls);
    assert_ne!(f.state().target, old);
    assert!(
        !f.input(InputEvent::TextFor {
            target: old,
            text: "stale".into()
        })
        .changed
    );
    assert_eq!(f.doc, before);
}

fn cell_fixture(identified: bool) -> Fixture {
    let mut f = Fixture::new("unused");
    f.graph.finish_text(&mut f.doc, false);
    f.doc.nodes[0].rows[0].value = GraphControlValue::List {
        columns: ["first", "second"]
            .into_iter()
            .map(|id| GraphListColumn {
                id: id.into(),
                label: id.into(),
                control: GraphControlValue::Text(String::new()),
            })
            .collect(),
        row_ids: if identified {
            vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4()]
        } else {
            Vec::new()
        },
        rows: [["OneA", "OneB"], ["TwoA", "TwoB"]]
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|value| GraphControlValue::Text(value.into()))
                    .collect()
            })
            .collect(),
    };
    let rect = f.doc.nodes[0].row_rect(0, &f.doc.metrics());
    let point = f.graph.viewport.to_screen([
        rect.origin[0] + 12.,
        rect.origin[1] + f.doc.metrics().list_header + f.doc.metrics().list_row + 10.,
    ]);
    f.session
        .pointer_down(
            &mut f.graph,
            &mut f.doc,
            &mut f.fonts,
            point,
            &BasicGraphControls,
        )
        .unwrap();
    f.session.input(
        &mut f.graph,
        &mut f.doc,
        &mut f.fonts,
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: point,
        },
        true,
    );
    assert_eq!(f.graph.text_focus().unwrap().cell, Some((1, 0)));
    f
}

#[test]
fn identified_graph_cells_keep_ime_history_and_transaction_targets_across_row_and_column_reordering()
 {
    let mut f = cell_fixture(true);
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Text("draft".into()));
    let target = f.state().target;
    let old_area = f.state().area;
    f.input(InputEvent::CompositionFor {
        target,
        text: "日本".into(),
        cursor: None,
    });
    let GraphControlValue::List {
        columns,
        rows,
        row_ids,
    } = &mut f.doc.nodes[0].rows[0].value
    else {
        unreachable!()
    };
    columns.swap(0, 1);
    rows.swap(0, 1);
    row_ids.swap(0, 1);
    for row in rows.iter_mut() {
        row.swap(0, 1);
    }
    rows[1][0] = GraphControlValue::Text("Unrelated current field".into());
    f.paint();
    assert_eq!(f.graph.text_focus().unwrap().cell, Some((0, 1)));
    assert_eq!(
        f.state().target,
        target,
        "layout reordering preserves the same native callback identity"
    );
    assert_ne!(f.state().area, old_area);
    assert_eq!(f.text(), "draft");
    assert!(f.graph.take_edits().is_empty());
    assert_eq!(
        f.input(key(PhysicalKey::Enter)).action,
        None,
        "preedit owns Enter after reordering"
    );
    f.input(InputEvent::TextFor {
        target,
        text: "日本".into(),
    });
    assert_eq!(f.text(), "draft日本");
    f.command(EditorCommand::Undo);
    assert_eq!(f.text(), "draft");
    f.command(EditorCommand::Redo);
    assert_eq!(f.text(), "draft日本");
    let mut before = f.doc.clone();
    let GraphControlValue::List { rows, .. } = &mut before.nodes[0].rows[0].value else {
        unreachable!()
    };
    rows[0][1] = GraphControlValue::Text("TwoA".into());
    f.input(key(PhysicalKey::Enter));
    let after = f.doc.clone();
    let edits = f.graph.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut f.doc, false);
    assert_eq!(
        f.doc, before,
        "undo restores the identified cell and preserves other current fields"
    );
    edits[0].apply(&mut f.doc, true);
    assert_eq!(f.doc, after);
}

#[test]
fn removed_or_ambiguous_graph_cells_revoke_ime_and_clipboard_without_editing_a_replacement() {
    for case in 0..8 {
        let mut f = cell_fixture(case < 5);
        f.command(EditorCommand::SelectAll);
        let target = f.state().target;
        let Some(EditorClipboard::Read { request }) = f.command(EditorCommand::Paste).clipboard
        else {
            panic!("paste request")
        };
        f.input(InputEvent::CompositionFor {
            target,
            text: "未確定".into(),
            cursor: None,
        });
        let value = &mut f.doc.nodes[0].rows[0].value;
        if case == 4 {
            *value = GraphControlValue::Text("Replacement scalar".into());
        } else {
            let GraphControlValue::List {
                rows,
                columns,
                row_ids,
            } = value
            else {
                unreachable!()
            };
            match case {
                0 => {
                    rows.remove(1);
                    row_ids.remove(1);
                }
                1 => {
                    columns.remove(0);
                    for row in rows {
                        row.remove(0);
                    }
                }
                2 => {
                    row_ids[0] = row_ids[1];
                }
                3 => {
                    columns[1].id = columns[0].id.clone();
                }
                5 => {
                    rows.swap(0, 1);
                }
                6 => {
                    rows.insert(0, vec![GraphControlValue::Text("Inserted".into()); 2]);
                }
                7 => {
                    rows[0][0] = GraphControlValue::Text("New legacy source".into());
                }
                _ => unreachable!(),
            }
        }
        let replaced = f.doc.clone();
        f.paint();
        assert!(f.graph.text_focus().is_none(), "case {case}");
        assert!(
            f.session
                .input_state(&mut f.graph, &f.doc, &mut f.fonts, true)
                .is_none()
        );
        assert!(
            !f.input(InputEvent::TextFor {
                target,
                text: "stale".into()
            })
            .changed
        );
        assert!(
            !f.input(InputEvent::CompositionFor {
                target,
                text: "stale".into(),
                cursor: None
            })
            .changed
        );
        assert!(
            !f.input(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Text(Some("stale".into()))
            })
            .changed
        );
        assert_eq!(f.doc, replaced, "case {case}");
        assert!(f.graph.take_edits().is_empty());
    }
}

#[test]
fn legacy_list_cell_edits_roundtrip_without_inventing_serialized_row_ids() {
    let mut f = cell_fixture(false);
    let before = f.doc.clone();
    let target = f.state().target;
    f.command(EditorCommand::SelectAll);
    f.input(InputEvent::Text("e\u{301}日本".into()));
    f.paint();
    assert_eq!(f.state().target, target);
    f.command(EditorCommand::Undo);
    assert_eq!(f.text(), "TwoA");
    f.command(EditorCommand::Redo);
    assert_eq!(f.text(), "e\u{301}日本");
    f.input(key(PhysicalKey::Enter));
    let GraphControlValue::List { row_ids, .. } = &f.doc.nodes[0].rows[0].value else {
        unreachable!()
    };
    assert!(row_ids.is_empty());
    let edits = f.graph.take_edits();
    assert_eq!(edits.len(), 1);
    edits[0].apply(&mut f.doc, false);
    assert_eq!(f.doc, before);
}

#[test]
fn reference_graph_text_input_and_cancel_follow_authored_cell_addresses() {
    let mut f = cell_fixture(true);
    f.graph.text_input(&mut f.doc, GraphTextInput::SelectAll);
    f.graph
        .text_input(&mut f.doc, GraphTextInput::Insert("draft".into()));
    let GraphControlValue::List { rows, row_ids, .. } = &mut f.doc.nodes[0].rows[0].value else {
        unreachable!()
    };
    rows.swap(0, 1);
    row_ids.swap(0, 1);
    rows[1][1] = GraphControlValue::Text("External field".into());
    assert!(
        f.graph
            .text_input(&mut f.doc, GraphTextInput::Insert("!".into()))
    );
    assert_eq!(f.graph.text_focus().unwrap().cell, Some((0, 0)));
    assert_eq!(f.text(), "draft!");
    f.graph.text_input(&mut f.doc, GraphTextInput::Cancel);
    let GraphControlValue::List { rows, .. } = &f.doc.nodes[0].rows[0].value else {
        unreachable!()
    };
    assert_eq!(rows[0][0], GraphControlValue::Text("TwoA".into()));
    assert_eq!(rows[1][1], GraphControlValue::Text("External field".into()));
    assert!(f.graph.take_edits().is_empty());
}

fn detached_draft(f: &mut Fixture) -> (GraphViewState, GraphTextDraft) {
    let target = f.graph.text_focus().unwrap().clone();
    f.graph.finish_text(&mut f.doc, false);
    let state = GraphViewState::new("detached-test", f.doc.clone());
    let mut draft =
        GraphTextDraft::new(state.binding(), target, state.document(), &mut f.fonts).unwrap();
    draft.view.prepare(
        &mut f.fonts,
        theframework::text::TextOptions {
            family: "Roboto",
            size: 15.,
            width: Some(220.),
            line_height: Some(24.),
        },
        theframework::paint::Rect::new(40., 60., 220., 180.),
        false,
    );
    draft.view.set_focused(&mut f.fonts, true);
    (state, draft)
}

#[test]
fn detached_graph_draft_checkpoints_multiline_source_without_ending_ime_or_local_history() {
    let original = "A\r\n東京 e\u{301}";
    let mut f = Fixture::new(original);
    let (mut state, mut draft) = detached_draft(&mut f);
    let source = state.document().clone();
    assert_eq!(draft.text(), original);
    assert!(!draft.has_changes());
    draft
        .view
        .editor
        .execute(&mut f.fonts, EditorCommand::SelectAll);
    draft
        .view
        .input(&mut f.fonts, &InputEvent::Text("next\r\n値".into()));
    let target = draft.view.text_input_state(&mut f.fonts).unwrap().target;
    draft.view.input(
        &mut f.fonts,
        &InputEvent::CompositionFor {
            target,
            text: "日本".into(),
            cursor: None,
        },
    );
    assert_eq!(draft.text(), "next\r\n値");
    let first = state.checkpoint_draft(&mut draft).unwrap().unwrap();
    assert_eq!(first.serial, 1);
    assert!(!draft.has_changes());
    assert_eq!(
        draft.view.text_input_state(&mut f.fonts).unwrap().target,
        target
    );
    assert!(draft.view.editor.is_composing());
    assert!(state.checkpoint_draft(&mut draft).unwrap().is_none());
    let mut persisted = source;
    for edit in &first.edits {
        edit.apply(&mut persisted, true);
    }
    assert_eq!(&persisted, state.document());
    assert!(state.acknowledge(first.binding, first.serial));
    assert_eq!(
        draft
            .view
            .input(&mut f.fonts, &key(PhysicalKey::Enter))
            .action,
        None
    );
    draft.view.input(
        &mut f.fonts,
        &InputEvent::TextFor {
            target,
            text: "日本".into(),
        },
    );
    assert_eq!(draft.text(), "next\r\n値日本");
    draft.view.editor.execute(&mut f.fonts, EditorCommand::Undo);
    assert_eq!(draft.text(), "next\r\n値");
    assert!(state.checkpoint_draft(&mut draft).unwrap().is_none());
    draft.view.editor.execute(&mut f.fonts, EditorCommand::Undo);
    assert_eq!(draft.text(), original);
    let second = state.checkpoint_draft(&mut draft).unwrap().unwrap();
    assert_eq!(second.serial, 2);
    for edit in &second.edits {
        edit.apply(&mut persisted, true);
    }
    assert_eq!(&persisted, state.document());
    assert!(state.acknowledge(second.binding, second.serial));
    assert!(
        state.pending_through(first.serial).is_empty(),
        "old wake-up cannot duplicate a saved draft"
    );
    draft.view.editor.execute(&mut f.fonts, EditorCommand::Redo);
    assert_eq!(draft.text(), "next\r\n値");
    assert!(state.checkpoint_draft(&mut draft).unwrap().is_some());
}

#[test]
fn detached_graph_draft_follows_identified_cells_and_preserves_other_current_fields() {
    let mut f = cell_fixture(true);
    let (mut state, mut draft) = detached_draft(&mut f);
    draft
        .view
        .editor
        .execute(&mut f.fonts, EditorCommand::SelectAll);
    draft
        .view
        .input(&mut f.fonts, &InputEvent::Text("long\n日本".into()));
    state.edit(|document, _| {
        let GraphControlValue::List {
            rows,
            columns,
            row_ids,
        } = &mut document.nodes[0].rows[0].value
        else {
            unreachable!()
        };
        rows.swap(0, 1);
        row_ids.swap(0, 1);
        columns.swap(0, 1);
        for row in rows.iter_mut() {
            row.swap(0, 1);
        }
        rows[1][0] = GraphControlValue::Text("Current external field".into());
    });
    let before = state.document().clone();
    let commit = state.checkpoint_draft(&mut draft).unwrap().unwrap();
    let GraphControlValue::List { rows, .. } = &state.document().nodes[0].rows[0].value else {
        unreachable!()
    };
    assert_eq!(rows[0][1], GraphControlValue::Text("long\n日本".into()));
    assert_eq!(
        rows[1][0],
        GraphControlValue::Text("Current external field".into())
    );
    let mut undo = state.document().clone();
    assert_eq!(commit.edits.len(), 1);
    commit.edits[0].apply(&mut undo, false);
    assert_eq!(undo, before);
}

#[test]
fn detached_graph_draft_conflicts_retain_the_draft_without_overwriting_new_source() {
    let mut f = Fixture::new("Original");
    let (mut state, mut draft) = detached_draft(&mut f);
    draft
        .view
        .editor
        .execute(&mut f.fonts, EditorCommand::SelectAll);
    draft.view.input(
        &mut f.fonts,
        &InputEvent::Text("Retained private draft".into()),
    );
    state.edit(|document, _| {
        document.nodes[0].rows[0].value = GraphControlValue::Text("External source".into());
    });
    let external = state.document().clone();
    assert_eq!(
        state.checkpoint_draft(&mut draft).unwrap_err(),
        GraphDraftError::SourceChanged
    );
    assert_eq!(state.document(), &external);
    assert_eq!(draft.text(), "Retained private draft");
    assert_eq!(state.last_serial(), 0);
    state.edit(|document, _| {
        document.nodes.clear();
    });
    assert_eq!(
        state.checkpoint_draft(&mut draft).unwrap_err(),
        GraphDraftError::MissingTarget
    );
    assert_eq!(draft.text(), "Retained private draft");
    state.replace_document("detached-test", external);
    assert_eq!(
        state.checkpoint_draft(&mut draft).unwrap_err(),
        GraphDraftError::WrongBinding
    );
    assert_eq!(state.last_serial(), 0);
}

#[test]
fn detached_graph_draft_requires_exclusive_inline_and_pointer_editing_scope() {
    let mut f = Fixture::new("Original");
    let (mut state, mut draft) = detached_draft(&mut f);
    draft
        .view
        .editor
        .execute(&mut f.fonts, EditorCommand::SelectAll);
    draft
        .view
        .input(&mut f.fonts, &InputEvent::Text("Detached".into()));
    for inline in [true, false] {
        state.edit(|document, editor| {
            let point = if inline {
                let r = document.nodes[0].row_rect(0, &document.metrics());
                [r.origin[0] + 12., r.origin[1] + 10.]
            } else {
                [
                    document.nodes[0].position[0] + 24.,
                    document.nodes[0].position[1] + 12.,
                ]
            };
            editor.pointer_down(
                document,
                editor.viewport.to_screen(point),
                &BasicGraphControls,
            );
        });
        assert_eq!(state.editor().text_focus().is_some(), inline);
        assert_eq!(state.editor().is_gesturing(), !inline);
        assert_eq!(
            state.checkpoint_draft(&mut draft).unwrap_err(),
            GraphDraftError::EditorBusy
        );
        assert_eq!(state.last_serial(), 0);
        state.finish_text(false);
        state.edit(|document, editor| editor.cancel_gesture(document));
    }
    assert!(state.checkpoint_draft(&mut draft).unwrap().is_some());
    assert_eq!(draft.text(), "Detached");
}

#[test]
fn detached_graph_checkpoint_keeps_a_preceding_inline_commit_in_its_own_serial() {
    let mut f = Fixture::new("Original");
    let target = f.graph.text_focus().unwrap().clone();
    f.graph.finish_text(&mut f.doc, false);
    let mut state = GraphViewState::new("ordered-detached-test", f.doc.clone());
    state.edit(|document, editor| {
        let r = document.nodes[0].row_rect(0, &document.metrics());
        editor.pointer_down(
            document,
            editor
                .viewport
                .to_screen([r.origin[0] + 12., r.origin[1] + 10.]),
            &BasicGraphControls,
        );
        assert!(editor.text_focus().is_some());
        editor.text_input(document, GraphTextInput::SelectAll);
        editor.text_input(document, GraphTextInput::Insert("Inline".into()));
        editor.finish_text(document, true);
    });
    let mut draft =
        GraphTextDraft::new(state.binding(), target, state.document(), &mut f.fonts).unwrap();
    draft.view.prepare(
        &mut f.fonts,
        theframework::text::TextOptions {
            family: "Roboto",
            size: 15.,
            width: Some(220.),
            line_height: Some(24.),
        },
        theframework::paint::Rect::new(0., 0., 220., 180.),
        false,
    );
    draft.view.set_focused(&mut f.fonts, true);
    draft
        .view
        .editor
        .execute(&mut f.fonts, EditorCommand::SelectAll);
    draft
        .view
        .input(&mut f.fonts, &InputEvent::Text("Detached\ntext".into()));
    let notification = state.checkpoint_draft(&mut draft).unwrap().unwrap();
    assert_eq!(notification.serial, 2);
    let commits = state.pending_through(notification.serial);
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].edits.len(), 1);
    assert_eq!(commits[1].edits.len(), 1);
    let mut authoritative = f.doc.clone();
    commits[0].edits[0].apply(&mut authoritative, true);
    assert_eq!(
        authoritative.nodes[0].rows[0].value,
        GraphControlValue::Text("Inline".into())
    );
    commits[1].edits[0].apply(&mut authoritative, true);
    assert_eq!(&authoritative, state.document());
    assert!(state.acknowledge(notification.binding, notification.serial));
    assert!(state.checkpoint_draft(&mut draft).unwrap().is_none());
}

#[test]
fn native_source_ranges_edit_the_graph_draft_without_publishing_preedit_or_reusing_old_owners() {
    let mut f = Fixture::new("A東京Z");
    let source = f
        .session
        .native_text_snapshot(&mut f.graph, &f.doc, &mut f.fonts, true)
        .unwrap();
    let response = f.input(InputEvent::NativeText {
        target: source.target,
        revision: source.revision,
        action: NativeTextAction::Compose {
            range: Some(1..7),
            text: "日本".into(),
            cursor: Some((6, 6)),
        },
    });
    assert!(!response.changed);
    assert_eq!(f.text(), "A東京Z");
    let marked = f
        .session
        .native_text_snapshot(&mut f.graph, &f.doc, &mut f.fonts, true)
        .unwrap();
    assert_eq!(&*marked.text, "A日本Z");
    assert!(
        f.input(InputEvent::NativeText {
            target: marked.target,
            revision: marked.revision,
            action: NativeTextAction::CommitComposition
        })
        .changed
    );
    assert_eq!(f.text(), "A日本Z");
    assert!(f.command(EditorCommand::Undo).changed);
    assert_eq!(f.text(), "A東京Z");
    f.session.reset();
    let replacement = f
        .session
        .native_text_snapshot(&mut f.graph, &f.doc, &mut f.fonts, true)
        .unwrap();
    assert_ne!(replacement.target, source.target);
    assert!(
        !f.input(InputEvent::NativeText {
            target: source.target,
            revision: source.revision,
            action: NativeTextAction::Replace {
                range: 0..1,
                text: "stale".into()
            }
        })
        .input
        .consumed
    );
    assert_eq!(f.text(), "A東京Z");
}
