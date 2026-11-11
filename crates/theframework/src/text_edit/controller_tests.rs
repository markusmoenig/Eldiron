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
    editor.history_budget = 1024;
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

#[test]
fn user_arrows_stop_at_extended_grapheme_boundaries() {
    let source = "A e\u{301} 👨‍👩‍👧‍👦 🇩🇪 கொ 東京 Z";
    let (mut fonts, mut editor) = fixture(false, source);
    let mut boundaries: Vec<_> = source.grapheme_indices(true).map(|(i, _)| i).collect();
    boundaries.push(source.len());
    for keycode in [PhysicalKey::Left, PhysicalKey::Right] {
        editor.select_bytes(
            &mut fonts,
            if keycode == PhysicalKey::Left {
                source.len()
            } else {
                0
            },
            if keycode == PhysicalKey::Left {
                source.len()
            } else {
                0
            },
        );
        for _ in 0..source.chars().count() * 2 {
            key(&mut editor, &mut fonts, keycode);
            let (_, focus) = editor.selection_bytes();
            assert!(
                boundaries.contains(&focus),
                "{keycode:?} stopped at {focus}, inside a grapheme in {source:?}"
            );
        }
    }
}
#[test]
fn undo_preserves_the_visual_side_of_a_bidi_boundary() {
    let (mut fonts, mut editor) = fixture(false, "abc אבג xyz");
    let width = editor.layout(&mut fonts).width();
    let mut found = false;
    for x in 0..=(width * 2.) as usize {
        editor.point(&mut fonts, [x as f32 * 0.5, 12.], false);
        let before = editor.caret(&mut fonts).unwrap();
        let selection = *editor.editor.raw_selection();
        let (a, b) = editor.selection_bytes();
        if a != b || a == 0 || a == editor.text().len() {
            continue;
        }
        // Restore-by-index chooses the default side; find a point on the other
        // side, where losing affinity changes visible caret position.
        editor.select_bytes(&mut fonts, a, b);
        let by_index = editor.caret(&mut fonts).unwrap();
        if (by_index.x - before.x).abs() < 2. {
            continue;
        }
        editor.point(&mut fonts, [x as f32 * 0.5, 12.], false);
        assert_eq!(*editor.editor.raw_selection(), selection);
        text(&mut editor, &mut fonts, "!");
        assert!(editor.undo(&mut fonts));
        let after = editor.caret(&mut fonts).unwrap();
        assert!(
            (after.x - before.x).abs() < 0.1,
            "bidi undo moved caret from {before:?} to {after:?}"
        );
        found = true;
        break;
    }
    assert!(
        found,
        "fixture must exercise both sides of a directional boundary"
    );
}

#[test]
fn user_pointer_and_vertical_navigation_do_not_split_graphemes() {
    let source = "A e\u{301} 👨‍👩‍👧‍👦 🇩🇪 கொ 東京 Z\nABCDEF\nகொ கொ கொ\ne\u{301} e\u{301}";
    let (mut fonts, mut editor) = fixture(true, source);
    let mut boundaries: Vec<_> = source.grapheme_indices(true).map(|(i, _)| i).collect();
    boundaries.push(source.len());
    let lines = editor.layout(&mut fonts).lines();
    for line in lines {
        for x in 0..=(line.advance * 2.) as usize {
            editor.point(
                &mut fonts,
                [x as f32 * 0.5, (line.top + line.bottom) / 2.],
                false,
            );
            let (_, focus) = editor.selection_bytes();
            assert!(
                boundaries.contains(&focus),
                "point stopped at {focus}, inside a grapheme"
            );
            for code in [
                PhysicalKey::Down,
                PhysicalKey::Up,
                PhysicalKey::Home,
                PhysicalKey::End,
            ] {
                key(&mut editor, &mut fonts, code);
                let (_, focus) = editor.selection_bytes();
                assert!(
                    boundaries.contains(&focus),
                    "{code:?} stopped at {focus}, inside a grapheme"
                );
            }
        }
    }
}

#[test]
fn latin_ligatures_keep_every_user_character_navigable() {
    let source = "office ffi fl";
    let (mut fonts, mut editor) = fixture(false, source);
    editor.select_bytes(&mut fonts, 0, 0);
    for index in 1..=source.len() {
        key(&mut editor, &mut fonts, PhysicalKey::Right);
        assert_eq!(
            editor.selection_bytes(),
            (index, index),
            "ligature skipped a character"
        );
    }
    for index in (0..source.len()).rev() {
        key(&mut editor, &mut fonts, PhysicalKey::Left);
        assert_eq!(
            editor.selection_bytes(),
            (index, index),
            "ligature skipped a character"
        );
    }
}

