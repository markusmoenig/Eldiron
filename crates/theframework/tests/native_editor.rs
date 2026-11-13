#![cfg(all(feature = "ui", feature = "text"))]
use theframework::{
    platform::{ClipboardResult, InputEvent, LogicalKey, Modifiers, PhysicalKey},
    prelude::*,
    text_edit::{EditorClipboard, EditorCommand},
};
fn key(physical: PhysicalKey) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    }
}
fn fixture() -> (TheUI, TheContext, TheId, TheId) {
    let mut ctx = TheContext::new(400, 120, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let a = TheId::named("First");
    let b = TheId::named("Second");
    let mut row = TheHLayout::new(TheId::named("Editors"));
    let mut first = TheTextLineEdit::new(a.clone());
    first.set_text("A東京Z".into());
    row.widgets().push(Box::new(first));
    row.widgets()
        .push(Box::new(TheTextLineEdit::new(b.clone())));
    ui.canvas.set_layout(row);
    ui.prepare_paint_layout(&mut ctx);
    ctx.ui.set_focus(&a);
    ui.process_events(&mut ctx);
    (ui, ctx, a, b)
}
fn send(ui: &mut TheUI, ctx: &mut TheContext, event: InputEvent) {
    ui.prepare_paint_layout(ctx);
    ui.dispatch_input(&event, ctx);
}
#[test]
fn native_field_preedit_commits_once_and_menu_undo_uses_the_same_history() {
    let (mut ui, mut ctx, _, _) = fixture();
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'a')));
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers::default()),
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Composition {
            text: "仮".into(),
            cursor: Some((0, 3)),
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    send(&mut ui, &mut ctx, InputEvent::Text("e\u{301}👨‍👩‍👧‍👦".into()));
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "e\u{301}👨‍👩‍👧‍👦");
    send(&mut ui, &mut ctx, key(PhysicalKey::Backspace));
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "e\u{301}");
    ui.undo(&mut ctx);
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "e\u{301}👨‍👩‍👧‍👦");
    ui.undo(&mut ctx);
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    ui.redo(&mut ctx);
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "e\u{301}👨‍👩‍👧‍👦");
}
#[test]
fn clipboard_is_host_serviced_and_old_completions_cannot_edit_another_field() {
    let (mut ui, mut ctx, a, b) = fixture();
    ui.get_widget_id(a.uuid)
        .unwrap()
        .editor_command(EditorCommand::SelectAll, &mut ctx);
    ui.cut(&mut ctx);
    let Some(EditorClipboard::Write {
        cut: Some(request),
        text,
    }) = ctx.ui.editor_clipboard.pop_front()
    else {
        panic!("host cut request");
    };
    assert_eq!(text, "A東京Z");
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Clipboard {
            request,
            result: ClipboardResult::Written(false),
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    ui.cut(&mut ctx);
    let Some(EditorClipboard::Write {
        cut: Some(request), ..
    }) = ctx.ui.editor_clipboard.pop_front()
    else {
        panic!("second cut");
    };
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Clipboard {
            request,
            result: ClipboardResult::Written(true),
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "");
    ui.undo(&mut ctx);
    ui.paste(&mut ctx);
    let Some(EditorClipboard::Read { request }) = ctx.ui.editor_clipboard.pop_front() else {
        panic!("host paste request");
    };
    send(&mut ui, &mut ctx, key(PhysicalKey::Tab));
    assert_eq!(ctx.ui.focus.as_ref(), Some(&b));
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Clipboard {
            request,
            result: ClipboardResult::Text(Some("stale".into())),
        },
    );
    assert_eq!(ui.get_text_line_edit("Second").unwrap().text(), "");
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    ui.paste(&mut ctx);
    let Some(EditorClipboard::Read { request }) = ctx.ui.editor_clipboard.pop_front() else {
        panic!("new host paste");
    };
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Clipboard {
            request,
            result: ClipboardResult::Text(Some("東京\n".into())),
        },
    );
    assert_eq!(ui.get_text_line_edit("Second").unwrap().text(), "東京");
}
#[test]
fn shift_tab_modifiers_survive_focus_changes_and_preedit_owns_tab() {
    let (mut ui, mut ctx, a, b) = fixture();
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            shift: true,
            ..Default::default()
        }),
    );
    for expected in [&b, &a, &b] {
        send(&mut ui, &mut ctx, key(PhysicalKey::Tab));
        assert_eq!(ctx.ui.focus.as_ref(), Some(expected));
    }
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Composition {
            text: "仮".into(),
            cursor: None,
        },
    );
    send(&mut ui, &mut ctx, key(PhysicalKey::Tab));
    assert_eq!(ctx.ui.focus.as_ref(), Some(&b));
}
#[test]
fn modal_scope_cancels_native_preedit_and_rotates_the_returning_editor_identity() {
    let (mut ui, mut ctx, a, _) = fixture();
    let before = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    send(
        &mut ui,
        &mut ctx,
        InputEvent::CompositionFor {
            target: before,
            text: "仮".into(),
            cursor: None,
        },
    );
    ui.context_menu = Some(TheContextMenu::new());
    ui.prepare_paint_layout(&mut ctx);
    assert!(
        ui.get_widget_id(a.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .is_none()
    );
    assert!(
        ui.dispatch_input(
            &InputEvent::TextFor {
                target: before,
                text: "late".into()
            },
            &mut ctx
        )
        .consumed
    );
    ui.context_menu = None;
    ui.prepare_paint_layout(&mut ctx);
    let after = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    assert_ne!(after, before);
    send(
        &mut ui,
        &mut ctx,
        InputEvent::TextFor {
            target: before,
            text: "stale".into(),
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
}
#[test]
fn same_text_replacement_is_a_new_document_even_before_the_next_host_state_update() {
    let (mut ui, mut ctx, a, _) = fixture();
    let before = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    ui.get_text_line_edit("First")
        .unwrap()
        .set_text("A東京Z".into());
    assert!(
        !ui.dispatch_input(
            &InputEvent::TextFor {
                target: before,
                text: "stale".into()
            },
            &mut ctx
        )
        .consumed
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    let after = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    assert_ne!(before, after);
}
#[test]
fn numeric_drag_commits_one_undo_unit_and_scope_loss_restores_an_uncommitted_gesture() {
    let mut ctx = TheContext::new(240, 80, 1.);
    ctx.ui.command_paint = true;
    let (sender, receiver) = std::sync::mpsc::channel();
    ctx.ui.state_events_sender = Some(sender);
    let id = TheId::named("Range");
    let mut field = TheTextLineEdit::new(id.clone());
    field.set_dim(TheDim::new(10, 10, 200, 30), &mut ctx);
    field.set_range(TheValue::RangeI32(0..=10));
    field.set_value(TheValue::Int(2));
    ctx.ui.set_focus(&id);
    field.editor_set_active(true, &mut ctx);
    field.on_event(&TheEvent::MouseDown(Vec2::new(10, 10)), &mut ctx);
    for x in [60, 80, 100] {
        field.on_event(&TheEvent::MouseDragged(Vec2::new(x, 10)), &mut ctx);
    }
    assert_eq!(field.value().to_i32(), Some(5));
    assert!(
        !receiver
            .try_iter()
            .any(|e| matches!(e, TheEvent::ValueChanged(..)))
    );
    field.on_event(&TheEvent::MouseUp(Vec2::new(100, 10)), &mut ctx);
    assert_eq!(
        receiver
            .try_iter()
            .filter(|e| matches!(e, TheEvent::ValueChanged(..)))
            .count(),
        1
    );
    field.editor_command(EditorCommand::Undo, &mut ctx);
    assert_eq!(field.value().to_i32(), Some(2));
    field.editor_command(EditorCommand::Redo, &mut ctx);
    assert_eq!(field.value().to_i32(), Some(5));
    field.on_event(&TheEvent::MouseDown(Vec2::new(10, 10)), &mut ctx);
    field.on_event(&TheEvent::MouseDragged(Vec2::new(180, 10)), &mut ctx);
    assert_eq!(field.value().to_i32(), Some(9));
    field.editor_set_active(false, &mut ctx);
    assert_eq!(field.value().to_i32(), Some(5));
    let _ = receiver.try_iter().collect::<Vec<_>>();
    field.on_event(&TheEvent::MouseUp(Vec2::new(180, 10)), &mut ctx);
    assert!(
        !receiver
            .try_iter()
            .any(|e| matches!(e, TheEvent::ValueChanged(..)))
    );
    field.editor_set_active(true, &mut ctx);
    field.editor_command(EditorCommand::Undo, &mut ctx);
    assert_eq!(field.value().to_i32(), Some(2));
    field.on_event(&TheEvent::MouseDown(Vec2::new(10, 10)), &mut ctx);
    field.on_event(&TheEvent::MouseDragged(Vec2::new(180, 10)), &mut ctx);
    field.set_value(TheValue::Int(7));
    field.editor_set_active(false, &mut ctx);
    field.editor_set_active(true, &mut ctx);
    field.editor_input_state(&mut ctx);
    assert_eq!(
        field.value().to_i32(),
        Some(7),
        "a pending blur cannot overwrite an externally replaced value"
    );
}

#[test]
fn tree_editor_forwards_addressed_input_clipboard_and_row_local_ime_geometry() {
    let mut ctx = TheContext::new(400, 120, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let mut tree = TheTreeLayout::new(TheId::named("Tree"));
    tree.set_headerless(true);
    tree.get_root().set_open(true);
    let row_id = TheId::named("Row");
    let mut row = TheTreeItem::new(row_id.clone());
    let mut field = TheTextLineEdit::new(TheId::named("Column"));
    field.set_text("A東京Z".into());
    row.add_widget_column(180, Box::new(field));
    tree.get_root().add_widget(Box::new(row));
    ui.canvas.set_layout(tree);
    ui.prepare_paint_layout(&mut ctx);
    ctx.ui.set_focus(&row_id);
    ui.process_events(&mut ctx);
    let row = ui.get_widget_id(row_id.uuid).unwrap();
    let dim = *row.dim();
    let state = row.editor_input_state(&mut ctx).unwrap();
    assert!(state.area.x >= dim.x as f32 && state.area.x <= (dim.x + dim.width) as f32);
    assert!(state.area.y >= dim.y as f32 && state.area.y <= (dim.y + dim.height) as f32);
    use theframework::text_edit::{NativeTextAnswer as A, NativeTextQuery as Q, NativeTextRequest};
    let source = ui
        .get_widget_id(row_id.uuid)
        .unwrap()
        .editor_native_text_snapshot(&mut ctx)
        .unwrap();
    let request = |query| NativeTextRequest {
        target: source.target,
        revision: source.revision,
        query,
    };
    let A::Geometry { caret, .. } = ui
        .get_widget_id(row_id.uuid)
        .unwrap()
        .editor_native_text_query(
            &request(Q::Geometry(source.selection.1..source.selection.1)),
            &mut ctx,
        )
        .unwrap()
    else {
        panic!("geometry")
    };
    // The candidate area deliberately includes surrounding context; compare
    // the child layout geometry after applying the row paint origin instead.
    let child = ui
        .get_widget_id(row_id.uuid)
        .unwrap()
        .as_tree_item()
        .unwrap()
        .embedded_widget_mut()
        .unwrap();
    let A::Geometry { caret: local, .. } = child
        .editor_native_text_query(
            &request(Q::Geometry(source.selection.1..source.selection.1)),
            &mut ctx,
        )
        .unwrap()
    else {
        panic!("geometry")
    };
    assert!(
        (caret.x - local.x - dim.x as f32).abs() < 0.001
            && (caret.y - local.y - dim.y as f32).abs() < 0.001
    );
    assert_eq!(
        ui.get_widget_id(row_id.uuid)
            .unwrap()
            .editor_native_text_query(
                &request(Q::Hit([caret.x, caret.y + caret.height / 2.])),
                &mut ctx
            ),
        Some(A::Hit(source.selection.1))
    );
    ui.get_widget_id(row_id.uuid)
        .unwrap()
        .editor_command(EditorCommand::SelectAll, &mut ctx);
    send(
        &mut ui,
        &mut ctx,
        InputEvent::TextFor {
            target: state.target,
            text: "新".into(),
        },
    );
    let value = |ui: &mut TheUI| {
        ui.get_widget_id(row_id.uuid)
            .unwrap()
            .as_tree_item()
            .unwrap()
            .embedded_widget_mut()
            .unwrap()
            .value()
    };
    assert_eq!(value(&mut ui), TheValue::Text("新".into()));
    ui.paste(&mut ctx);
    let Some(EditorClipboard::Read { request }) = ctx.ui.editor_clipboard.pop_front() else {
        panic!("embedded paste request");
    };
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Clipboard {
            request,
            result: ClipboardResult::Text(Some("!".into())),
        },
    );
    assert_eq!(value(&mut ui), TheValue::Text("新!".into()));
    ui.get_layout("Tree")
        .unwrap()
        .as_tree_layout()
        .unwrap()
        .get_root()
        .set_open(false);
    ui.prepare_paint_layout(&mut ctx);
    assert!(
        ui.get_widget_id(row_id.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .is_none()
    );
}

#[test]
fn physical_application_accelerators_work_while_native_text_commands_keep_local_priority() {
    let (mut ui, mut ctx, _, _) = fixture();
    let receiver = ui.add_state_listener("Native shortcuts".into());
    let save = TheId::named("Save");
    let other_copy = TheId::named("Other Copy");
    ctx.ui.accelerators.insert(
        save.clone(),
        TheAccelerator::new(TheAcceleratorKey::CTRL, 's'),
    );
    ctx.ui.accelerators.insert(
        other_copy.clone(),
        TheAccelerator::new(TheAcceleratorKey::CTRL, 'c'),
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    assert!(
        ui.dispatch_input(&key(PhysicalKey::Letter(b's')), &mut ctx)
            .consumed
    );
    assert!(
        receiver
            .try_iter()
            .any(|event| matches!(event,TheEvent::ContextMenuSelected(id,_) if id == save))
    );
    send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'a')));
    send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'c')));
    assert!(
        !receiver
            .try_iter()
            .any(|event| matches!(event,TheEvent::ContextMenuSelected(id,_) if id == other_copy))
    );
    assert!(
        matches!(ctx.ui.editor_clipboard.pop_front(),Some(EditorClipboard::Write { text,cut:None }) if text == "A東京Z")
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
}
#[test]
fn popover_editor_escape_cancels_preedit_before_dismissing_its_scope() {
    let (mut ui, mut ctx, _, _) = fixture();
    let editor = TheId::named("Popover Field");
    let mut canvas = TheCanvas::new();
    canvas.limiter.set_max_width(220);
    canvas.limiter.set_max_height(60);
    canvas.set_widget(TheTextLineEdit::new(editor.clone()));
    ui.show_popover(
        TheId::named("Popover"),
        TheDim::new(20, 20, 100, 20),
        canvas,
        &mut ctx,
    );
    ui.prepare_paint_layout(&mut ctx);
    ctx.ui.set_focus(&editor);
    ui.process_events(&mut ctx);
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Composition {
            text: "仮".into(),
            cursor: None,
        },
    );
    send(&mut ui, &mut ctx, key(PhysicalKey::Escape));
    assert!(ui.popover.is_some());
    send(&mut ui, &mut ctx, key(PhysicalKey::Escape));
    assert!(ui.popover.is_none());
}

