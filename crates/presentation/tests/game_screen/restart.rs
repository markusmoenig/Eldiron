use super::*;
use eldiron_presentation::ScreenTextTarget;
use rusterix::{
    ServerState,
    client::widget::messages::MessageLine,
    world_text::{TextAnchor, WorldText},
};
use theframework::platform::*;
use theframework::text_edit::{ClipboardResult, EditorClipboard};

fn next_session(previous: &GameSession) -> GameSession {
    let mut next = GameSession::new(previous.project.clone());
    next.current_map = previous.current_map.clone();
    next.server.state = previous.server.state;
    next.assets.config = next.project.config.clone();
    assert_ne!(next.view().session_id, previous.view().session_id);
    assert_eq!(next.view().map.unwrap().id, previous.view().map.unwrap().id);
    next
}
fn key(physical: PhysicalKey) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    }
}
fn choice() -> MessageLine {
    MessageLine {
        id: Uuid::new_v4(),
        text: "1) Old question".into(),
        rect: Rect::empty(),
        choice: Some(rusterix::Choice::ScriptChoice(
            "Old question".into(),
            "pick".into(),
            1,
            1,
            0,
            i64::MAX,
            10.,
        )),
        choice_key: Some('1'),
        color: [255; 4],
        sender_entity: None,
    }
}
#[test]
fn restart_before_frame_revokes_old_choices_ime_and_clipboard_without_erasing_bindings() {
    let (session, mut fonts, mut screen) = command_screen();
    screen.focus_input(&mut fonts, Some(3));
    let old_target = screen.text_input_state(&mut fonts).unwrap().target;
    screen.text_input_event(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    let paste = screen.text_input_event(&mut fonts, &key(PhysicalKey::Letter(b'v')));
    let Some(EditorClipboard::Read { request }) = paste.editor.clipboard else {
        panic!("clipboard request")
    };
    screen.text_input_event(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
    let row = choice();
    let widget = &mut screen.elements.messages_widgets[0];
    widget.messages.push(row.clone());
    widget.active_choices.push(row);
    widget.command_text = "old pending command".into();
    let next = next_session(&session);
    // Input can precede receive/prepare. No old dialog may address the new game.
    let response = screen
        .input(&next.view(), &mut fonts, &key(PhysicalKey::Digit(1)))
        .unwrap();
    assert!(
        response.effects.is_empty(),
        "old choices cannot become new-session actions"
    );
    assert!(
        screen.elements.messages_widgets[0]
            .active_choices
            .is_empty()
    );
    assert!(screen.elements.messages_widgets[0].messages.is_empty());
    assert!(screen.elements.messages_widgets[0].command_text.is_empty());
    assert!(screen.text_input_state(&mut fonts).is_none());
    screen
        .prepare(&next.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    screen.focus_input(&mut fonts, Some(3));
    assert_ne!(
        screen.text_input_state(&mut fonts).unwrap().target,
        old_target
    );
    assert!(
        !screen
            .input(
                &next.view(),
                &mut fonts,
                &InputEvent::TextFor {
                    target: old_target,
                    text: "late IME".into()
                }
            )
            .unwrap()
            .input
            .consumed
    );
    assert!(
        !screen
            .complete_editor_clipboard(
                &mut fonts,
                ScreenTextTarget::Input(3),
                request,
                ClipboardResult::Text(Some("late clipboard".into()))
            )
            .editor
            .changed
    );
    assert_eq!(screen.elements.text_input_widgets[&3].text, "Éowyn");
    assert!(
        screen.elements.messages_widgets[0].command_input,
        "authored command behavior survives"
    );
}

#[test]
fn restart_receive_keeps_only_fresh_transcript_speech_and_private_world_text() {
    let mut session = fixture(false);
    session.server.state = ServerState::Running;
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let events = |text: &str| RegionEvents {
        messages: vec![(None, None, 1, text.into(), "status".into())],
        says: vec![(Some(1), None, text.into(), "status".into())],
        world_text: vec![(
            1,
            WorldText {
                anchor: TextAnchor::Entity(1),
                text: text.into(),
            },
        )],
        ..Default::default()
    };
    screen.receive(&session.view(), &events("Old"));
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    assert_eq!(screen.speech.messages().len(), 1);
    assert_eq!(screen.world_texts.passages().len(), 1);
    let next = next_session(&session);
    screen.receive(
        &next.view(),
        &RegionEvents {
            messages: events("Fresh").messages,
            ..Default::default()
        },
    );
    // The first new frame must not clear the already-received new batch.
    screen
        .prepare(&next.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let lines = &screen.elements.messages_widgets[0].messages;
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Fresh");
    assert!(screen.speech.messages().is_empty());
    assert!(screen.world_texts.passages().is_empty());
    screen.receive(&next.view(), &events("New overlays"));
    screen
        .prepare(&next.view(), &mut fonts, ContentRevision(1), 1)
        .unwrap();
    assert_eq!(screen.speech.messages()[0].text.as_ref(), "New overlays");
    assert_eq!(
        screen.world_texts.passages()[0].message.text,
        "New overlays"
    );
    let speech_id = screen.speech.messages()[0].id;
    let passage_id = screen.world_texts.passages()[0].id;
    screen.invalidate_world_input(); // GPU recreation is not a new runtime.
    screen
        .prepare(&next.view(), &mut fonts, ContentRevision(1), 2)
        .unwrap();
    assert_eq!(screen.speech.messages()[0].id, speech_id);
    assert_eq!(screen.world_texts.passages()[0].id, passage_id);
}

#[test]
fn transplanted_overlays_bind_runtime_before_new_screen_receive() {
    let mut session = fixture(false);
    session.server.state = ServerState::Running;
    let mut old = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    old.receive(
        &session.view(),
        &RegionEvents {
            says: vec![(Some(1), None, "Old speech".into(), "status".into())],
            world_text: vec![(
                1,
                WorldText {
                    anchor: TextAnchor::Entity(1),
                    text: "Old description".into(),
                },
            )],
            ..Default::default()
        },
    );
    let next = next_session(&session);
    let mut screen = GameScreenController::from_screen("Main", &next.view(), layout()).unwrap();
    screen.speech = old.speech;
    screen.world_texts = old.world_texts;
    screen.receive(&next.view(), &RegionEvents::default());
    assert!(screen.speech.messages().is_empty());
    assert!(screen.world_texts.passages().is_empty());
}

#[test]
fn font_publication_checks_new_runtime_even_with_identical_content_revision() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let version = fonts.refresh(&session.view(), ContentRevision(1));
    let mut next = next_session(&session);
    assert_eq!(
        fonts.refresh(&next.view(), ContentRevision(1)),
        version,
        "identical font bytes retain glyph identities"
    );
    let font = next
        .project
        .assets
        .values_mut()
        .find(|a| a.name == "Roboto")
        .unwrap();
    font.buffer = shared::asset::AssetBuffer::Font(
        include_bytes!("../../../theframework/embedded/fonts/Roboto-Bold.ttf").to_vec(),
    );
    let changed = next_session(&next);
    assert_ne!(fonts.refresh(&changed.view(), ContentRevision(1)), version);
    let shaped = fonts.text_mut().layout("Font", "Roboto", 18., None);
    assert_eq!(
        &*shaped.runs[0].font.data,
        include_bytes!("../../../theframework/embedded/fonts/Roboto-Bold.ttf")
    );
}
