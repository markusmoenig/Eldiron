use super::*;
use crate::platform::LogicalKey;

fn fixture(multiline: bool, text: &str) -> (TextSystem, EditorController) {
    let mut fonts = TextSystem::default();
    fonts.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let mut editor = EditorController::new(multiline);
    editor.prepare(
        &mut fonts,
        TextOptions {
            family: "Roboto",
            size: 18.,
            width: Some(120.),
            line_height: Some(24.),
        },
    );
    editor.set_text(&mut fonts, text);
    editor.set_focused(&mut fonts, true);
    (fonts, editor)
}
fn key(
    editor: &mut EditorController,
    fonts: &mut TextSystem,
    physical: PhysicalKey,
) -> EditorResponse {
    editor.input(
        fonts,
        &InputEvent::Key {
            physical,
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        },
    )
}
fn text(editor: &mut EditorController, fonts: &mut TextSystem, text: &str) -> EditorResponse {
    editor.input(fonts, &InputEvent::Text(text.into()))
}
fn shortcut(editor: &mut EditorController, fonts: &mut TextSystem, letter: u8) -> EditorResponse {
    editor.input(
        fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Modifiers::default()
        }),
    );
    let result = key(editor, fonts, PhysicalKey::Letter(letter));
    editor.input(fonts, &InputEvent::Modifiers(Modifiers::default()));
    result
}
#[test]
fn composition_preserves_committed_selection_until_commit_and_undo() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    let before = editor.selection_bytes();
    let event = InputEvent::Composition {
        text: "にほん".into(),
        cursor: Some((3, 6)),
    };
    let response = editor.input(&mut fonts, &event);
    assert!(!response.changed);
    assert_eq!(editor.text(), "A東京Z");
    assert_eq!(editor.layout(&mut fonts).source.as_ref(), "AにほんZ");
    assert!(!editor.composition_geometry(&mut fonts).is_empty());
    assert!(editor.ime_area(&mut fonts).height > 0.);
    key(&mut editor, &mut fonts, PhysicalKey::Escape);
    assert_eq!(editor.text(), "A東京Z");
    assert_eq!(editor.selection_bytes(), before);
    assert_eq!(editor.history_bytes(), 0);
    editor.input(&mut fonts, &event);
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "日本".into(),
            cursor: None,
        },
    );
    assert!(editor.caret(&mut fonts).is_none());
    assert!(text(&mut editor, &mut fonts, "日本").changed);
    assert_eq!(editor.text(), "A日本Z");
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "A東京Z");
    assert_eq!(editor.selection_bytes(), before);
    assert!(editor.redo(&mut fonts));
    assert_eq!(editor.text(), "A日本Z");
    assert!(editor.caret(&mut fonts).is_some());
    editor.input(&mut fonts, &event);
    editor.input(&mut fonts, &InputEvent::Focus(false));
    assert!(!editor.is_composing());
    assert_eq!(editor.text(), "A日本Z");
}
#[test]
fn backspace_and_forward_delete_handle_graphemes_and_crlf() {
    for cluster in ["e\u{301}", "👨‍👩‍👧‍👦", "🇩🇪", "கொ", "\r\n", "東"] {
        let original = format!("A{cluster}Z");
        let (mut fonts, mut editor) = fixture(true, &original);
        editor.select_bytes(&mut fonts, 1 + cluster.len(), 1 + cluster.len());
        assert!(
            key(&mut editor, &mut fonts, PhysicalKey::Backspace).changed,
            "{cluster:?}"
        );
        assert_eq!(editor.text(), "AZ", "{cluster:?}");
        editor.undo(&mut fonts);
        editor.select_bytes(&mut fonts, 1, 1);
        assert!(
            key(&mut editor, &mut fonts, PhysicalKey::Delete).changed,
            "{cluster:?}"
        );
        assert_eq!(editor.text(), "AZ", "{cluster:?}");
    }
}
#[test]
fn physical_keys_do_not_insert_text_and_single_line_submits() {
    let (mut fonts, mut editor) = fixture(false, "東");
    assert!(!key(&mut editor, &mut fonts, PhysicalKey::Letter(b'a')).changed);
    assert_eq!(editor.text(), "東");
    assert!(text(&mut editor, &mut fonts, "e\u{301}\n\t").changed);
    assert_eq!(editor.text(), "東e\u{301}");
    let response = key(&mut editor, &mut fonts, PhysicalKey::Enter);
    assert_eq!(response.action, Some(EditorAction::Submit));
    assert_eq!(editor.text(), "東e\u{301}");
    editor.set_focused(&mut fonts, false);
    assert!(!text(&mut editor, &mut fonts, "ignored").input.consumed);
}
#[test]
fn bidi_navigation_uses_visual_carets_without_rebuilding_glyphs() {
    let (mut fonts, mut editor) = fixture(false, "שלום");
    editor.select_bytes(&mut fonts, 0, 0);
    let first = editor.caret(&mut fonts).unwrap();
    let builds = editor.snapshots_built();
    key(&mut editor, &mut fonts, PhysicalKey::Left);
    let next = editor.caret(&mut fonts).unwrap();
    assert!(next.x < first.x);
    assert!(editor.selection_bytes().1 > 0);
    key(&mut editor, &mut fonts, PhysicalKey::Right);
    assert_eq!(editor.selection_bytes(), (0, 0));
    assert_eq!(editor.snapshots_built(), builds);
    editor.point(&mut fonts, [next.x, next.y + next.height / 2.], false);
    assert!(editor.selection_bytes().1 > 0);
    assert_eq!(editor.snapshots_built(), builds);
}
#[test]
fn vertical_wrapped_selection_and_font_owner_invalidation() {
    let (mut fonts, mut editor) = fixture(true, "One two three four five six\n東京\nEnd");
    let lines = editor.layout(&mut fonts).lines();
    assert!(lines.len() > 3);
    editor.select_bytes(&mut fonts, 0, 0);
    key(&mut editor, &mut fonts, PhysicalKey::Down);
    assert!(editor.selection_bytes().1 > 0);
    editor.input(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            shift: true,
            ..Modifiers::default()
        }),
    );
    key(&mut editor, &mut fonts, PhysicalKey::End);
    assert!(!editor.selection(&mut fonts).is_empty());
    let builds = editor.snapshots_built();
    editor.layout(&mut fonts);
    assert_eq!(editor.snapshots_built(), builds);
    let mut replacement = TextSystem::default();
    replacement.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
    assert_eq!(replacement.font_revision(), fonts.font_revision());
    assert_ne!(replacement.font_stamp(), fonts.font_stamp());
    assert!(!editor.layout(&mut replacement).runs.is_empty());
    assert_eq!(editor.snapshots_built(), builds + 1);
    replacement.register_font(include_bytes!("../../embedded/fonts/Roboto-Bold.ttf").to_vec());
    editor.layout(&mut replacement);
    assert_eq!(editor.snapshots_built(), builds + 2);
}
#[test]
fn asynchronous_cut_and_paste_require_success_and_unchanged_editor() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    let cut = shortcut(&mut editor, &mut fonts, b'x');
    let Some(EditorClipboard::Write {
        text: copied,
        cut: Some(request),
    }) = cut.clipboard
    else {
        panic!("cut request")
    };
    assert_eq!(copied, "東京");
    assert_eq!(editor.text(), "A東京Z");
    assert!(
        !editor
            .complete_clipboard(&mut fonts, request, ClipboardResult::Written(false))
            .changed
    );
    let cut = shortcut(&mut editor, &mut fonts, b'x');
    let Some(EditorClipboard::Write {
        cut: Some(request), ..
    }) = cut.clipboard
    else {
        panic!("cut request")
    };
    assert!(
        editor
            .complete_clipboard(&mut fonts, request, ClipboardResult::Written(true))
            .changed
    );
    assert_eq!(editor.text(), "AZ");
    editor.undo(&mut fonts);
    let Some(EditorClipboard::Read { request }) = shortcut(&mut editor, &mut fonts, b'v').clipboard
    else {
        panic!("paste request")
    };
    editor.select_bytes(&mut fonts, 0, 0);
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some("stale".into()))
            )
            .changed
    );
    let Some(EditorClipboard::Read { request }) = shortcut(&mut editor, &mut fonts, b'v').clipboard
    else {
        panic!("paste request")
    };
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request + 1,
                ClipboardResult::Text(Some("wrong".into()))
            )
            .changed
    );
    assert!(
        editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some("新\n".into()))
            )
            .changed
    );
    assert_eq!(editor.text(), "新A東京Z");
    let Some(EditorClipboard::Read { request }) = shortcut(&mut editor, &mut fonts, b'v').clipboard
    else {
        panic!("paste request")
    };
    editor.set_focused(&mut fonts, false);
    editor.set_focused(&mut fonts, true);
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some("late".into()))
            )
            .changed
    );
}
#[test]
fn history_is_bounded_and_replacing_redo_keeps_accounting_exact() {
    let (mut fonts, mut editor) = fixture(false, "");
    editor.history_budget = 64;
    for _ in 0..20 {
        text(&mut editor, &mut fonts, "東");
        assert!(editor.history_bytes() <= 64);
    }
    assert!(editor.undo.is_empty());
    editor.set_text(&mut fonts, "");
    for s in ["A", "B", "C"] {
        text(&mut editor, &mut fonts, s);
    }
    editor.undo(&mut fonts);
    assert_eq!(editor.text(), "AB");
    text(&mut editor, &mut fonts, "Z");
    assert!(!editor.redo(&mut fonts));
    assert_eq!(editor.text(), "ABZ");
    assert_eq!(
        editor.history_bytes(),
        editor.undo.iter().map(|c| c.bytes).sum::<usize>()
    );
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "AB");
}

