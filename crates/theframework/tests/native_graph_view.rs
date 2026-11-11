#![cfg(all(feature = "ui", feature = "text"))]
use std::sync::{Arc, Mutex, mpsc::Receiver};
use theframework::{
    paint::{ImageHandle, PaintList},
    platform::*,
    prelude::*,
    text_edit::{EditorClipboard, EditorCommand},
    thegraph::*,
    theui::paint::{UiImageResources, UiPaintError},
};
struct NoImages;
impl UiImageResources for NoImages {
    fn image(&mut self, _: &str, _: &TheRGBABuffer, _: bool) -> Result<ImageHandle, UiPaintError> {
        panic!("native graph chrome/text must not allocate a CPU panel image")
    }
}
fn key(physical: PhysicalKey) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    }
}
struct Fixture {
    ui: TheUI,
    ctx: TheContext,
    id: TheId,
    model: Arc<Mutex<GraphViewState>>,
    events: Receiver<TheEvent>,
}
impl Fixture {
    fn new() -> Self {
        let mut node = GraphNode::new("Native graph", [24., 24.], [50, 80, 100, 255]);
        node.rows.push(GraphRow::new(
            "Message",
            GraphControlValue::Text("A東京Z".into()),
        ));
        node.rows.push(GraphRow::new(
            "Speed",
            GraphControlValue::Number {
                value: 2.,
                min: 0.,
                max: 10.,
                step: 1.,
            },
        ));
        let model = Arc::new(Mutex::new(GraphViewState::new(
            "test-document",
            GraphDocument {
                nodes: vec![node],
                ..Default::default()
            },
        )));
        let mut ctx = TheContext::new(640, 400, 1.5);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        let events = ui.add_state_listener("graph-app".into());
        let id = TheId::named("graph");
        let view = TheGraphView::with_state(id.clone(), model.clone());
        let mut side = TheCanvas::new();
        let mut spacer = TheSpacer::new(TheId::named("sidebar"));
        spacer.limiter_mut().set_max_width(80);
        side.set_widget(spacer);
        let mut center = TheCanvas::new();
        center.set_widget(view);
        ui.canvas.set_left(side);
        ui.canvas.set_center(center);
        let mut f = Self {
            ui,
            ctx,
            id,
            model,
            events,
        };
        f.paint();
        f.events.try_iter().for_each(drop);
        f
    }
    fn paint(&mut self) -> PaintList {
        self.ui
            .paint_commands(&mut self.ctx, &mut NoImages)
            .unwrap()
    }
    fn send(&mut self, event: InputEvent) -> InputResponse {
        self.ui.prepare_paint_layout(&mut self.ctx);
        self.ui.dispatch_input(&event, &mut self.ctx)
    }
    fn click_row(&mut self, index: usize, fraction: f32) {
        let dim = *self.ui.get_widget("graph").unwrap().dim();
        assert!(dim.x > 0, "exercise a translated graph viewport");
        let model = self.model.lock().unwrap();
        let rect = model.document().nodes[0].row_rect(index, &model.document().metrics());
        let local = model.editor().viewport.to_screen([
            rect.origin[0] + fraction * rect.size[0],
            rect.origin[1] + 10.,
        ]);
        drop(model);
        let position = [dim.x as f32 + local[0], dim.y as f32 + local[1]];
        self.send(InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        });
        self.send(InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position,
        });
    }
    fn command(&mut self, command: EditorCommand) {
        self.ui
            .get_widget("graph")
            .unwrap()
            .editor_command(command, &mut self.ctx);
        self.ui.process_events(&mut self.ctx);
    }
    fn state(&mut self) -> TextInputState {
        self.ui
            .get_widget("graph")
            .unwrap()
            .editor_input_state(&mut self.ctx)
            .unwrap()
    }
    fn text(&self) -> String {
        let model = self.model.lock().unwrap();
        let GraphControlValue::Text(text) = &model.document().nodes[0].rows[0].value else {
            panic!("text");
        };
        text.clone()
    }
    fn commits(&mut self) -> Vec<(u64, Vec<GraphEdit>)> {
        self.events
            .try_iter()
            .filter_map(|event| match event {
                TheEvent::GraphCommitted(id, commit) if id == self.id => {
                    Some((commit.binding, commit.edits.clone()))
                }
                _ => None,
            })
            .collect()
    }
}

