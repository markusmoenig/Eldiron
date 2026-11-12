use super::hud::dispatch;
use super::*;
use crate::game_screen::{HudPanelId, HudPart};
use rusterix::client::{
    command::ClientCommandBinding,
    screen_model::{PreferenceSetting, TooltipDelay},
};
use theframework::platform::LogicalKey;
fn fixture() -> (GameSession, GameScreenController, GameFonts) {
    let (mut session, mut screen, fonts, _) = setup();
    session.server.state = ServerState::Off;
    session.current_map = "No selected world".into();
    screen.resize((640, 480));
    screen.elements.game_widgets.clear();
    toggle(&session, &mut screen, "preferences");
    (session, screen, fonts)
}
fn toggle(session: &GameSession, screen: &mut GameScreenController, name: &str) {
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui(name.into()))
    ));
}
fn ready(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    screen.finish_world_frame(&session.view());
    ack(session, screen);
}
fn frame(
    session: &GameSession,
    screen: &GameScreenController,
) -> crate::game_screen::hud::model::PanelFrame {
    screen
        .hud
        .frames(&session.view(), &screen.elements)
        .into_iter()
        .find(|f| matches!(f.id, HudPanelId::Preferences { .. }))
        .unwrap()
}
fn point(session: &GameSession, screen: &GameScreenController, part: HudPart) -> [f32; 2] {
    let bounds = frame(session, screen)
        .controls()
        .into_iter()
        .find(|(id, _)| id.part == part)
        .unwrap()
        .1;
    [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.]
}
fn click(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    part: HudPart,
) {
    let p = point(session, screen, part);
    assert!(
        dispatch(session, screen, fonts, button(p, true))
            .effects
            .is_empty()
    );
    assert!(
        dispatch(session, screen, fonts, button(p, false))
            .effects
            .is_empty()
    );
}
fn key(physical: PhysicalKey, pressed: bool) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed,
        repeat: false,
    }
}
#[test]
fn preferences_on_a_worldless_stopped_menu_require_ack_and_publish_bindings_without_gameplay() {
    let (session, mut screen, mut fonts) = fixture();
    let time = session.project.time;
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::Tooltips),
    );
    assert!(!screen.hud.popup_open());
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::Tooltips),
    );
    assert!(screen.hud.popup_open());
    // A newly opened option cannot consume the previous frame's acknowledgement.
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::PreferenceOption(PreferenceSetting::Tooltips, 1),
    );
    assert!(screen.ui_preferences().tooltips_enabled);
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::PreferenceOption(PreferenceSetting::Tooltips, 1),
    );
    assert!(!screen.ui_preferences().tooltips_enabled);
    assert_eq!(screen.elements.ui_state["preferences.tooltips"], "off");
    assert!(!screen.hud.popup_open());
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::TooltipDelay),
    );
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::PreferenceOption(PreferenceSetting::TooltipDelay, 1),
    );
    assert_eq!(screen.ui_preferences().tooltip_delay, TooltipDelay::Short);
    assert_eq!(
        screen.elements.ui_state["preferences.tooltip_delay"],
        "short"
    );
    assert!(session.server.state == ServerState::Off);
    assert!(session.project.time == time);
    assert!(session.view().map.is_none());
}
#[test]
fn preference_option_receipts_reject_focus_reopen_geometry_and_session_changes() {
    for change in 0..4 {
        let (mut session, mut screen, mut fonts) = fixture();
        ready(&session, &mut screen, &mut fonts);
        click(
            &session,
            &mut screen,
            &mut fonts,
            HudPart::Preference(PreferenceSetting::Tooltips),
        );
        ready(&session, &mut screen, &mut fonts);
        let p = point(
            &session,
            &screen,
            HudPart::PreferenceOption(PreferenceSetting::Tooltips, 1),
        );
        dispatch(&session, &mut screen, &mut fonts, button(p, true));
        match change {
            0 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
            }
            1 => {
                toggle(&session, &mut screen, "preferences");
                toggle(&session, &mut screen, "preferences");
            }
            2 => screen.elements.toolbar_preferences_panel_config.width += 20.,
            _ => {
                session = GameSession::new(session.project.clone());
            }
        }
        assert!(
            dispatch(&session, &mut screen, &mut fonts, button(p, false))
                .effects
                .is_empty()
        );
        assert!(screen.ui_preferences().tooltips_enabled, "change {change}");
    }
}
#[test]
fn preference_popups_share_keyboard_selection_escape_and_outside_press_ownership() {
    let (session, mut screen, mut fonts) = fixture();
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::TooltipDelay),
    );
    ready(&session, &mut screen, &mut fonts);
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Home, true),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, false),
    );
    assert_eq!(screen.ui_preferences().tooltip_delay, TooltipDelay::Instant);
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::TooltipDelay),
    );
    ready(&session, &mut screen, &mut fonts);
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Escape, true),
    );
    assert!(!screen.hud.popup_open());
    assert!(
        matches!(screen.control_capture(),Some(ScreenCapture::Hud(id)) if id.part == HudPart::Preference(PreferenceSetting::TooltipDelay))
    );
    ready(&session, &mut screen, &mut fonts);
    click(
        &session,
        &mut screen,
        &mut fonts,
        HudPart::Preference(PreferenceSetting::Tooltips),
    );
    ready(&session, &mut screen, &mut fonts);
    let mut target = Widget::new();
    target.id = 44;
    target.name = "Covered action".into();
    target.rect = Rect::new(500., 300., 40., 40.);
    target.command = Some("game.start".into());
    screen.elements.button_widgets.insert(44, target);
    ready(&session, &mut screen, &mut fonts);
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([520., 320.], true)
        )
        .effects
        .is_empty()
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([520., 320.], false)
        )
        .effects
        .is_empty()
    );
    assert!(!screen.hud.popup_open());
}
#[test]
fn native_panel_positions_survive_family_switches_and_reset_to_authored_defaults() {
    let (session, mut screen, mut fonts, _) = setup();
    screen.resize((640, 480));
    let c = rusterix::client::screen_model::InventoryPanelConfig {
        authored_rect: Some(Rect::new(40., 40., 360., 200.)),
        ..Default::default()
    };
    screen.elements.custom_inventory_panel_config = Some(c);
    toggle(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts);
    let f = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let title = f.title_rect.unwrap();
    let p = [title.x + 10., title.y + title.height / 2.];
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [p[0] + 50., p[1] + 30.],
        },
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([p[0] + 50., p[1] + 30.], false),
    );
    ready(&session, &mut screen, &mut fonts);
    let moved = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap()
        .rect;
    assert_eq!([moved.x, moved.y], [90., 70.]);
    toggle(&session, &mut screen, "preferences");
    ready(&session, &mut screen, &mut fonts);
    toggle(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts);
    assert_eq!(
        screen
            .hud
            .frames(&session.view(), &screen.elements)
            .pop()
            .unwrap()
            .rect,
        moved
    );
    toggle(&session, &mut screen, "preferences");
    ready(&session, &mut screen, &mut fonts);
    click(&session, &mut screen, &mut fonts, HudPart::ResetPositions);
    toggle(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts);
    let reset = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap()
        .rect;
    assert_eq!([reset.x, reset.y], [40., 40.]);
}
#[test]
fn stopped_menu_receipts_never_enable_world_or_owned_item_actions_and_reject_start_transition() {
    let (mut session, mut screen, mut fonts) = fixture();
    ready(&session, &mut screen, &mut fonts);
    let p = point(
        &session,
        &screen,
        HudPart::Preference(PreferenceSetting::Tooltips),
    );
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    session.server.state = ServerState::Running;
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(p, false))
            .effects
            .is_empty()
    );
    assert!(!screen.hud.popup_open());
    assert!(!screen.ui_frame_matches(&session.view()));
    assert!(matches!(
        screen.query_world(&session.view(), None, [600., 400.], false),
        Query::Unavailable
    ));
}