#[test]
fn addressed_ime_callbacks_cannot_cross_editors_focus_epochs_or_document_replacement() {
    let (mut fonts, mut first) = fixture(false, "First");
    let (_, mut second) = fixture(false, "Second");
    let a = first.input_target();
    let b = second.input_target();
    assert_ne!(a, b);
    first.input(
        &mut fonts,
        &InputEvent::CompositionFor {
            target: a,
            text: "仮".into(),
            cursor: Some((0, 3)),
        },
    );
    assert_eq!(first.text(), "First");
    first.set_focused(&mut fonts, false);
    assert!(
        !second
            .input(
                &mut fonts,
                &InputEvent::TextFor {
                    target: a,
                    text: "late".into()
                }
            )
            .changed
    );
    assert_eq!(second.text(), "Second");
    first.set_focused(&mut fonts, true);
    assert_ne!(first.input_target(), a);
    assert!(
        !first
            .input(
                &mut fonts,
                &InputEvent::TextFor {
                    target: a,
                    text: "late".into()
                }
            )
            .changed
    );
    assert_eq!(first.text(), "First");
    let current = first.input_target();
    assert!(
        first
            .input(
                &mut fonts,
                &InputEvent::TextFor {
                    target: current,
                    text: "!".into()
                }
            )
            .changed
    );
    assert_eq!(first.text(), "First!");
    assert_eq!(
        first.input_target(),
        current,
        "ordinary committed edits retain their native session"
    );
    first.set_text(&mut fonts, "Replacement");
    assert_ne!(first.input_target(), current);
    assert!(
        !first
            .input(
                &mut fonts,
                &InputEvent::CompositionFor {
                    target: current,
                    text: "stale".into(),
                    cursor: None
                }
            )
            .input
            .consumed
    );
    assert_eq!(first.text(), "Replacement");
}

#[test]
fn menu_commands_supersede_pending_clipboard_and_share_keyboard_history() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    let Some(EditorClipboard::Write { cut: Some(cut), .. }) =
        editor.execute(&mut fonts, EditorCommand::Cut).clipboard
    else {
        panic!("cut");
    };
    assert!(matches!(
        editor.execute(&mut fonts, EditorCommand::Copy).clipboard,
        Some(EditorClipboard::Write { cut: None, .. })
    ));
    assert!(
        !editor
            .complete_clipboard(&mut fonts, cut, ClipboardResult::Written(true))
            .changed
    );
    assert_eq!(editor.text(), "A東京Z");
    let Some(EditorClipboard::Read { request }) =
        editor.execute(&mut fonts, EditorCommand::Paste).clipboard
    else {
        panic!("paste");
    };
    editor.execute(&mut fonts, EditorCommand::Copy);
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some("late".into()))
            )
            .changed
    );
    let target = editor.input_target();
    assert!(editor.replace_text(&mut fonts, "20"));
    assert_eq!(editor.input_target(), target);
    assert!(shortcut(&mut editor, &mut fonts, b'z').changed);
    assert_eq!(editor.text(), "A東京Z");
    assert_eq!(editor.selection_bytes(), (1, 7));
    assert!(editor.execute(&mut fonts, EditorCommand::Redo).changed);
    assert_eq!(editor.text(), "20");
}
#[test]
fn altgr_physical_letters_do_not_select_copy_or_paste_before_committed_text() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    editor.input(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            alt: true,
            ..Default::default()
        }),
    );
    for letter in [b'a', b'c', b'x', b'v', b'z'] {
        assert!(
            key(&mut editor, &mut fonts, PhysicalKey::Letter(letter))
                .clipboard
                .is_none()
        );
        assert_eq!(editor.selection_bytes(), (1, 7));
        assert_eq!(editor.text(), "A東京Z");
    }
    text(&mut editor, &mut fonts, "ą");
    assert_eq!(editor.text(), "AąZ");
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "A東京Z");
}