#[test]
fn graph_widget_routes_shaped_text_ime_clipboard_and_exactly_one_commit_through_the_ui() {
    let mut f = Fixture::new();
    let before = f.model.lock().unwrap().document().clone();
    f.click_row(0, 0.1);
    assert_eq!(f.ctx.ui.focus, Some(f.id.clone()));
    let state = f.state();
    let dim = *f.ui.get_widget("graph").unwrap().dim();
    assert!(state.area.x >= dim.x as f32 && state.area.y >= dim.y as f32);
    assert!(state.area.x < (dim.x + dim.width) as f32);
    f.command(EditorCommand::SelectAll);
    assert!(
        f.send(InputEvent::CompositionFor {
            target: state.target,
            text: "日本".into(),
            cursor: Some((0, 6))
        })
        .consumed
    );
    assert_eq!(f.text(), "A東京Z");
    f.paint();
    assert!(
        f.send(InputEvent::TextFor {
            target: state.target,
            text: "e\u{301}日本".into()
        })
        .consumed
    );
    assert_eq!(f.text(), "e\u{301}日本");
    f.ui.undo(&mut f.ctx);
    f.ui.process_events(&mut f.ctx);
    assert_eq!(f.text(), "A東京Z");
    f.ui.redo(&mut f.ctx);
    f.ui.process_events(&mut f.ctx);
    assert_eq!(f.text(), "e\u{301}日本");
    f.command(EditorCommand::SelectAll);
    f.ui.copy(&mut f.ctx);
    assert!(
        matches!(f.ctx.ui.editor_clipboard.pop_front(), Some(EditorClipboard::Write { text, cut: None }) if text == "e\u{301}日本")
    );
    assert!(
        f.commits().is_empty(),
        "local draft undo/copy must not produce a document transaction"
    );
    f.send(key(PhysicalKey::Enter));
    let commits = f.commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, f.model.lock().unwrap().binding());
    assert_eq!(commits[0].1.len(), 1);
    let mut authoritative = before.clone();
    commits[0].1[0].apply(&mut authoritative, true);
    assert_eq!(&authoritative, f.model.lock().unwrap().document());
    commits[0].1[0].apply(&mut authoritative, false);
    assert_eq!(authoritative, before);
    assert!(f.ui.canvas.buffer.pixels().is_empty());
}

#[test]
fn native_graph_scope_blocks_menu_clipboard_and_old_ime_after_document_replacement() {
    let mut f = Fixture::new();
    f.click_row(0, 0.1);
    f.command(EditorCommand::SelectAll);
    let target = f.state().target;
    f.ui.paste(&mut f.ctx);
    let Some(EditorClipboard::Read { request }) = f.ctx.ui.editor_clipboard.pop_front() else {
        panic!("paste");
    };
    f.send(InputEvent::CompositionFor {
        target,
        text: "仮".into(),
        cursor: None,
    });
    f.ctx.ui.overlay = Some(TheId::named("other-scope"));
    f.ui.prepare_paint_layout(&mut f.ctx);
    f.ui.copy(&mut f.ctx);
    f.ui.cut(&mut f.ctx);
    f.ui.paste(&mut f.ctx);
    assert!(f.ctx.ui.editor_clipboard.is_empty());
    f.ctx.ui.overlay = None;
    f.ui.prepare_paint_layout(&mut f.ctx);
    assert_ne!(f.state().target, target);
    f.send(InputEvent::Clipboard {
        request,
        result: ClipboardResult::Text(Some("stale".into())),
    });
    assert_eq!(f.text(), "A東京Z");
    let old_binding = f.model.lock().unwrap().binding();
    let doc = f.model.lock().unwrap().document().clone();
    f.model
        .lock()
        .unwrap()
        .replace_document("new-document", doc);
    assert_ne!(f.model.lock().unwrap().binding(), old_binding);
    assert!(f.ui.needs_paint(&f.ctx));
    f.paint();
    f.click_row(0, 0.1);
    assert_ne!(f.state().target, target);
    f.send(InputEvent::TextFor {
        target,
        text: "old callback".into(),
    });
    assert_eq!(f.text(), "A東京Z");
    assert!(f.commits().is_empty());
}