#[test]
fn embedded_scrollbar_gestures_keep_the_editor_focus_and_native_input_epoch() {
    let (mut ui, mut ctx, a, _) = fixture();
    let before = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    let mut horizontal = TheHorizontalScrollbar::new(TheId::named("Horizontal"));
    horizontal.set_parent_id(a.clone());
    horizontal.set_dim(TheDim::new(0, 0, 100, 16), &mut ctx);
    horizontal
        .as_horizontal_scrollbar()
        .unwrap()
        .set_total_width(1000);
    let mut vertical = TheVerticalScrollbar::new(TheId::named("Vertical"));
    vertical.set_parent_id(a.clone());
    vertical.set_dim(TheDim::new(0, 0, 16, 100), &mut ctx);
    vertical
        .as_vertical_scrollbar()
        .unwrap()
        .set_total_height(1000);
    horizontal.on_event(&TheEvent::MouseDown(Vec2::new(5, 5)), &mut ctx);
    horizontal.on_event(&TheEvent::MouseDragged(Vec2::new(25, 5)), &mut ctx);
    horizontal.on_event(&TheEvent::MouseUp(Vec2::new(25, 5)), &mut ctx);
    assert!(
        horizontal
            .as_horizontal_scrollbar()
            .unwrap()
            .scroll_offset()
            > 0
    );
    vertical.on_event(&TheEvent::MouseDown(Vec2::new(5, 5)), &mut ctx);
    vertical.on_event(&TheEvent::MouseDragged(Vec2::new(5, 25)), &mut ctx);
    vertical.on_event(&TheEvent::MouseUp(Vec2::new(5, 25)), &mut ctx);
    assert!(vertical.as_vertical_scrollbar().unwrap().scroll_offset() > 0);
    ui.process_events(&mut ctx);
    assert_eq!(ctx.ui.focus.as_ref(), Some(&a));
    assert_eq!(
        ui.get_widget_id(a.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .unwrap()
            .target,
        before
    );
}

fn code_fixture(source: &str, external_undo: bool) -> (TheUI, TheContext, TheId) {
    let mut ctx = TheContext::new(420, 240, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let id = TheId::named("Code");
    let mut area = TheTextAreaEdit::new(id.clone());
    area.set_text(source.into());
    area.as_code_editor("Python", TheCodeEditorSettings::default());
    area.set_continuous(true);
    area.set_supports_undo(!external_undo);
    area.display_line_number(true);
    area.use_statusbar(true);
    ui.canvas.set_widget(area);
    ui.prepare_paint_layout(&mut ctx);
    ctx.ui.set_focus(&id);
    ui.process_events(&mut ctx);
    (ui, ctx, id)
}
#[test]
fn multiline_widget_uses_common_unicode_code_policy_and_one_edit_history() {
    let (mut ui, mut ctx, _) = code_fixture("    東京", false);
    ui.get_text_area_edit("Code").unwrap().goto_char_by_index(6);
    send(&mut ui, &mut ctx, key(PhysicalKey::Enter));
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "    東京\n    "
    );
    send(&mut ui, &mut ctx, InputEvent::Text("(".into()));
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "    東京\n    ()"
    );
    send(&mut ui, &mut ctx, InputEvent::Text("e\u{301}".into()));
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "    東京\n    (e\u{301})"
    );
    ui.undo(&mut ctx);
    ui.undo(&mut ctx);
    ui.undo(&mut ctx);
    assert_eq!(ui.get_text_area_edit("Code").unwrap().text(), "    東京");
    ui.redo(&mut ctx);
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "    東京\n    "
    );
}
#[test]
fn creator_style_external_undo_receives_the_shortcut_and_installs_its_saved_state() {
    let (mut ui, mut ctx, id) = code_fixture("A東京Z", true);
    let before = ui.get_text_area_edit("Code").unwrap().get_state();
    let receiver = ui.add_state_listener("External document undo".into());
    let undo = TheId::named("Document Undo");
    ctx.ui.accelerators.insert(
        undo.clone(),
        TheAccelerator::new(TheAcceleratorKey::CTRL, 'z'),
    );
    send(&mut ui, &mut ctx, InputEvent::Text("!".into()));
    assert_eq!(ui.get_text_area_edit("Code").unwrap().text(), "!A東京Z");
    let changed = receiver
        .try_iter()
        .filter(|event| matches!(event,TheEvent::ValueChanged(value,_) if value==&id))
        .count();
    assert_eq!(changed, 1);
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'z')));
    assert!(
        receiver
            .try_iter()
            .any(|event| matches!(event,TheEvent::ContextMenuSelected(value,_) if value==undo))
    );
    assert_eq!(ui.get_text_area_edit("Code").unwrap().text(), "!A東京Z");
    let target = ui
        .get_widget_id(id.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    TheTextAreaEditTrait::set_state(ui.get_text_area_edit("Code").unwrap(), before);
    ui.prepare_paint_layout(&mut ctx);
    assert_eq!(ui.get_text_area_edit("Code").unwrap().text(), "A東京Z");
    assert_ne!(
        ui.get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .unwrap()
            .target,
        target
    );
    // The deferred Creator listener retains input modifiers; traditional
    // observers still receive the same keys after the host has released Ctrl.
    let captured = ui.add_input_state_listener("Deferred Creator input".into());
    ctx.ui.clear_focus();
    ctx.ui.command_paint = true;
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Key {
            physical: PhysicalKey::Other(91),
            logical: LogicalKey::Character("[".into()),
            pressed: true,
            repeat: false,
        },
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers::default()),
    );
    ui.process_events(&mut ctx);
    assert!(!ui.ctrl && !ctx.ui.input_modifiers.control);
    assert!(captured.try_iter().any(|event| matches!(event,
        TheEvent::InputKey { event, modifiers } if modifiers.control
            && matches!(*event, TheEvent::KeyDown(TheValue::Char('['))))));
    let legacy: Vec<_> = receiver.try_iter().collect();
    assert!(
        legacy
            .iter()
            .any(|event| matches!(event, TheEvent::KeyDown(TheValue::Char('['))))
    );
    assert!(
        !legacy
            .iter()
            .any(|event| matches!(event, TheEvent::InputKey { .. }))
    );
}