#[test]
fn constrained_preference_controls_remain_clipped_and_never_allocate_world_or_panel_pixels() {
    let (session, mut screen, mut fonts) = fixture();
    for size in [(128, 96), (24, 24), (8, 8)] {
        screen.resize(size);
        ready(&session, &mut screen, &mut fonts);
        let frame = frame(&session, &screen);
        for (_, bounds) in frame.controls() {
            assert!(
                bounds.x >= 0.
                    && bounds.y >= 0.
                    && bounds.x + bounds.width <= size.0 as f32
                    && bounds.y + bounds.height <= size.1 as f32,
                "control outside the actual viewport: {bounds:?} at {size:?}"
            );
            assert!(
                bounds.x >= frame.rect.x
                    && bounds.y >= frame.rect.y
                    && bounds.width > 0.
                    && bounds.height > 0.
            );
            assert!(
                bounds.x + bounds.width <= frame.rect.x + frame.rect.width
                    && bounds.y + bounds.height <= frame.rect.y + frame.rect.height
            );
        }
        assert!(screen.elements.game_widgets.is_empty());
        assert!(screen.world.displayed.as_ref().unwrap().views.is_empty());
    }
}

#[test]
fn resetting_positions_keeps_a_server_opened_container_and_its_item_owner() {
    let (mut session, mut screen, mut fonts, _) = setup();
    screen.resize((640, 480));
    let mut child = Item::new();
    child.id = 8;
    let mut bag = Item::new();
    bag.id = 7;
    bag.max_capacity = 2;
    bag.container = Some(vec![child]);
    session.project.regions[0].map.entities[0].inventory = vec![Some(bag)];
    toggle(&session, &mut screen, "preferences");
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(400., 20., 20., 20.)
        }
    ));
    ready(&session, &mut screen, &mut fonts);
    let container = |screen: &GameScreenController| {
        screen
            .hud
            .frames(&session.view(), &screen.elements)
            .into_iter()
            .find(|f| matches!(f.id, HudPanelId::Container { .. }))
            .unwrap()
    };
    let original = container(&screen);
    let title = original.title_rect.unwrap();
    let p = [title.x + 4., title.y + title.height / 2.];
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [p[0] - 30., p[1] + 40.],
        },
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([p[0] - 30., p[1] + 40.], false),
    );
    ready(&session, &mut screen, &mut fonts);
    assert_ne!(container(&screen).position, original.position);
    click(&session, &mut screen, &mut fonts, HudPart::ResetPositions);
    ready(&session, &mut screen, &mut fonts);
    let reset = container(&screen);
    assert_eq!(reset.id, original.id);
    assert_eq!(reset.position, original.position);
    assert_eq!(reset.slots[0].source, original.slots[0].source);
}