#[test]
fn native_graph_edit_signals_separate_a_committed_text_value_from_the_next_unfinished_slider() {
    let mut f = Fixture::new();
    let before = f.model.lock().unwrap().document().clone();
    f.click_row(0, 0.1);
    f.command(EditorCommand::SelectAll);
    f.send(InputEvent::Text("committed first".into()));
    let dim = *f.ui.get_widget("graph").unwrap().dim();
    let model = f.model.lock().unwrap();
    let rect = model.document().nodes[0].row_rect(1, &model.document().metrics());
    let local = model
        .editor()
        .viewport
        .to_screen([rect.origin[0] + rect.size[0] * 0.8, rect.origin[1] + 10.]);
    drop(model);
    let position = [dim.x as f32 + local[0], dim.y as f32 + local[1]];
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position,
    });
    let commits = f.commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].1.len(), 1);
    let mut first = before.clone();
    commits[0].1[0].apply(&mut first, true);
    assert_eq!(
        first.nodes[0].rows[1].value, before.nodes[0].rows[1].value,
        "the first transaction must not absorb the later slider's live draft"
    );
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position,
    });
    let second = f.commits();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].1.len(), 1);
    second[0].1[0].apply(&mut first, true);
    assert_eq!(&first, f.model.lock().unwrap().document());
}

#[test]
fn native_graph_external_camera_changes_invalidate_paint_without_a_root_bitmap_or_idle_loop() {
    let mut f = Fixture::new();
    f.paint();
    f.ui.process_events(&mut f.ctx);
    f.paint();
    assert!(!f.ui.needs_paint(&f.ctx));
    f.model
        .lock()
        .unwrap()
        .edit(|_, editor| editor.viewport.pan = [75., 52.]);
    assert!(f.ui.needs_paint(&f.ctx));
    f.paint();
    f.ui.process_events(&mut f.ctx);
    f.paint();
    assert!(!f.ui.needs_paint(&f.ctx));
    f.send(InputEvent::Scroll {
        delta: [0., 2.25],
        precise: true,
    });
    // Scroll routing needs an actual hover target; a subsequent pointer/scroll
    // must update only the viewport, never document content.
    let dim = *f.ui.get_widget("graph").unwrap().dim();
    f.send(InputEvent::PointerMoved {
        position: [dim.x as f32 + 8., dim.y as f32 + 8.],
    });
    let before = f.model.lock().unwrap().document().clone();
    let pan = f.model.lock().unwrap().editor().viewport.pan;
    f.send(InputEvent::Scroll {
        delta: [0., 2.25],
        precise: true,
    });
    assert_ne!(f.model.lock().unwrap().editor().viewport.pan, pan);
    assert_eq!(f.model.lock().unwrap().document(), &before);
    assert!(f.commits().is_empty());
    assert!(f.ui.canvas.buffer.pixels().is_empty());
}

