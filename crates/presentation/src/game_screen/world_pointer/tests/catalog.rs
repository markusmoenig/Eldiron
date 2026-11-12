use super::hud::dispatch;
use super::*;
use crate::game_screen::{HudPanelId, HudPart, ScreenCapture};
use rusterix::client::{command::ClientCommandBinding, rules_ui, screen_model::CatalogPanelConfig};
use theframework::platform::LogicalKey;
fn fixture() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, view) = setup();
    screen.resize((640, 480));
    session.assets.rules = r#"
        [identity.defaults]
        class = 'Adventurer'
        [classes.Adventurer.action_bar]
        main = ['rules.basic_attack', 'rules.defend', 'rules.minor_heal']
        [actions.basic_attack]
        name = 'Arc 東京'
        kind = 'attack'
        description = 'A carefully aimed strike that keeps the original target and respects the current rules. This long description explains the attack, its requirements, its effects and the choice of target across several wrapped lines in a narrow details panel.'
        [actions.defend]
        name = 'Defend'
        kind = 'attack'
        [actions.minor_heal]
        name = 'Minor Heal'
        kind = 'spell'
    "#.into();
    session.project.regions[0].map.entities[0]
        .set_attribute("class", Value::Str("Adventurer".into()));
    let config = CatalogPanelConfig {
        columns: 2,
        rows: Some(1),
        cell_size: 40.,
        spacing: 4.,
        padding: 4.,
        title_height: 26.,
        tab_height: 20.,
        detail_width: 110.,
        font_size: 12.,
        small_font_size: 10.,
        title_font_size: 14.,
        show_assign: true,
        authored_rect: Some(Rect::new(16., 16., 360., 200.)),
        ..Default::default()
    };
    screen.elements.custom_actions_panel_config = Some(config.clone());
    screen.elements.custom_spellbook_config = Some(config);
    let mut target = Widget::new();
    target.id = 40;
    target.name = "Action slot".into();
    target.rect = Rect::new(500., 300., 32., 32.);
    target.command_slot = Some("main.1".into());
    screen.elements.button_widgets.insert(40, target);
    toggle(&session, &mut screen, "actions");
    (session, screen, fonts, view)
}
fn toggle(session: &GameSession, screen: &mut GameScreenController, command: &str) {
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui(command.into()))
    ));
}
fn ready(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    view: Uuid,
) {
    stage(session, screen, fonts, view);
    ack(session, screen);
}
fn layout(
    session: &GameSession,
    screen: &GameScreenController,
) -> crate::game_screen::hud::model::PanelFrame {
    screen
        .hud
        .frames(&session.view(), &screen.elements)
        .into_iter()
        .find(|frame| matches!(frame.id, HudPanelId::Catalog { .. }))
        .unwrap()
}
fn point(session: &GameSession, screen: &GameScreenController, part: HudPart) -> [f32; 2] {
    let bounds = layout(session, screen)
        .controls()
        .into_iter()
        .find(|(id, _)| id.part == part)
        .unwrap()
        .1;
    [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.]
}
fn click_part(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    part: HudPart,
) -> Vec<ScreenEffect> {
    let point = point(session, screen, part);
    dispatch(session, screen, fonts, button(point, true));
    dispatch(session, screen, fonts, button(point, false)).effects
}
fn key(physical: PhysicalKey, pressed: bool, repeat: bool) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed,
        repeat,
    }
}
#[test]
fn native_catalogue_waits_for_its_frame_and_activates_one_original_command() {
    let (session, mut screen, mut fonts, view) = fixture();
    let source = point(&session, &screen, HudPart::CatalogEntry(0));
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(source, false))
            .effects
            .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    assert_eq!(
        layout(&session, &screen).cards[0].command,
        "rules.basic_attack"
    );
    assert_eq!(
        click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0)),
        vec![ScreenEffect::Action(EntityAction::Intent(
            "action:basic_attack".into()
        ))]
    );
    assert_eq!(screen.intent, "action:basic_attack");
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(source, false))
            .effects
            .is_empty()
    );
}
#[test]
fn native_command_drag_assigns_the_acknowledged_slot_once_and_rejects_rebinding_or_coverage() {
    let (session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    let source = point(&session, &screen, HudPart::CatalogEntry(0));
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects,
        vec![ScreenEffect::Action(EntityAction::SetCommandSlot {
            slot: "main.1".into(),
            command: Some("rules.basic_attack".into())
        })]
    );
    assert!(
        screen.intent.is_empty(),
        "assignment does not activate an action"
    );
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    screen
        .elements
        .button_widgets
        .get_mut(&40)
        .unwrap()
        .command_slot = Some("main.2".into());
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects
        .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    let mut cover = Widget::new();
    cover.id = 41;
    cover.rect = Rect::new(500., 300., 32., 32.);
    screen.elements.button_widgets.insert(41, cover);
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects
        .is_empty()
    );
}
#[test]
fn assignment_mode_arms_a_command_then_commits_the_original_target_release() {
    let (session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    click_part(&session, &mut screen, &mut fonts, HudPart::Assign);
    ready(&session, &mut screen, &mut fonts, view);
    assert!(click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0)).is_empty());
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([516., 316.], true),
    );
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects,
        vec![ScreenEffect::Action(EntityAction::SetCommandSlot {
            slot: "main.1".into(),
            command: Some("rules.basic_attack".into())
        })]
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects
        .is_empty()
    );
    assert!(screen.intent.is_empty());
}
#[test]
fn catalogue_pages_tabs_and_spellbook_aliases_keep_command_identities_and_cancel_old_captures() {
    let (session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    let original = layout(&session, &screen).id;
    click_part(&session, &mut screen, &mut fonts, HudPart::NextPage);
    ready(&session, &mut screen, &mut fonts, view);
    assert_eq!(
        layout(&session, &screen).cards[0].command,
        "rules.minor_heal"
    );
    let source = point(&session, &screen, HudPart::CatalogEntry(0));
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    toggle(&session, &mut screen, "spells");
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(source, false))
            .effects
            .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    assert_ne!(frame.id, original);
    assert!(matches!(
        frame.id,
        HudPanelId::Catalog {
            kind: screen_model::CatalogPanelKind::Spellbook,
            ..
        }
    ));
    click_part(&session, &mut screen, &mut fonts, HudPart::CatalogTab(2));
    ready(&session, &mut screen, &mut fonts, view);
    assert_eq!(
        layout(&session, &screen).cards[0].command,
        "rules.minor_heal"
    );
}
#[test]
fn catalogue_keyboard_and_cooldown_changes_never_retarget_or_activate_disabled_sources() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0));
    screen.intent.clear();
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true, false),
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Enter, true, true)
        )
        .effects
        .is_empty()
    );
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Enter, false, false)
        )
        .effects,
        vec![ScreenEffect::Action(EntityAction::Intent(
            "action:basic_attack".into()
        ))]
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true, false),
    );
    session.project.regions[0].map.entities[0].set_attribute(
        &rules_ui::cooldown_left_attr("rules", "basic_attack"),
        Value::Float(2.),
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Enter, false, false)
        )
        .effects
        .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true, false),
    );
    session.project.regions[0].map.entities[0].set_attribute(
        &rules_ui::cooldown_left_attr("rules", "basic_attack"),
        Value::Float(0.),
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Enter, false, false)
        )
        .effects
        .is_empty()
    );
}
#[test]
fn catalogue_source_class_rules_focus_and_leave_changes_revoke_captured_commands() {
    for change in 0..4 {
        let (mut session, mut screen, mut fonts, view) = fixture();
        ready(&session, &mut screen, &mut fonts, view);
        let source = point(&session, &screen, HudPart::CatalogEntry(0));
        dispatch(&session, &mut screen, &mut fonts, button(source, true));
        match change {
            0 => session.project.regions[0].map.entities[0]
                .set_attribute("class", Value::Str("Other".into())),
            1 => session.assets.rules = session.assets.rules.replace("'rules.basic_attack', ", ""),
            2 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
            }
            _ => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::PointerLeft);
            }
        }
        assert!(
            dispatch(
                &session,
                &mut screen,
                &mut fonts,
                button([516., 316.], false)
            )
            .effects
            .is_empty(),
            "change {change}"
        );
    }
}
#[test]
fn catalogue_detail_keyboard_scroll_is_measured_and_independent_of_card_grid() {
    let (session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    click_part(&session, &mut screen, &mut fonts, HudPart::Detail);
    assert!(
        matches!(screen.control_capture(), Some(ScreenCapture::Hud(id)) if id.part == HudPart::Detail)
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::End, true, false),
    );
    assert!(layout(&session, &screen).detail.unwrap().scroll > 0.);
    assert_eq!(layout(&session, &screen).scroll, [0.; 2]);
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Home, true, false),
    );
    assert_eq!(layout(&session, &screen).detail.unwrap().scroll, 0.);
}
#[test]
fn constrained_catalogues_clip_controls_and_keep_declared_huge_pages_bounded() {
    let (session, mut screen, mut fonts, view) = fixture();
    screen.resize((128, 96));
    let config = screen
        .elements
        .custom_actions_panel_config
        .as_mut()
        .unwrap();
    config.columns = usize::MAX;
    config.rows = Some(usize::MAX);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    assert!(frame.cards.len() <= 3);
    for (_, bounds) in frame.controls() {
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
}
#[test]
fn pending_assignment_escape_and_changed_pressed_slot_cannot_submit_a_command() {
    for change in 0..3 {
        let (session, mut screen, mut fonts, view) = fixture();
        ready(&session, &mut screen, &mut fonts, view);
        click_part(&session, &mut screen, &mut fonts, HudPart::Assign);
        ready(&session, &mut screen, &mut fonts, view);
        click_part(&session, &mut screen, &mut fonts, HudPart::CatalogEntry(0));
        if change == 0 {
            dispatch(
                &session,
                &mut screen,
                &mut fonts,
                key(PhysicalKey::Escape, true, false),
            );
        }
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], true),
        );
        if change == 1 {
            screen
                .elements
                .button_widgets
                .get_mut(&40)
                .unwrap()
                .command_slot = Some("main.9".into());
        }
        if change == 2 {
            dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
        }
        let effects = dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false),
        )
        .effects;
        assert!(
            !effects.iter().any(|effect| matches!(
                effect,
                ScreenEffect::Action(EntityAction::SetCommandSlot { .. })
            )),
            "cancelled assignment submitted a command: change {change}: {effects:?}"
        );
        if change == 0 {
            // Escape cancels assignment; a fresh ordinary authored-slot click
            // still activates its existing command through the normal policy.
            assert_eq!(
                effects,
                vec![ScreenEffect::Action(EntityAction::Intent(
                    "action:defend".into()
                ))]
            );
        } else {
            assert!(effects.is_empty(), "change {change}: {effects:?}");
        }
    }
}
#[test]
fn published_source_revision_revokes_a_command_drag_before_it_can_resume_on_an_old_revision() {
    let (session, mut screen, mut fonts, view) = fixture();
    ready(&session, &mut screen, &mut fonts, view);
    let source = point(&session, &screen, HudPart::CatalogEntry(0));
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(2), 0)
        .unwrap();
    assert_eq!(screen.control_capture(), Some(ScreenCapture::Cancelled));
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([516., 316.], false)
        )
        .effects
        .is_empty()
    );
}