#[test]
fn readonly_multiline_widget_copies_visible_source_and_rejects_text_and_paste() {
    let (mut ui, mut ctx, id) = code_fixture("visible 東京\nsecond", false);
    let area = ui.get_text_area_edit("Code").unwrap();
    area.readonly(true);
    area.set_word_wrap(true);
    area.auto_scroll_to_cursor(false);
    area.set_highlight_source(Some("# hidden marker\n# source style".into()));
    ui.prepare_paint_layout(&mut ctx);
    assert!(
        ui.get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .is_none()
    );
    send(&mut ui, &mut ctx, InputEvent::Text("stale".into()));
    ui.paste(&mut ctx);
    assert!(ctx.ui.editor_clipboard.is_empty());
    ui.get_widget_id(id.uuid)
        .unwrap()
        .editor_command(EditorCommand::SelectAll, &mut ctx);
    ui.copy(&mut ctx);
    assert!(
        matches!(ctx.ui.editor_clipboard.pop_front(),Some(EditorClipboard::Write { text,cut:None }) if text=="visible 東京\nsecond")
    );
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "visible 東京\nsecond"
    );
}

#[test]
fn multiline_loaded_controls_survive_edits_and_code_zoom_is_not_a_document_edit() {
    let source = "A\u{000c}東京\0Z";
    let (mut ui, mut ctx, id) = code_fixture(source, false);
    let receiver = ui.add_state_listener("Code shortcuts".into());
    send(&mut ui, &mut ctx, InputEvent::Text("!".into()));
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        format!("!{source}")
    );
    let _ = receiver.try_iter().collect::<Vec<_>>();
    let target = ui
        .get_widget_id(id.uuid)
        .unwrap()
        .editor_input_state(&mut ctx)
        .unwrap()
        .target;
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Key {
            physical: PhysicalKey::Other(1),
            logical: LogicalKey::Character("+".into()),
            pressed: true,
            repeat: false,
        },
    );
    assert!(
        !receiver
            .try_iter()
            .any(|event| matches!(event, TheEvent::ValueChanged(..)))
    );
    assert_eq!(
        ui.get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(&mut ctx)
            .unwrap()
            .target,
        target
    );
    send(
        &mut ui,
        &mut ctx,
        InputEvent::Key {
            physical: PhysicalKey::Other(2),
            logical: LogicalKey::Character("/".into()),
            pressed: true,
            repeat: false,
        },
    );
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        format!("# !{source}")
    );
    ui.undo(&mut ctx);
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        format!("!{source}")
    );
}