#[test]
fn suspended_graph_capture_restores_uncommitted_controls_and_keeps_graph_commands_host_owned() {
    let mut f = Fixture::new();
    let before = f.model.lock().unwrap().document().clone();
    let dim = *f.ui.get_widget("graph").unwrap().dim();
    let model = f.model.lock().unwrap();
    let rect = model.document().nodes[0].row_rect(1, &model.document().metrics());
    let local = model
        .editor()
        .viewport
        .to_screen([rect.origin[0] + rect.size[0] * 0.9, rect.origin[1] + 10.]);
    drop(model);
    let position = [dim.x as f32 + local[0], dim.y as f32 + local[1]];
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position,
    });
    assert_ne!(f.model.lock().unwrap().document(), &before);
    // Canvas focus remains an input scope even without a text field. A normal
    // event must not blur/cancel the slider; a modal scope must cancel it.
    f.send(InputEvent::PointerMoved {
        position: [position[0] - 10., position[1]],
    });
    assert_ne!(f.model.lock().unwrap().document(), &before);
    f.ctx.ui.overlay = Some(TheId::named("menu-scope"));
    f.ui.prepare_paint_layout(&mut f.ctx);
    assert_eq!(f.model.lock().unwrap().document(), &before);
    f.ctx.ui.overlay = None;
    f.ui.prepare_paint_layout(&mut f.ctx);
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position,
    });
    assert!(f.commits().is_empty());
    f.ui.copy(&mut f.ctx);
    f.ui.cut(&mut f.ctx);
    f.ui.paste(&mut f.ctx);
    f.ui.process_events(&mut f.ctx);
    let commands: Vec<_> = f
        .events
        .try_iter()
        .filter_map(|event| match event {
            TheEvent::GraphCommand(id, binding, command) if id == f.id => {
                assert_eq!(binding, f.model.lock().unwrap().binding());
                Some(command)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        commands,
        vec![
            EditorCommand::Copy,
            EditorCommand::Cut,
            EditorCommand::Paste
        ]
    );
    assert!(
        f.ctx.ui.editor_clipboard.is_empty(),
        "graph commands never read the OS text clipboard"
    );
    assert_eq!(f.model.lock().unwrap().document(), &before);
}

#[cfg(feature = "render-wgpu")]
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn graph_widget_keeps_the_ui_editor_and_commands_across_gpu_replacement() {
    use renderkit::*;
    use theframework::render_wgpu::{PaintRenderer, UiImages};
    let mut f = Fixture::new();
    f.click_row(0, 0.1);
    f.command(EditorCommand::SelectAll);
    let target = f.state().target;
    f.send(InputEvent::CompositionFor {
        target,
        text: "日本".into(),
        cursor: Some((0, 6)),
    });
    let before = f.model.lock().unwrap().document().clone();
    let mut captures = Vec::new();
    for _ in 0..2 {
        let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 600)).unwrap();
        let paint = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        images.finish_frame(&paint).unwrap();
        assert_eq!(
            paint.image_handles().count(),
            0,
            "no rasterized graph panel/source image"
        );
        renderer
            .render(
                &paint,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (960, 600),
                1.5,
                [0.02, 0.02, 0.02, 1.],
            )
            .unwrap();
        let uploaded = gpu.transfers().uploaded_bytes;
        let rasterized = renderer.glyphs.rasterizations;
        assert!(rasterized > 0);
        let repeated = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        images.finish_frame(&repeated).unwrap();
        renderer
            .render(
                &repeated,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (960, 600),
                1.5,
                [0.02, 0.02, 0.02, 1.],
            )
            .unwrap();
        assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
        assert_eq!(renderer.glyphs.rasterizations, rasterized);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        assert_eq!(f.state().target, target);
        assert_eq!(f.model.lock().unwrap().document(), &before);
        assert!(f.ui.canvas.buffer.pixels().is_empty());
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        captures.push(ticket.try_finish(&gpu).unwrap().unwrap().pixels().to_vec());
    }
    assert_eq!(captures[0], captures[1]);
    let file = std::fs::File::create("/private/tmp/eldiron-native-graph-widget.png").unwrap();
    let mut encoder = png::Encoder::new(file, 960, 600);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&captures[0])
        .unwrap();
    f.send(InputEvent::TextFor {
        target,
        text: "日本".into(),
    });
    f.send(key(PhysicalKey::Enter));
    assert_eq!(f.text(), "日本");
    let commits = f.commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].1.len(), 1);
}