#[test]
fn gesture_edits_commit_one_undo_record_and_blur_cancels_without_destroying_redo() {
    let (mut fonts, mut editor) = fixture(false, "2");
    assert!(editor.begin_edit_group(&mut fonts));
    assert!(!editor.begin_edit_group(&mut fonts));
    for value in ["3", "4", "5"] {
        assert!(editor.replace_text(&mut fonts, value));
    }
    assert_eq!(editor.history_bytes(), 0);
    assert!(editor.commit_edit_group());
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "2");
    assert!(editor.begin_edit_group(&mut fonts));
    editor.replace_text(&mut fonts, "9");
    editor.set_focused(&mut fonts, false);
    assert_eq!(editor.text(), "2");
    editor.set_focused(&mut fonts, true);
    assert!(editor.redo(&mut fonts));
    assert_eq!(editor.text(), "5");
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "2");
    assert!(!editor.undo(&mut fonts), "drag samples form one undo unit");
}

#[test]
fn paint_only_syntax_colors_preserve_shaped_positions_bidi_carets_and_preedit_runs() {
    let (mut fonts, mut editor) =
        fixture(true, "office العربية e\u{301} 東京\nlet title = \"test\";");
    editor.select_bytes(&mut fonts, 2, 2);
    let caret = editor.caret(&mut fonts);
    let builds = editor.snapshots_built();
    let spans = vec![
        crate::text::TextStyleSpan {
            range: 0..6,
            color: [1., 0., 0., 1.],
        },
        crate::text::TextStyleSpan {
            range: 1..3,
            color: [0., 1., 0., 1.],
        },
        crate::text::TextStyleSpan {
            range: 7..21,
            color: [0., 0., 1., 1.],
        },
    ];
    for preedit in [false, true] {
        if preedit {
            editor.input(
                &mut fonts,
                &InputEvent::Composition {
                    text: "仮".into(),
                    cursor: None,
                },
            );
        }
        let layout = editor.layout(&mut fonts);
        let original = layout
            .runs
            .iter()
            .flat_map(|r| {
                r.glyphs
                    .iter()
                    .map(move |g| (r.font.identity, r.size, g.id, g.position, g.advance))
            })
            .collect::<Vec<_>>();
        let painted = layout.colorized_runs(&spans);
        let colored = painted
            .iter()
            .flat_map(|(r, _)| {
                r.glyphs
                    .iter()
                    .map(move |g| (r.font.identity, r.size, g.id, g.position, g.advance))
            })
            .collect::<Vec<_>>();
        assert_eq!(colored, original);
        assert!(
            painted
                .iter()
                .any(|(_, color)| *color == Some([0., 1., 0., 1.]))
        );
    }
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: String::new(),
            cursor: None,
        },
    );
    assert_eq!(editor.caret(&mut fonts), caret);
    let after = editor.snapshots_built();
    assert!(after > builds);
    editor.layout(&mut fonts).colorized_runs(&spans);
    assert_eq!(
        editor.snapshots_built(),
        after,
        "paint colors do not reshape the editor"
    );
}

#[test]
fn unclaimed_application_accelerators_do_not_edit_or_cancel_preedit() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "仮".into(),
            cursor: None,
        },
    );
    editor.input(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    let response = key(&mut editor, &mut fonts, PhysicalKey::Letter(b's'));
    assert!(!response.input.consumed && !response.changed);
    assert!(editor.is_composing());
    assert_eq!(editor.text(), "A東京Z");
}

