use super::hud::dispatch;
use super::*;
use crate::game_screen::ScreenEffect;
use rusterix::client::command::ClientCommandBinding;
use std::time::Duration;
fn fixture() -> (GameSession, GameScreenController, GameFonts) {
    let (mut session, mut screen, fonts, _) = setup();
    session.server.state = ServerState::Off;
    session.current_map = "No world".into();
    screen.resize((640, 480));
    screen.elements.game_widgets.clear();
    let mut button = Widget::new();
    button.id = 40;
    button.name = "Start command".into();
    button.command = Some("game.start".into());
    button.rect = Rect::new(420., 200., 80., 40.);
    screen.elements.button_widgets.insert(40, button);
    screen.set_tooltip_shortcuts(
        [("game.start".into(), vec!["Enter".into()])]
            .into_iter()
            .collect(),
    );
    (session, screen, fonts)
}
fn prepare(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    screen.finish_world_frame(&session.view());
}
fn ready(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    prepare(session, screen, fonts);
    ack(session, screen);
}
fn hover(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    dispatch(
        session,
        screen,
        fonts,
        InputEvent::PointerMoved {
            position: [450., 220.],
        },
    );
}
fn refresh(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    let content = screen.hover_content(&session.view());
    let prefs = screen.ui_preferences();
    screen
        .tooltips
        .prepare(content, prefs, fonts, "Roboto", (640, 480));
}
#[test]
fn worldless_menu_tooltip_waits_for_ack_and_exact_host_deadline_and_retains_layouts() {
    let (session, mut screen, mut fonts) = fixture();
    hover(&session, &mut screen, &mut fonts);
    refresh(&session, &mut screen, &mut fonts);
    assert!(screen.tooltip_bounds().is_none());
    ready(&session, &mut screen, &mut fonts);
    hover(&session, &mut screen, &mut fonts);
    let content = screen.hover_content(&session.view()).unwrap();
    assert!(
        content
            .description
            .lines
            .iter()
            .any(|line| line == "Shortcut: Enter")
    );
    refresh(&session, &mut screen, &mut fonts);
    assert_eq!(
        screen.presentation_wake_after(),
        Some(Duration::from_millis(650))
    );
    assert!(!screen.advance_presentation(Duration::from_millis(649)));
    refresh(&session, &mut screen, &mut fonts);
    assert!(screen.tooltip_bounds().is_none());
    assert!(screen.advance_presentation(Duration::from_millis(1)));
    refresh(&session, &mut screen, &mut fonts);
    let bounds = screen.tooltip_bounds().unwrap();
    assert!(
        bounds.x >= 0.
            && bounds.y >= 0.
            && bounds.x + bounds.width <= 640.
            && bounds.y + bounds.height <= 480.
    );
    let builds = screen.tooltip_layout_builds();
    assert!(builds > 0);
    refresh(&session, &mut screen, &mut fonts);
    assert_eq!(screen.tooltip_layout_builds(), builds);
    assert_eq!(screen.presentation_wake_after(), None);
    assert!(
        !screen.advance_presentation(Duration::from_secs(5)),
        "a visible tooltip never schedules repeated deadline frames"
    );
    assert!(session.server.state == ServerState::Off);
}
#[test]
fn reauthored_hidden_covered_or_unacknowledged_commands_cannot_supply_a_tooltip() {
    for change in 0..4 {
        let (session, mut screen, mut fonts) = fixture();
        ready(&session, &mut screen, &mut fonts);
        hover(&session, &mut screen, &mut fonts);
        assert!(screen.hover_content(&session.view()).is_some());
        match change {
            0 => {
                screen.elements.button_widgets.get_mut(&40).unwrap().command =
                    Some("game.stop".into())
            }
            1 => screen.elements.widgets_to_hide.push("Start command".into()),
            2 => {
                let mut cover = Widget::new();
                cover.id = 41;
                cover.rect = Rect::new(440., 210., 40., 40.);
                screen.elements.button_widgets.insert(41, cover);
            }
            _ => screen.elements.button_widgets.get_mut(&40).unwrap().rect.x += 10.,
        }
        assert!(
            screen.hover_content(&session.view()).is_none(),
            "change {change}"
        );
        prepare(&session, &mut screen, &mut fonts);
        assert!(
            screen.hover_content(&session.view()).is_none(),
            "prepared is not displayed: change {change}"
        );
        ack(&session, &mut screen);
        if change == 0 || change == 3 {
            assert!(screen.hover_content(&session.view()).is_some());
        } else {
            assert!(screen.hover_content(&session.view()).is_none());
        }
    }
}
#[test]
fn pointer_press_modal_leave_focus_and_runtime_epoch_changes_revoke_hover_without_world_actions() {
    for change in 0..8 {
        let (mut session, mut screen, mut fonts) = fixture();
        ready(&session, &mut screen, &mut fonts);
        hover(&session, &mut screen, &mut fonts);
        assert!(screen.hover_content(&session.view()).is_some());
        match change {
            0 => {
                dispatch(
                    &session,
                    &mut screen,
                    &mut fonts,
                    button([450., 220.], true),
                );
            }
            1 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::PointerLeft);
            }
            2 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
            }
            3 => {
                screen.resize((640, 400));
            }
            4 => {
                session.server.state = ServerState::Running;
            }
            5 => {
                assert!(screen.handle_hud_effect(
                    &session.view(),
                    &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
                ));
                ready(&session, &mut screen, &mut fonts);
                let panel = screen
                    .hud
                    .frames(&session.view(), &screen.elements)
                    .pop()
                    .unwrap();
                let bounds = panel
                    .buttons
                    .iter()
                    .find(|button| {
                        matches!(
                            button.part,
                            crate::game_screen::HudPart::Preference(
                                screen_model::PreferenceSetting::Tooltips
                            )
                        )
                    })
                    .unwrap()
                    .rect;
                let p = [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.];
                dispatch(&session, &mut screen, &mut fonts, button(p, true));
                dispatch(&session, &mut screen, &mut fonts, button(p, false));
                hover(&session, &mut screen, &mut fonts);
                assert!(screen.hud.popup_open());
            }
            6 => {
                screen
                    .prepare(&session.view(), &mut fonts, ContentRevision(2), 0)
                    .unwrap();
            }
            _ => {
                session = GameSession::new(session.project.clone());
            }
        }
        assert!(
            screen.hover_content(&session.view()).is_none(),
            "change {change}"
        );
    }
}
#[test]
fn authored_and_native_inventory_tooltips_resolve_the_acknowledged_item_address_and_suppress_replaced_sources()
 {
    let (mut session, mut screen, mut fonts) = fixture();
    session.server.state = ServerState::Running;
    session.current_map = session.project.regions[0].map.name.clone();
    let mut item = Item::new();
    item.id = 7;
    item.set_attribute("name", Value::Str("Copper key 東京".into()));
    session.project.regions[0].map.entities[0].inventory = vec![Some(item)];
    screen
        .elements
        .button_widgets
        .get_mut(&40)
        .unwrap()
        .inventory_index = Some(0);
    ready(&session, &mut screen, &mut fonts);
    hover(&session, &mut screen, &mut fonts);
    assert_eq!(
        screen
            .hover_content(&session.view())
            .unwrap()
            .description
            .title,
        "Copper key 東京"
    );
    // The displayed icon has the same item ID after this edit, but its
    // authored absolute slot binding changed and must wait for acknowledgement.
    let item = session.project.regions[0].map.entities[0].inventory[0].take();
    session.project.regions[0].map.entities[0]
        .inventory
        .push(item);
    screen
        .elements
        .button_widgets
        .get_mut(&40)
        .unwrap()
        .inventory_index = Some(1);
    assert!(
        screen.hover_content(&session.view()).is_none(),
        "rebound slot reused the previous hover receipt"
    );
    ready(&session, &mut screen, &mut fonts);
    session.project.regions[0].map.entities[0].inventory[1]
        .as_mut()
        .unwrap()
        .id = 8;
    assert!(screen.hover_content(&session.view()).is_none());
    ready(&session, &mut screen, &mut fonts);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("inventory".into()))
    ));
    ready(&session, &mut screen, &mut fonts);
    let panel = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = panel
        .slots
        .iter()
        .find(|slot| slot.source.is_some())
        .unwrap();
    let p = [
        slot.hit.x + slot.hit.width / 2.,
        slot.hit.y + slot.hit.height / 2.,
    ];
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved { position: p },
    );
    assert_eq!(
        screen
            .hover_content(&session.view())
            .unwrap()
            .description
            .title,
        "Copper key 東京"
    );
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    assert!(
        screen.hover_content(&session.view()).is_none(),
        "held and dragged item sources suppress hover"
    );
}
#[test]
fn disabled_tooltips_and_font_replacement_preserve_delay_and_rebuild_only_text_resources() {
    let (session, mut screen, mut fonts) = fixture();
    ready(&session, &mut screen, &mut fonts);
    hover(&session, &mut screen, &mut fonts);
    refresh(&session, &mut screen, &mut fonts);
    screen.advance_presentation(Duration::from_millis(650));
    refresh(&session, &mut screen, &mut fonts);
    let builds = screen.tooltip_layout_builds();
    let bounds = screen.tooltip_bounds();
    fonts.text_mut();
    refresh(&session, &mut screen, &mut fonts);
    assert!(screen.tooltip_layout_builds() > builds);
    assert_eq!(screen.tooltip_bounds(), bounds);
    let content = screen.hover_content(&session.view());
    let mut prefs = screen.ui_preferences();
    prefs.tooltips_enabled = false;
    screen
        .tooltips
        .prepare(content, prefs, &mut fonts, "Roboto", (640, 480));
    assert!(screen.tooltip_bounds().is_none());
    assert_eq!(screen.presentation_wake_after(), None);
}