#[test]
fn graph_save_checkpoint_collects_a_serial_prefix_without_losing_ime_or_replaying_wakeups() {
    let mut f = Fixture::new();
    let original = f.model.lock().unwrap().document().clone();
    f.click_row(0, 0.1);
    f.command(EditorCommand::SelectAll);
    f.send(InputEvent::Text("first commit".into()));
    f.send(key(PhysicalKey::Enter));
    // Leave its UI wake-up queued, then continue editing before an owner save.
    f.click_row(0, 0.1);
    f.command(EditorCommand::SelectAll);
    f.send(InputEvent::Text("saved draft".into()));
    let target = f.state().target;
    f.send(InputEvent::CompositionFor {
        target,
        text: "仮".into(),
        cursor: None,
    });
    let checkpoint = f.model.lock().unwrap().checkpoint_text().unwrap();
    assert_eq!(checkpoint.serial, 2);
    assert_eq!(f.state().target, target);
    assert_eq!(
        f.text(),
        "saved draft",
        "preedit is never checkpointed as committed source"
    );
    let mut authoritative = original.clone();
    let commits: Vec<_> = f.model.lock().unwrap().pending_commits().cloned().collect();
    assert_eq!(
        commits
            .iter()
            .map(|commit| commit.serial)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    for commit in &commits {
        for edit in &commit.edits {
            edit.apply(&mut authoritative, true);
        }
    }
    assert_eq!(&authoritative, f.model.lock().unwrap().document());
    assert!(!f.model.lock().unwrap().acknowledge(checkpoint.binding, 3));
    assert_eq!(f.model.lock().unwrap().pending_commits().count(), 2);
    assert!(f.model.lock().unwrap().acknowledge(checkpoint.binding, 1));
    assert_eq!(
        f.model
            .lock()
            .unwrap()
            .pending_commits()
            .map(|commit| commit.serial)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(
        f.model
            .lock()
            .unwrap()
            .acknowledge(checkpoint.binding, checkpoint.serial)
    );
    assert!(
        f.model
            .lock()
            .unwrap()
            .acknowledge(checkpoint.binding, checkpoint.serial)
    );
    assert_eq!(f.model.lock().unwrap().pending_commits().count(), 0);
    // The old wake-up was not lost, but its serial is already acknowledged;
    // an owner collecting pending commits finds nothing to apply twice.
    let wakeups: Vec<_> = f
        .events
        .try_iter()
        .filter_map(|event| match event {
            TheEvent::GraphCommitted(_, commit) => Some(commit),
            _ => None,
        })
        .collect();
    assert_eq!(wakeups.len(), 1);
    assert_eq!(wakeups[0].serial, 1);
    assert!(f.model.lock().unwrap().pending_commits().next().is_none());
    assert!(f.model.lock().unwrap().checkpoint_text().is_none());
    // The same in-progress native composition remains addressable after Save.
    f.send(key(PhysicalKey::Enter));
    assert!(
        f.model.lock().unwrap().editor().text_focus().is_some(),
        "preedit still owns Enter after checkpoint"
    );
    assert_eq!(f.model.lock().unwrap().last_serial(), 2);
    f.send(InputEvent::TextFor {
        target,
        text: "日本".into(),
    });
    assert!(f.text().contains("日本"));
    f.command(EditorCommand::Undo);
    assert_eq!(f.text(), "saved draft");
    f.command(EditorCommand::Undo);
    assert_eq!(
        f.text(),
        "first commit",
        "checkpoint retains local text undo history"
    );
    let undo_checkpoint = f.model.lock().unwrap().checkpoint_text().unwrap();
    assert_eq!(undo_checkpoint.serial, 3);
    let mut undo_source = authoritative;
    for edit in &undo_checkpoint.edits {
        edit.apply(&mut undo_source, true);
    }
    assert_eq!(&undo_source, f.model.lock().unwrap().document());
    for edit in undo_checkpoint.edits.iter().rev() {
        edit.apply(&mut undo_source, false);
    }
    assert_eq!(
        &undo_source.nodes[0].rows[0].value,
        &GraphControlValue::Text("saved draft".into())
    );
    let old_binding = checkpoint.binding;
    f.model
        .lock()
        .unwrap()
        .replace_document("replacement", original);
    assert!(!f.model.lock().unwrap().acknowledge(old_binding, 3));
    assert_eq!(f.model.lock().unwrap().last_serial(), 0);
    assert!(f.model.lock().unwrap().pending_commits().next().is_none());
}

#[test]
fn native_graph_folds_publish_once_and_survive_owner_acknowledgement_and_reversible_storage() {
    let mut f = Fixture::new();
    let source = f.model.lock().unwrap().document().clone();
    let dim = *f.ui.get_widget("graph").unwrap().dim();
    let node = &source.nodes[0];
    let handle = node.fold_handle(&source.metrics());
    let local = f.model.lock().unwrap().editor().viewport.to_screen([
        handle.origin[0] + handle.size[0] * 0.5,
        handle.origin[1] + handle.size[1] * 0.5,
    ]);
    let position = [dim.x as f32 + local[0], dim.y as f32 + local[1]];
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position,
    });
    f.send(InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position,
    });
    let commits = f.commits();
    assert_eq!(commits.len(), 1);
    let (binding, edits) = &commits[0];
    assert_eq!(edits.len(), 1);
    let mut authoritative = source.clone();
    edits[0].apply(&mut authoritative, true);
    assert!(authoritative.nodes[0].folded);
    let mut model = f.model.lock().unwrap();
    assert_eq!(model.document(), &authoritative);
    let serial = model.last_serial();
    assert!(model.acknowledge(*binding, serial));
    assert!(model.publish_edits().is_none());
    drop(model);
    f.paint();
    assert!(f.commits().is_empty());
    edits[0].apply(&mut authoritative, false);
    assert_eq!(authoritative, source);
}