#[test]
fn multiline_drag_move_updates_the_saved_document_once_and_undo_restores_selection() {
    let source = "A東京e\u{301} Z";
    let (mut ui, mut ctx, id) = code_fixture(source, false);
    let caret = |ui: &mut TheUI, ctx: &mut TheContext, index| {
        ui.get_text_area_edit("Code")
            .unwrap()
            .goto_char_by_index(index);
        ui.prepare_paint_layout(ctx);
        let state = ui
            .get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(ctx)
            .unwrap();
        state.area
    };
    let start_area = caret(&mut ui, &mut ctx, 1);
    let end_area = caret(&mut ui, &mut ctx, 6);
    // Parley's native candidate rectangle includes three glyphs of context
    // on either side, clipped at the start of the line. Use its right edge
    // to recover the insertion position without treating context as a caret.
    let context = end_area.width / 2.;
    let press = [
        start_area.x + start_area.width - context + 2.,
        start_area.y + start_area.height / 2.,
    ];
    let end = [end_area.x + context, end_area.y + end_area.height / 2.];
    let mut selected = ui.get_text_area_edit("Code").unwrap().get_state();
    selected.select(1, 4);
    TheTextAreaEditTrait::set_state(ui.get_text_area_edit("Code").unwrap(), selected);
    ui.prepare_paint_layout(&mut ctx);
    let receiver = ui.add_state_listener("Drag move".into());
    ui.touch_down(press[0], press[1], &mut ctx);
    ui.touch_dragged(end[0], end[1], &mut ctx);
    ui.process_events(&mut ctx);
    assert_eq!(ui.get_text_area_edit("Code").unwrap().text(), source);
    assert!(
        !receiver
            .try_iter()
            .any(|event| matches!(event, TheEvent::ValueChanged(..)))
    );
    ui.touch_up(end[0], end[1], &mut ctx);
    ui.process_events(&mut ctx);
    assert_eq!(
        ui.get_text_area_edit("Code").unwrap().text(),
        "A Z東京e\u{301}"
    );
    assert_eq!(
        receiver
            .try_iter()
            .filter(|event| matches!(event, TheEvent::ValueChanged(value, _) if value==&id))
            .count(),
        1
    );
    assert_eq!(
        ui.get_text_area_edit("Code")
            .unwrap()
            .get_state()
            .rows
            .join("\n"),
        "A Z東京e\u{301}"
    );
    assert!(ctx.ui.editor_clipboard.is_empty());
    ui.undo(&mut ctx);
    let restored = ui.get_text_area_edit("Code").unwrap().get_state();
    assert_eq!(restored.rows.join("\n"), source);
    assert_eq!((restored.selection.start, restored.selection.end), (1, 4));
}