#[test]
fn native_catalogue_hover_uses_shared_description_and_is_suppressed_when_details_or_command_capture_take_ownership()
 {
    let (mut session, mut screen, mut fonts) = fixture();
    session.server.state = ServerState::Running;
    session.current_map = session.project.regions[0].map.name.clone();
    session.project.regions[0].map.entities[0]
        .set_attribute("class", Value::Str("Adventurer".into()));
    session.assets.rules="[classes.Adventurer.action_bar]\nmain=['rules.arc']\n[actions.arc]\nname='Arc 東京'\nkind='attack'\ndescription='A catalogue description.'".into();
    screen.elements.custom_actions_panel_config = Some(screen_model::CatalogPanelConfig {
        show_details: false,
        authored_rect: Some(Rect::new(20., 20., 360., 200.)),
        ..Default::default()
    });
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("actions".into()))
    ));
    ready(&session, &mut screen, &mut fonts);
    let panel = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let card = &panel.cards[0];
    let p = [
        card.hit.x + card.hit.width / 2.,
        card.hit.y + card.hit.height / 2.,
    ];
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved { position: p },
    );
    assert_eq!(
        screen
            .hover_content(&session.view())
            .unwrap()
            .description
            .title,
        "Arc 東京"
    );
    let key = screen.hover_content(&session.view()).unwrap().key;
    session
        .assets
        .rules
        .push_str("\n[classes.Other.action_bar]\nmain=['rules.arc']\n");
    session.project.regions[0].map.entities[0].set_attribute("class", Value::Str("Other".into()));
    assert!(
        screen.hover_content(&session.view()).is_none(),
        "same command still belongs to a new class policy"
    );
    ready(&session, &mut screen, &mut fonts);
    assert_ne!(screen.hover_content(&session.view()).unwrap().key, key);
    session.project.regions[0].map.entities[0]
        .set_attribute("class", Value::Str("Adventurer".into()));
    ready(&session, &mut screen, &mut fonts);
    screen
        .elements
        .custom_actions_panel_config
        .as_mut()
        .unwrap()
        .show_details = true;
    assert!(screen.hover_content(&session.view()).is_none());
    ready(&session, &mut screen, &mut fonts);
    assert!(screen.hover_content(&session.view()).is_none());
    screen
        .elements
        .custom_actions_panel_config
        .as_mut()
        .unwrap()
        .show_details = false;
    ready(&session, &mut screen, &mut fonts);
    dispatch(&session, &mut screen, &mut fonts, button(p, true));
    assert!(screen.hover_content(&session.view()).is_none());
}