#[test]
fn native_host_control_commits_keep_their_own_serial_and_reversible_source() {
    let mut f = Fixture::new();
    let expected = GraphControlValue::Custom {
        kind: "conversation".into(),
        data: serde_json::json!({"line":"Original", "choices":[]}),
    };
    let value = GraphControlValue::Custom {
        kind: "conversation".into(),
        data: serde_json::json!({"line":"日本\nNew line", "choices":["Continue"]}),
    };
    let row = GraphRow::new("Conversation", expected.clone());
    let row_id = row.id;
    f.model.lock().unwrap().edit(|document, _| {
        document.nodes[0].rows.push(row);
    });
    let source = f.model.lock().unwrap().document().clone();
    f.click_row(0, 0.5);
    f.command(EditorCommand::SelectAll);
    f.send(InputEvent::Text("Completed inline".into()));
    let mut model = f.model.lock().unwrap();
    model.finish_text(true);
    let binding = model.binding();
    let node = model.document().nodes[0].id;
    let notification = model
        .commit_control(binding, node, row_id, &expected, value.clone())
        .unwrap()
        .unwrap();
    assert_eq!(notification.serial, 2);
    let commits = model.pending_through(notification.serial);
    assert_eq!(commits.len(), 2);
    assert!(Arc::ptr_eq(&notification, &commits[1]));
    let mut authoritative = source.clone();
    commits[0].edits[0].apply(&mut authoritative, true);
    assert_eq!(
        authoritative.nodes[0].rows[0].value,
        GraphControlValue::Text("Completed inline".into())
    );
    assert_eq!(authoritative.nodes[0].rows[2].value, expected);
    commits[1].edits[0].apply(&mut authoritative, true);
    assert_eq!(authoritative.nodes[0].rows[2].value, value);
    assert_eq!(&authoritative, model.document());
    assert!(model.acknowledge(binding, notification.serial));
    assert!(
        model
            .commit_control(binding, node, row_id, &value, value.clone())
            .unwrap()
            .is_none()
    );
    assert!(model.pending_through(notification.serial).is_empty());
    commits[1].edits[0].apply(&mut authoritative, false);
    commits[0].edits[0].apply(&mut authoritative, false);
    assert_eq!(authoritative, source);
    drop(model);
    f.paint();
    assert!(f.ui.canvas.buffer.pixels().is_empty());
}