#[test]
fn rich_and_markdown_selection_copy_share_native_commands_without_ime_or_document_mutation() {
    for markdown in [false, true] {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(420, 200, 1.);
        ui.init(&mut ctx);
        let id = TheId::named("Display");
        let source = "A東京e\u{301}\nsecond";
        if markdown {
            let mut display = TheMarkdownView::new(id.clone());
            display.set_text("**A東京e\u{301}**\n\nsecond".into());
            ui.canvas.set_widget(display);
        } else {
            let mut display = TheTextView::new(id.clone());
            display.set_text(source.into());
            ui.canvas.set_widget(display);
        }
        ui.prepare_paint_layout(&mut ctx);
        ctx.ui.set_focus(&id);
        ui.process_events(&mut ctx);
        let widget = ui.get_widget_id(id.uuid).unwrap();
        assert!(!widget.supports_text_input());
        assert!(!widget.supports_undo_redo());
        assert!(widget.uses_native_editor());
        assert!(widget.editor_input_state(&mut ctx).is_none());
        send(
            &mut ui,
            &mut ctx,
            InputEvent::Modifiers(Modifiers {
                control: true,
                ..Default::default()
            }),
        );
        send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'a')));
        send(&mut ui, &mut ctx, key(PhysicalKey::Letter(b'c')));
        assert!(
            matches!(ctx.ui.editor_clipboard.pop_front(), Some(EditorClipboard::Write { text, cut: None }) if text==source)
        );
        send(
            &mut ui,
            &mut ctx,
            InputEvent::Modifiers(Modifiers::default()),
        );
        for event in [
            InputEvent::Text("bad".into()),
            InputEvent::Composition {
                text: "仮".into(),
                cursor: None,
            },
            key(PhysicalKey::Backspace),
        ] {
            send(&mut ui, &mut ctx, event);
        }
        ui.cut(&mut ctx);
        ui.paste(&mut ctx);
        ui.undo(&mut ctx);
        assert!(ctx.ui.editor_clipboard.is_empty());
        assert_eq!(
            ui.get_widget_id(id.uuid).unwrap().value(),
            TheValue::Text(source.into())
        );
        ui.copy(&mut ctx);
        assert!(
            matches!(ctx.ui.editor_clipboard.pop_front(), Some(EditorClipboard::Write { text, cut: None }) if text==source)
        );
        ui.context_menu = Some(TheContextMenu::new());
        ui.prepare_paint_layout(&mut ctx);
        ui.copy(&mut ctx);
        ui.paste(&mut ctx);
        assert!(ctx.ui.editor_clipboard.is_empty());
        ui.context_menu = None;
        ui.prepare_paint_layout(&mut ctx);
        ui.copy(&mut ctx);
        assert!(
            matches!(ctx.ui.editor_clipboard.pop_front(), Some(EditorClipboard::Write { text, cut: None }) if text==source)
        );
    }
}