#[test]
fn container_tooltips_keep_child_identity_and_cannot_resolve_a_replaced_parent_container() {
    let (mut session, mut screen, mut fonts) = fixture();
    session.server.state = ServerState::Running;
    session.current_map = session.project.regions[0].map.name.clone();
    let mut child = Item::new();
    child.id = 8;
    child.set_attribute("name", Value::Str("Private bag child".into()));
    let mut bag = Item::new();
    bag.id = 7;
    bag.max_capacity = 2;
    bag.container = Some(vec![child]);
    session.project.regions[0].map.entities[0].inventory = vec![Some(bag)];
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(20., 20., 20., 20.)
        }
    ));
    ready(&session, &mut screen, &mut fonts);
    let panel = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let bounds = panel.slots[0].hit;
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.],
        },
    );
    assert_eq!(
        screen
            .hover_content(&session.view())
            .unwrap()
            .description
            .title,
        "Private bag child"
    );
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .id = 9;
    assert!(screen.hover_content(&session.view()).is_none());
}

#[test]
fn first_acknowledgement_arms_hover_deadline_without_waiting_for_another_visual_frame() {
    let (session, mut screen, mut fonts) = fixture();
    hover(&session, &mut screen, &mut fonts);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        screen.presentation_wake_after(),
        None,
        "an unpresented source cannot arm hover"
    );
    ack(&session, &mut screen);
    assert_eq!(
        screen.presentation_wake_after(),
        Some(Duration::from_millis(650)),
        "the first displayed source must arm its own deadline"
    );
    assert!(screen.advance_presentation(Duration::from_millis(650)));
    refresh(&session, &mut screen, &mut fonts);
    assert!(screen.tooltip_bounds().is_some());
}

#[test]
fn instant_acknowledged_tooltip_requests_one_redraw_without_creating_a_polling_loop() {
    let (session, mut screen, mut fonts) = fixture();
    hover(&session, &mut screen, &mut fonts);
    ready(&session, &mut screen, &mut fonts);
    let content = screen.hover_content(&session.view()).unwrap();
    let prefs = screen_model::UiPreferences {
        tooltip_delay: screen_model::TooltipDelay::Instant,
        ..Default::default()
    };
    screen.tooltips.observe(Some(content.clone()), prefs);
    assert_eq!(screen.presentation_wake_after(), Some(Duration::ZERO));
    assert!(screen.advance_presentation(Duration::ZERO));
    assert!(!screen.advance_presentation(Duration::ZERO));
    screen
        .tooltips
        .prepare(Some(content), prefs, &mut fonts, "Roboto", (640, 480));
    assert!(screen.tooltip_bounds().is_some());
    assert_eq!(screen.presentation_wake_after(), None);
    assert!(!screen.advance_presentation(Duration::ZERO));
}