#[test]
fn code_policy_newline_keeps_crlf_indent_and_one_undo_boundary() {
    let source = "    東京\r\nZ";
    let (mut fonts, mut editor) = fixture(true, source);
    editor.set_policy(
        &mut fonts,
        EditorPolicy {
            tab: EditorTab::Spaces(4),
            auto_indent: true,
            auto_brackets: true,
            ..Default::default()
        },
    );
    let cursor = "    東京".len();
    editor.select_bytes(&mut fonts, cursor, cursor);
    assert!(key(&mut editor, &mut fonts, PhysicalKey::Enter).changed);
    assert_eq!(editor.text(), "    東京\r\n    \r\nZ");
    assert_eq!(editor.selection_bytes(), (cursor + 6, cursor + 6));
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), source);
    assert_eq!(editor.selection_bytes(), (cursor, cursor));
    assert!(!editor.undo(&mut fonts));
}
#[test]
fn code_policy_indentation_keeps_unicode_backward_selection_and_excludes_terminal_line_start() {
    let source = "東京\ne\u{301}\n最後";
    let (mut fonts, mut editor) = fixture(true, source);
    editor.set_policy(
        &mut fonts,
        EditorPolicy {
            tab: EditorTab::Spaces(4),
            ..Default::default()
        },
    );
    let end = source.find("最後").unwrap();
    editor.select_bytes(&mut fonts, end, 0);
    assert!(key(&mut editor, &mut fonts, PhysicalKey::Tab).changed);
    assert_eq!(editor.text(), "    東京\n    e\u{301}\n最後");
    assert_eq!(editor.selection_bytes(), (end + 8, 0));
    editor.input(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            shift: true,
            ..Default::default()
        }),
    );
    assert!(key(&mut editor, &mut fonts, PhysicalKey::Tab).changed);
    assert_eq!(editor.text(), source);
    assert_eq!(editor.selection_bytes(), (end, 0));
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "    東京\n    e\u{301}\n最後");
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), source);
    editor.set_text(&mut fonts, "      x");
    editor.select_bytes(&mut fonts, 7, 7);
    key(&mut editor, &mut fonts, PhysicalKey::Tab);
    assert_eq!(editor.text(), "    x");
    assert_eq!(editor.selection_bytes(), (5, 5));
}
#[test]
fn code_brackets_wrap_the_selection_but_paste_and_ime_commit_remain_literal() {
    let source = "A東京e\u{301}Z";
    let (mut fonts, mut editor) = fixture(true, source);
    editor.set_policy(
        &mut fonts,
        EditorPolicy {
            auto_brackets: true,
            ..Default::default()
        },
    );
    let end = source.len() - 1;
    editor.select_bytes(&mut fonts, end, 1);
    assert!(text(&mut editor, &mut fonts, "(").changed);
    assert_eq!(editor.text(), "A(東京e\u{301})Z");
    assert_eq!(editor.selection_bytes(), (end + 1, 2));
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.selection_bytes(), (end, 1));
    let Some(EditorClipboard::Read { request }) =
        editor.execute(&mut fonts, EditorCommand::Paste).clipboard
    else {
        panic!("paste");
    };
    assert!(
        editor
            .complete_clipboard(&mut fonts, request, ClipboardResult::Text(Some("[".into())))
            .changed
    );
    assert_eq!(editor.text(), "A[Z");
    editor.undo(&mut fonts);
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "(".into(),
            cursor: None,
        },
    );
    assert!(text(&mut editor, &mut fonts, "(").changed);
    assert_eq!(editor.text(), "A(Z");
    editor.undo(&mut fonts);
    assert!(editor.replace_selection(&mut fonts, "{"));
    assert_eq!(editor.text(), "A{Z");
}
#[test]
fn readonly_policy_cancels_pending_input_but_retains_selection_copy_navigation_and_history() {
    let (mut fonts, mut editor) = fixture(true, "A東京Z");
    text(&mut editor, &mut fonts, "!");
    editor.execute(&mut fonts, EditorCommand::SelectAll);
    let Some(EditorClipboard::Read { request }) =
        editor.execute(&mut fonts, EditorCommand::Paste).clipboard
    else {
        panic!("paste");
    };
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "仮".into(),
            cursor: None,
        },
    );
    let old = editor.input_target();
    editor.set_policy(
        &mut fonts,
        EditorPolicy {
            read_only: true,
            ..Default::default()
        },
    );
    assert!(!editor.is_composing());
    assert!(!editor.accepts_text_input());
    assert_ne!(editor.input_target(), old);
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some("late".into()))
            )
            .changed
    );
    for physical in [
        PhysicalKey::Backspace,
        PhysicalKey::Delete,
        PhysicalKey::Enter,
    ] {
        assert!(!key(&mut editor, &mut fonts, physical).changed);
    }
    assert!(!text(&mut editor, &mut fonts, "stale").changed);
    assert!(!editor.replace_selection(&mut fonts, "stale"));
    assert!(!editor.undo(&mut fonts));
    assert!(!editor.begin_edit_group(&mut fonts));
    assert!(
        editor
            .execute(&mut fonts, EditorCommand::Paste)
            .clipboard
            .is_none()
    );
    assert!(
        matches!(editor.execute(&mut fonts,EditorCommand::Copy).clipboard,Some(EditorClipboard::Write { text,cut:None }) if text == "A東京Z!")
    );
    editor.set_policy(&mut fonts, EditorPolicy::default());
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "A東京Z");
}
#[test]
fn empty_clipboard_text_preserves_the_selection_and_committed_document() {
    let (mut fonts, mut editor) = fixture(false, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    let Some(EditorClipboard::Read { request }) =
        editor.execute(&mut fonts, EditorCommand::Paste).clipboard
    else {
        panic!("paste");
    };
    assert!(
        !editor
            .complete_clipboard(
                &mut fonts,
                request,
                ClipboardResult::Text(Some(String::new()))
            )
            .changed
    );
    assert_eq!(editor.text(), "A東京Z");
    assert_eq!(editor.selection_bytes(), (1, 7));
}

#[test]
fn saved_state_projection_keeps_unicode_crlf_schema_and_backward_selection_direction() {
    let source = "A東京\r\ne\u{301}Z";
    let (mut fonts, mut editor) = fixture(true, source);
    editor.select_bytes(&mut fonts, source.len() - 1, 1);
    let mut state = crate::text_edit::TextEditState::default();
    state.auto_indent = true;
    state.tab_spaces = 8;
    state.allow_select_blank = false;
    editor.export_state(&mut state);
    assert_eq!(state.to_text(), source);
    assert_eq!(state.rows, vec!["A東京\r", "e\u{301}Z"]);
    assert_eq!((state.cursor.row, state.cursor.column), (0, 1));
    assert_eq!((state.selection.start, state.selection.end), (1, 6));
    assert!(state.auto_indent && !state.allow_select_blank && state.tab_spaces == 8);
    let encoded = state.save();
    let restored = crate::text_edit::TextEditState::load(&encoded);
    let old_target = editor.input_target();
    editor.import_state(&mut fonts, &restored);
    assert_ne!(editor.input_target(), old_target);
    assert_eq!(editor.text(), source);
    assert_eq!(editor.selection_bytes(), (source.len() - 1, 1));
    assert!(!editor.undo(&mut fonts));
    let mut roundtrip = restored.clone();
    editor.export_state(&mut roundtrip);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&roundtrip.save()).unwrap(),
        serde_json::from_str::<serde_json::Value>(&encoded).unwrap()
    );
}
#[test]
fn saved_state_projection_uses_committed_ime_state_and_bounds_malformed_positions() {
    let (mut fonts, mut editor) = fixture(true, "A東京Z");
    editor.select_bytes(&mut fonts, 1, 7);
    editor.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "仮".into(),
            cursor: None,
        },
    );
    let mut state = crate::text_edit::TextEditState::default();
    editor.export_state(&mut state);
    assert_eq!(state.to_text(), "A東京Z");
    assert_eq!((state.selection.start, state.selection.end), (1, 3));
    state.cursor.row = usize::MAX;
    state.cursor.column = usize::MAX;
    state.selection.start = usize::MAX;
    state.selection.end = usize::MAX;
    editor.import_state(&mut fonts, &state);
    assert_eq!(editor.selection_bytes(), (8, 8));
    state.rows.clear();
    editor.import_state(&mut fonts, &state);
    assert_eq!(editor.text(), "");
    assert_eq!(editor.selection_bytes(), (0, 0));
}

#[test]
fn external_undo_policy_leaves_document_transactions_and_shortcuts_with_the_application() {
    let (mut fonts, mut editor) = fixture(true, "A東京Z");
    editor.set_policy(
        &mut fonts,
        EditorPolicy {
            undo: EditorUndo::External,
            ..Default::default()
        },
    );
    assert!(text(&mut editor, &mut fonts, "!").changed);
    assert_eq!(editor.history_bytes(), 0);
    let undo = shortcut(&mut editor, &mut fonts, b'z');
    assert!(!undo.input.consumed && !undo.changed);
    assert_eq!(editor.text(), "A東京Z!");
    assert!(!editor.undo(&mut fonts));
    editor.execute(&mut fonts, EditorCommand::SelectAll);
    assert!(
        matches!(editor.execute(&mut fonts,EditorCommand::Copy).clipboard,Some(EditorClipboard::Write { text,.. }) if text == "A東京Z!")
    );
    editor.select_bytes(&mut fonts, editor.text().len(), editor.text().len());
    editor.set_policy(&mut fonts, EditorPolicy::default());
    assert!(text(&mut editor, &mut fonts, "?").changed);
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "A東京Z!");
    assert!(!editor.undo(&mut fonts));
}