#[test]
fn tab_traversal_includes_selectable_displays_and_skips_nonselectable_help() {
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1000, 150, 1.);
    ui.init(&mut ctx);
    let mut row = TheHLayout::new(TheId::named("Text owners"));
    let a = TheId::named("Edit");
    let b = TheId::named("Rich");
    let c = TheId::named("Markdown");
    row.widgets()
        .push(Box::new(TheTextLineEdit::new(a.clone())));
    let mut help = TheTextView::new(TheId::named("Help"));
    help.set_text("Nonselectable hint".into());
    help.set_selectable(false);
    row.widgets().push(Box::new(help));
    let mut rich = TheTextView::new(b.clone());
    rich.set_text("Selectable rich text".into());
    row.widgets().push(Box::new(rich));
    let mut markdown = TheMarkdownView::new(c.clone());
    markdown.set_text("**Selectable** text".into());
    row.widgets().push(Box::new(markdown));
    ui.canvas.set_layout(row);
    ui.prepare_paint_layout(&mut ctx);
    ctx.ui.set_focus(&a);
    ui.process_events(&mut ctx);
    for expected in [&b, &c, &a, &b] {
        send(&mut ui, &mut ctx, key(PhysicalKey::Tab));
        assert_eq!(ctx.ui.focus.as_ref(), Some(expected));
    }
}