#[test]
fn clipboard_tokens_do_not_alias_recreated_editors_and_preedit_offsets_follow_filtered_text() {
    let (mut fonts, mut first) = fixture(false, "One");
    let Some(EditorClipboard::Read { request: old }) =
        shortcut(&mut first, &mut fonts, b'v').clipboard
    else {
        panic!("paste")
    };
    drop(first);
    let mut next = EditorController::new(false);
    next.set_text(&mut fonts, "Two");
    next.set_focused(&mut fonts, true);
    let Some(EditorClipboard::Read { request: new }) =
        shortcut(&mut next, &mut fonts, b'v').clipboard
    else {
        panic!("paste")
    };
    assert_ne!(new, old);
    assert!(
        !next
            .complete_clipboard(&mut fonts, old, ClipboardResult::Text(Some("stale".into())))
            .changed
    );
    assert!(
        next.complete_clipboard(&mut fonts, new, ClipboardResult::Text(Some("!".into())))
            .changed
    );
    assert_eq!(next.text(), "Two!");
    next.select_bytes(&mut fonts, 0, 0);
    next.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "\n東\t京".into(),
            cursor: Some((5, 8)),
        },
    );
    assert_eq!(next.layout(&mut fonts).source.as_ref(), "東京Two!");
    assert_eq!(next.selection_bytes(), (3, 6));
    assert_eq!(next.text(), "Two!");
}
