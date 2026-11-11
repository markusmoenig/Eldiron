use super::*;
use eldiron_presentation::{ScreenCapture, ScreenEffect, ScreenFocus, ScreenTextTarget};
use rusterix::{
    EntityAction,
    client::widget::choice::{ChoiceWidget, ChoiceWidgetKind},
};
use theframework::platform::*;
fn key(physical: PhysicalKey, pressed: bool, repeat: bool) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed,
        repeat,
    }
}
fn choice(screen: &mut GameScreenController, id: u32, kind: ChoiceWidgetKind) {
    screen.elements.choice_widgets.insert(
        id,
        ChoiceWidget::from_ui(
            format!("Choice{id}"),
            id,
            Rect::new(16., 50., 180., 30.),
            kind,
            &format!("bind='choice{id}'\noptions=['Warrior','Mage','Rogue']")
                .parse()
                .unwrap(),
        )
        .unwrap(),
    );
    screen
        .elements
        .set_binding(&format!("choice{id}"), "warrior");
}
#[test]
fn tab_traverses_all_control_families_in_stable_order_and_skips_ineligible_controls() {
    let (session, mut fonts, mut screen) = command_screen();
    choice(&mut screen, 30, ChoiceWidgetKind::Dropdown);
    choice(&mut screen, 40, ChoiceWidgetKind::TabBar);
    let mut disabled = rusterix::client::widget::Widget::new();
    disabled.rect = Rect::new(250., 50., 80., 32.);
    disabled.id = 6;
    disabled.name = "Unavailable".into();
    disabled.command = Some("rules.unknown_action".into());
    screen.elements.button_widgets.insert(6, disabled);
    let mut outside = rusterix::client::widget::Widget::new();
    outside.rect = Rect::new(500., 50., 80., 32.);
    outside.id = 21;
    outside.rect.x = 500.;
    screen.elements.button_widgets.insert(21, outside);
    choice(&mut screen, 70, ChoiceWidgetKind::Dropdown);
    screen.elements.widgets_to_hide.push("Choice70".into());
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let targets = vec![
        ScreenFocus::Button(2),
        ScreenFocus::Text(ScreenTextTarget::Input(3)),
        ScreenFocus::Text(ScreenTextTarget::Command(4)),
        ScreenFocus::Choice(30),
        ScreenFocus::Choice(40),
    ];
    assert_eq!(screen.focus_targets(&session.view()), targets);
    for target in &targets {
        assert!(
            screen
                .input(
                    &session.view(),
                    &mut fonts,
                    &key(PhysicalKey::Tab, true, false)
                )
                .unwrap()
                .input
                .consumed
        );
        assert_eq!(screen.focused_control(), Some(*target));
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Tab, false, false),
            )
            .unwrap();
    }
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_control(), Some(targets[0]));
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::Modifiers(Modifiers {
                shift: true,
                ..Default::default()
            }),
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_control(), Some(targets[4]));
    // Hiding a focused control cancels its identity before the next event.
    screen.elements.widgets_to_hide.push("Choice40".into());
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    assert_eq!(screen.focused_control(), None);
}
#[test]
fn gameplay_tab_binding_is_available_until_ui_focus_and_preedit_owns_navigation() {
    let session = fixture(true);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    assert!(
        !screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Tab, true, false)
            )
            .unwrap()
            .input
            .consumed
    );
    screen.focus_input(&mut fonts, Some(3));
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::Composition {
                text: "仮".into(),
                cursor: Some((0, 3)),
            },
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_input(), Some(3));
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Escape, true, false),
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_control(), Some(ScreenFocus::Button(2)));
}
#[test]
fn button_activation_commits_one_matching_release_and_focus_change_consumes_old_release() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    screen.focus_control(&mut fonts, Some(ScreenFocus::Button(2)));
    for event in [
        key(PhysicalKey::Enter, true, false),
        key(PhysicalKey::Enter, true, true),
        key(PhysicalKey::Space, false, false),
    ] {
        assert!(
            screen
                .input(&session.view(), &mut fonts, &event)
                .unwrap()
                .effects
                .is_empty()
        );
    }
    assert_eq!(
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Enter, false, false)
            )
            .unwrap()
            .effects
            .len(),
        1
    );
    assert!(
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Enter, false, false)
            )
            .unwrap()
            .effects
            .is_empty()
    );
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, true, false),
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_input(), Some(3));
    let release = screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, false, false),
        )
        .unwrap();
    assert!(release.input.consumed && release.effects.is_empty());
    assert_eq!(
        screen.focused_input(),
        Some(3),
        "old Enter release cannot submit the newly focused editor"
    );
}
#[test]
fn held_keyboard_controls_stop_on_release_and_pointer_takeover_keeps_its_own_release() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen.elements.button_widgets.get_mut(&2).unwrap().command = Some("control.forward".into());
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    screen.focus_control(&mut fonts, Some(ScreenFocus::Button(2)));
    let down = screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Space, true, false),
        )
        .unwrap();
    assert_eq!(
        down.effects,
        vec![ScreenEffect::Action(EntityAction::Forward)]
    );
    assert!(down.cancel_controls);
    assert!(
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Space, true, true)
            )
            .unwrap()
            .effects
            .is_empty()
    );
    let release = screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Space, false, false),
        )
        .unwrap();
    assert_eq!(
        release.effects,
        vec![ScreenEffect::Action(EntityAction::Off)]
    );
    assert!(!release.cancel_controls, "release emits Off exactly once");
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Space, true, false),
        )
        .unwrap();
    let pointer = |pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed,
        position: [30., 200.],
    };
    let takeover = screen
        .input(&session.view(), &mut fonts, &pointer(true))
        .unwrap();
    assert!(takeover.cancel_controls);
    assert_eq!(
        takeover.effects,
        vec![ScreenEffect::Action(EntityAction::Forward)]
    );
    assert!(
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Space, false, false)
            )
            .unwrap()
            .effects
            .is_empty()
    );
    assert_eq!(screen.pressed_button, Some(2));
    assert_eq!(screen.control_capture(), Some(ScreenCapture::Button(2)));
    assert_eq!(
        screen
            .input(&session.view(), &mut fonts, &pointer(false))
            .unwrap()
            .effects,
        vec![ScreenEffect::Action(EntityAction::Off)]
    );
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Space, true, false),
        )
        .unwrap();
    screen.elements.widgets_to_hide.push("Widget2".into());
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    assert_eq!(screen.focused_control(), None);
    assert_eq!(screen.control_capture(), Some(ScreenCapture::Cancelled));
    assert!(
        screen
            .input(
                &session.view(),
                &mut fonts,
                &key(PhysicalKey::Space, false, false)
            )
            .unwrap()
            .effects
            .is_empty()
    );
    assert_eq!(screen.control_capture(), None);
}
#[test]
fn keyboard_choices_keep_dropdown_pending_but_commit_tabs_and_tab_cancels_popup() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    choice(&mut screen, 30, ChoiceWidgetKind::Dropdown);
    choice(&mut screen, 40, ChoiceWidgetKind::TabBar);
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    screen.focus_control(&mut fonts, Some(ScreenFocus::Choice(30)));
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Down, true, false),
        )
        .unwrap();
    assert_eq!(screen.elements.choice_widgets[&30].selected, 1);
    assert_eq!(screen.elements.ui_state["choice30"], "warrior");
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Tab, true, false),
        )
        .unwrap();
    assert_eq!(screen.focused_control(), Some(ScreenFocus::Choice(40)));
    assert_eq!(screen.choice_controller.open_id(), None);
    assert_eq!(screen.elements.choice_widgets[&30].selected, 0);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::End, true, false),
        )
        .unwrap();
    assert_eq!(screen.elements.ui_state["choice40"], "rogue");
    screen.focus_control(&mut fonts, Some(ScreenFocus::Choice(30)));
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, true, false),
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Down, true, false),
        )
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, true, false),
        )
        .unwrap();
    assert_eq!(screen.elements.ui_state["choice30"], "mage");
    screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, true, true),
        )
        .unwrap();
    assert_eq!(screen.choice_controller.open_id(), None);
}
#[test]
fn blocking_transcript_preempts_keyboard_button_and_root_clip_prevents_invisible_clicks() {
    let (session, mut fonts, mut screen) = command_screen();
    screen.focus_control(&mut fonts, Some(ScreenFocus::Button(2)));
    screen.receive(
        &session.view(),
        &RegionEvents {
            messages: vec![(None, None, 1, "".into(), "continue".into())],
            ..Default::default()
        },
    );
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let response = screen
        .input(
            &session.view(),
            &mut fonts,
            &key(PhysicalKey::Enter, true, false),
        )
        .unwrap();
    assert!(response.input.consumed && response.effects.is_empty());
    assert_eq!(screen.pressed_button, None);
    assert!(!screen.elements.messages_widgets[0].blocks_input());
    screen.elements.button_widgets.get_mut(&2).unwrap().rect = Rect::new(-50., 180., 100., 50.);
    let pointer = |position, pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed,
        position,
    };
    assert!(
        !screen
            .input(&session.view(), &mut fonts, &pointer([-20., 200.], true))
            .unwrap()
            .input
            .consumed
    );
    screen
        .input(&session.view(), &mut fonts, &pointer([20., 200.], true))
        .unwrap();
    assert!(
        screen
            .input(&session.view(), &mut fonts, &pointer([-20., 200.], false))
            .unwrap()
            .effects
            .is_empty()
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn focus_chrome_is_gpu_paint_and_reuses_glyphs_without_source_images() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    choice(&mut screen, 30, ChoiceWidgetKind::Dropdown);
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut cache = ScreenImageCache::default();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(800, 480)).unwrap();
    let mut pixels = Vec::new();
    let mut steady = None;
    for focus in [
        None,
        Some(ScreenFocus::Button(2)),
        Some(ScreenFocus::Choice(30)),
    ] {
        screen.focus_control(&mut fonts, focus);
        screen
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
        let list = {
            let mut images = cache.begin_frame(&mut registry).unwrap();
            screen
                .paint(&session.view(), &mut fonts, &mut images)
                .unwrap()
        };
        assert!(
            !list
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::Image { .. } | PaintOp::FilteredImage { .. }))
        );
        renderer
            .render(
                &list,
                &registry,
                &target.view,
                target.descriptor.format,
                (800, 480),
                2.,
                [0.; 4],
            )
            .unwrap();
        let stats = (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
        );
        if let Some(old) = steady {
            assert_eq!(
                stats, old,
                "focus changes cannot rasterize glyphs or upload screen bitmaps"
            );
        }
        steady = Some(stats);
        pixels.push(capture(&gpu, target.clone()));
    }
    let at =
        |frame: usize, x: usize, y: usize| &pixels[frame][(y * 800 + x) * 4..(y * 800 + x) * 4 + 4];
    assert_ne!(
        at(0, 60, 355),
        at(1, 60, 355),
        "button focus border is visible on the actual GPU target"
    );
    assert_ne!(
        at(1, 60, 103),
        at(2, 60, 103),
        "choice focus border is visible on the actual GPU target"
    );
    assert_eq!(
        gpu.transfers().capture_requests,
        3,
        "only explicit verification captures read back color"
    );
}