#[cfg(feature = "render-wgpu")]
#[test]
fn display_copy_reaches_host_services_without_starting_a_gpu_or_enabling_ime() {
    use theframework::{platform::*, render_wgpu::UiFrameImages};
    struct App;
    impl UiApplication for App {
        fn initialize(
            &mut self,
            _: &mut UiSession,
            _: &mut UiFrameImages<'_>,
        ) -> renderkit::Result<()> {
            unreachable!()
        }
    }
    let mut app = UiApplicationAdapter::new(App);
    let id = TheId::named("Transcript");
    let mut display = TheTextView::new(id.clone());
    display.set_text("A東京e\u{301}".into());
    app.session.ui.canvas.set_widget(display);
    app.session
        .ui
        .prepare_paint_layout(&mut app.session.context);
    app.session.context.ui.set_focus(&id);
    let mut host = HostContext::new((1200, 700));
    app.input(
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
        &mut host,
    );
    app.input(&key(PhysicalKey::Letter(b'a')), &mut host);
    app.input(&key(PhysicalKey::Letter(b'c')), &mut host);
    assert!(host.text_input.is_none());
    assert_eq!(host.requests.len(), 1);
    assert!(
        matches!(host.requests.pop(), Some(HostRequest::ClipboardWrite { text, request: None }) if text=="A東京e\u{301}")
    );
}

