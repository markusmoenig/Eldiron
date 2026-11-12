use super::*;
use rusterix::client::widget::game::GameWidget;

fn stage_look(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    for key in screen
        .elements
        .game_widgets
        .keys()
        .copied()
        .collect::<Vec<_>>()
    {
        let output = ViewImage {
            view: ViewId::new(),
            image: renderkit::ImageHandle {
                owner: 1,
                slot: 0,
                generation: 1,
            },
            descriptor: renderkit::ImageDescriptor::rgba(64, 48),
            revision: 1,
            content_revision: 1,
        };
        screen
            .world
            .push_view(
                key,
                &screen.elements.game_widgets[&key],
                output,
                session.view().map.unwrap(),
            )
            .unwrap();
    }
}
fn motion(
    session: &GameSession,
    screen: &mut GameScreenController,
    delta: [f32; 2],
) -> ScreenInputResponse {
    send(session, screen, InputEvent::RelativeMotion { delta })
}
fn setup_look() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, key) = setup();
    session.project.regions[0].map.entities[0].set_attribute("player", Value::Bool(true));
    screen.elements.game_widgets.get_mut(&key).unwrap().camera = PlayerCamera::D3FirstPMouse;
    (session, screen, fonts, key)
}
#[test]
fn relative_look_needs_an_acknowledged_view_but_does_not_need_a_geometry_hit() {
    let (session, mut screen, mut fonts, key) = setup_look();
    stage_look(&session, &mut screen, &mut fonts);
    move_to(&session, &mut screen, [24., 24.]);
    assert!(
        motion(&session, &mut screen, [10., -10.])
            .effects
            .is_empty()
    );
    assert!(screen.mouse_look_view().is_none());
    ack(&session, &mut screen);
    assert_eq!(screen.mouse_look_view(), Some(key));
    assert!(
        screen.world_pointer_hit().is_none(),
        "synthetic receipt has no query service"
    );
    let result = motion(&session, &mut screen, [10., -10.]);
    assert_eq!(
        result.effects,
        vec![ScreenEffect::Action(EntityAction::LookYaw(2.))]
    );
    assert!(result.input.consumed && result.input.redraw);
    assert_eq!(screen.elements.game_widgets[&key].mouse_look_pitch, 2.);
    assert!(
        motion(&session, &mut screen, [0., -10000.])
            .effects
            .is_empty()
    );
    assert_eq!(screen.elements.game_widgets[&key].mouse_look_pitch, 85.);
    assert!(!motion(&session, &mut screen, [f32::NAN, 0.]).input.redraw);
    assert_eq!(screen.elements.game_widgets[&key].mouse_look_pitch, 85.);
    assert!(!motion(&session, &mut screen, [0., 0.]).input.redraw);
}
#[test]
fn relative_look_is_per_view_and_live_settings_wait_for_their_own_acknowledgement() {
    let (session, mut screen, mut fonts, first) = setup_look();
    screen
        .elements
        .game_widgets
        .get_mut(&first)
        .unwrap()
        .rect
        .width = 64.;
    let second = Uuid::new_v4();
    let mut widget = GameWidget::new();
    widget.camera = PlayerCamera::D3FirstPMouse;
    widget.name = "Second".into();
    widget.rect = Rect::new(64., 0., 64., 96.);
    widget.mouse_invert_y = true;
    widget.mouse_sensitivity = 1.;
    screen.elements.game_widgets.insert(second, widget);
    stage_look(&session, &mut screen, &mut fonts);
    ack(&session, &mut screen);
    move_to(&session, &mut screen, [24., 24.]);
    motion(&session, &mut screen, [10., -10.]);
    move_to(&session, &mut screen, [90., 24.]);
    assert_eq!(screen.mouse_look_view(), Some(second));
    assert_eq!(
        motion(&session, &mut screen, [10., -10.]).effects,
        vec![ScreenEffect::Action(EntityAction::LookYaw(10.))]
    );
    assert_eq!(screen.elements.game_widgets[&first].mouse_look_pitch, 2.);
    assert_eq!(screen.elements.game_widgets[&second].mouse_look_pitch, -10.);
    screen
        .elements
        .game_widgets
        .get_mut(&second)
        .unwrap()
        .mouse_sensitivity = 2.;
    assert!(
        motion(&session, &mut screen, [10., -10.])
            .effects
            .is_empty()
    );
    stage_look(&session, &mut screen, &mut fonts);
    assert!(
        motion(&session, &mut screen, [10., -10.])
            .effects
            .is_empty()
    );
    ack(&session, &mut screen);
    assert_eq!(
        motion(&session, &mut screen, [10., -10.]).effects,
        vec![ScreenEffect::Action(EntityAction::LookYaw(20.))]
    );
    assert_eq!(screen.elements.game_widgets[&second].mouse_look_pitch, -30.);
    move_to(&session, &mut screen, [24., 24.]);
    motion(&session, &mut screen, [0., 10.]);
    assert_eq!(screen.elements.game_widgets[&first].mouse_look_pitch, 0.);
}
#[test]
fn relative_look_revokes_on_modal_focus_leave_escape_and_source_visibility_changes() {
    let (mut session, mut screen, mut fonts, key) = setup_look();
    stage_look(&session, &mut screen, &mut fonts);
    ack(&session, &mut screen);
    move_to(&session, &mut screen, [24., 24.]);
    assert!(screen.mouse_look_view().is_some());
    let blocked = screen.world_input(
        &session.view(),
        None,
        &InputEvent::RelativeMotion { delta: [10., 0.] },
        true,
    );
    assert!(blocked.effects.is_empty());
    assert!(screen.mouse_look_view().is_none());
    move_to(&session, &mut screen, [24., 24.]);
    send(
        &session,
        &mut screen,
        InputEvent::Key {
            physical: PhysicalKey::Escape,
            logical: theframework::platform::LogicalKey::Other,
            pressed: true,
            repeat: false,
        },
    );
    assert!(motion(&session, &mut screen, [10., 0.]).effects.is_empty());
    move_to(&session, &mut screen, [24., 24.]);
    send(&session, &mut screen, InputEvent::PointerLeft);
    assert!(motion(&session, &mut screen, [10., 0.]).effects.is_empty());
    move_to(&session, &mut screen, [24., 24.]);
    screen
        .elements
        .widgets_to_hide
        .push(screen.elements.game_widgets[&key].name.clone());
    assert!(motion(&session, &mut screen, [10., 0.]).effects.is_empty());
    screen.elements.widgets_to_hide.clear();
    move_to(&session, &mut screen, [24., 24.]);
    send(&session, &mut screen, InputEvent::Focus(false));
    assert!(motion(&session, &mut screen, [10., 0.]).effects.is_empty());
    send(&session, &mut screen, InputEvent::Focus(true));
    move_to(&session, &mut screen, [24., 24.]);
    assert!(
        motion(&session, &mut screen, [10., 0.]).effects.is_empty(),
        "focus requires a fresh frame"
    );
    stage_look(&session, &mut screen, &mut fonts);
    ack(&session, &mut screen);
    assert!(!motion(&session, &mut screen, [10., 0.]).effects.is_empty());
    session.project.regions[0].map.id = Uuid::new_v4();
    assert!(motion(&session, &mut screen, [10., 0.]).effects.is_empty());
}