#[test]
fn native_host_control_rejects_stale_sources_and_active_graph_editors_atomically() {
    let f = Fixture::new();
    let mut model = f.model.lock().unwrap();
    let binding = model.binding();
    let node = model.document().nodes[0].id;
    let row = model.document().nodes[0].rows[0].id;
    let expected = model.document().nodes[0].rows[0].value.clone();
    let value = GraphControlValue::Text("New host control".into());
    let initial = model.document().clone();
    for inline in [true, false] {
        model.edit(|document, editor| {
            let r = document.nodes[0].row_rect(if inline { 0 } else { 1 }, &document.metrics());
            editor.pointer_down(
                document,
                editor
                    .viewport
                    .to_screen([r.origin[0] + 12., r.origin[1] + 10.]),
                &BasicGraphControls,
            );
        });
        assert_eq!(model.editor().text_focus().is_some(), inline);
        assert_eq!(model.editor().is_gesturing(), !inline);
        let working = model.document().clone();
        assert_eq!(
            model
                .commit_control(binding, node, row, &expected, value.clone())
                .unwrap_err(),
            GraphDraftError::EditorBusy
        );
        assert_eq!(
            model.document(),
            &working,
            "failed commit must leave the active draft intact"
        );
        assert_eq!(model.last_serial(), 0);
        model.finish_text(false);
        model.edit(|document, editor| editor.cancel_gesture(document));
        assert_eq!(model.document(), &initial);
    }
    let wrong = GraphControlValue::Text("Obsolete expected source".into());
    assert_eq!(
        model
            .commit_control(binding, node, row, &wrong, value.clone())
            .unwrap_err(),
        GraphDraftError::SourceChanged
    );
    assert_eq!(
        model
            .commit_control(
                binding,
                node,
                uuid::Uuid::new_v4(),
                &expected,
                value.clone()
            )
            .unwrap_err(),
        GraphDraftError::MissingTarget
    );
    assert_eq!(model.document(), &initial);
    model.replace_document("test-document", initial.clone());
    assert_eq!(
        model
            .commit_control(binding, node, row, &expected, value)
            .unwrap_err(),
        GraphDraftError::WrongBinding
    );
    assert_eq!(model.document(), &initial);
    assert_eq!(model.last_serial(), 0);
    assert!(model.pending_commits().next().is_none());
}

#[test]
fn retained_source_commits_have_one_publication_order_across_interleaved_views() {
    let f = Fixture::new();
    let source = f.model.lock().unwrap().document().clone();
    let mut first = GraphViewState::new("first-source", source.clone());
    let mut second = GraphViewState::new("second-source", source.clone());
    let (a, b) = (first.binding(), second.binding());
    let node = source.nodes[0].id;
    let row = source.nodes[0].rows[0].id;
    let original = source.nodes[0].rows[0].value.clone();
    let a1_value = GraphControlValue::Text("First edit of A".into());
    let b1_value = GraphControlValue::Text("Edit of B".into());
    let a2_value = GraphControlValue::Text("Later edit of A".into());
    let a1 = first
        .commit_control(a, node, row, &original, a1_value.clone())
        .unwrap()
        .unwrap();
    let b1 = second
        .commit_control(b, node, row, &original, b1_value.clone())
        .unwrap()
        .unwrap();
    let a2 = first
        .commit_control(a, node, row, &a1_value, a2_value.clone())
        .unwrap()
        .unwrap();
    assert!(a1.sequence < b1.sequence && b1.sequence < a2.sequence);
    let mut journal = first.pending_through(a2.serial);
    journal.extend(second.pending_through(b1.serial));
    journal.sort_by_key(|commit| commit.sequence);
    assert_eq!(
        journal
            .iter()
            .map(|commit| (commit.binding, commit.serial))
            .collect::<Vec<_>>(),
        vec![(a, 1), (b, 1), (a, 2)]
    );
    let (mut a_source, mut b_source) = (source.clone(), source.clone());
    for commit in &journal {
        let document = if commit.binding == a {
            &mut a_source
        } else {
            &mut b_source
        };
        for edit in &commit.edits {
            edit.apply(document, true);
        }
    }
    assert_eq!(a_source.nodes[0].rows[0].value, a2_value);
    assert_eq!(b_source.nodes[0].rows[0].value, b1_value);
    for commit in journal.iter().rev() {
        let document = if commit.binding == a {
            &mut a_source
        } else {
            &mut b_source
        };
        for edit in commit.edits.iter().rev() {
            edit.apply(document, false);
        }
    }
    assert_eq!(a_source, source);
    assert_eq!(b_source, source);
    assert!(first.acknowledge(a, a2.serial));
    assert!(second.acknowledge(b, b1.serial));
    assert!(first.pending_through(a1.serial).is_empty());
    assert!(second.pending_through(b1.serial).is_empty());
}