#[test]
fn native_source_range_edits_reach_the_focused_widget_and_share_its_undo_owner() {
    use theframework::platform::NativeTextAction;
    let (mut ui, mut ctx, a, b) = fixture();
    let source = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_native_text_snapshot(&mut ctx)
        .unwrap();
    assert_eq!(&*source.text, "A東京Z");
    send(
        &mut ui,
        &mut ctx,
        InputEvent::NativeText {
            target: source.target,
            revision: source.revision,
            action: NativeTextAction::Compose {
                range: Some(1..7),
                text: "日本".into(),
                cursor: Some((6, 6)),
            },
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    let marked = ui
        .get_widget_id(a.uuid)
        .unwrap()
        .editor_native_text_snapshot(&mut ctx)
        .unwrap();
    assert_eq!(&*marked.text, "A日本Z");
    send(
        &mut ui,
        &mut ctx,
        InputEvent::NativeText {
            target: marked.target,
            revision: marked.revision,
            action: NativeTextAction::CommitComposition,
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A日本Z");
    ui.undo(&mut ctx);
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    ctx.ui.set_focus(&b);
    ui.process_events(&mut ctx);
    send(
        &mut ui,
        &mut ctx,
        InputEvent::NativeText {
            target: source.target,
            revision: source.revision,
            action: NativeTextAction::Replace {
                range: 0..1,
                text: "stale".into(),
            },
        },
    );
    assert_eq!(ui.get_text_line_edit("First").unwrap().text(), "A東京Z");
    assert_eq!(ui.get_text_line_edit("Second").unwrap().text(), "");
    assert!(ui.canvas.buffer.pixels().is_empty());
}
