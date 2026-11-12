use super::*;
use theframework::platform::{InputResponse, LogicalKey};

fn inventory() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, view) = setup();
    let mut item = Item::new();
    item.id = 7;
    session.project.regions[0].map.entities[0].inventory = vec![Some(item), None, None];
    for (id, x, index, equipped) in [
        (10, 0., Some(0), None),
        (20, 32., Some(1), None),
        (30, 64., None, Some("body")),
    ] {
        let mut widget = Widget::new();
        widget.id = id;
        widget.name = format!("Slot{id}");
        widget.rect = Rect::new(x, 0., 32., 32.);
        widget.inventory_index = index;
        widget.equipped_slot = equipped.map(str::to_owned);
        widget.drag_drop = true;
        screen.elements.button_widgets.insert(id, widget);
    }
    (session, screen, fonts, view)
}
fn dispatch(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    event: InputEvent,
) -> ScreenInputResponse {
    let mut result = if screen.route_world_first(&event) {
        ScreenInputResponse::default()
    } else {
        screen.input(&session.view(), fonts, &event).unwrap()
    };
    let blocked = result.input.consumed || screen.captures_world_input();
    let world = screen.world_input(&session.view(), None, &event, blocked);
    result.input = InputResponse {
        consumed: result.input.consumed || world.input.consumed,
        redraw: result.input.redraw || world.input.redraw,
    };
    result.cancel_controls |= world.cancel_controls;
    result.effects.extend(world.effects);
    result
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
fn begin(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
) -> ScreenInputResponse {
    dispatch(session, screen, fonts, button([8., 8.], true))
}
fn finish(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    point: [f32; 2],
) -> ScreenInputResponse {
    dispatch(session, screen, fonts, button(point, false))
}
#[test]
fn owned_item_click_waits_for_ack_uses_its_original_owner_and_submits_one_release() {
    let (session, mut screen, mut fonts, view) = inventory();
    stage(&session, &mut screen, &mut fonts, view);
    assert!(begin(&session, &mut screen, &mut fonts).effects.is_empty());
    assert!(
        finish(&session, &mut screen, &mut fonts, [8., 8.])
            .effects
            .is_empty()
    );
    ack(&session, &mut screen);
    let down = begin(&session, &mut screen, &mut fonts);
    assert!(down.cancel_controls && down.effects.is_empty() && down.input.consumed);
    let up = finish(&session, &mut screen, &mut fonts, [8., 8.]);
    assert_eq!(
        up.effects,
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            0.,
            None,
            Some(1)
        ))]
    );
    assert!(
        finish(&session, &mut screen, &mut fonts, [8., 8.])
            .effects
            .is_empty()
    );
}
#[test]
fn owned_item_drag_routes_inventory_equipment_and_world_destinations_without_click_fallthrough() {
    let (session, mut screen, mut fonts, view) = inventory();
    ready(&session, &mut screen, &mut fonts, view);
    for (point, index, slot) in [([40., 8.], Some(1), None), ([72., 8.], None, Some("body"))] {
        begin(&session, &mut screen, &mut fonts);
        let result = finish(&session, &mut screen, &mut fonts, point);
        assert_eq!(
            result.effects,
            vec![ScreenEffect::Action(EntityAction::MoveItem {
                item_id: 7,
                owner_entity_id: Some(1),
                target_entity_id: Some(1),
                to_inventory_index: index,
                to_equipped_slot: slot.map(str::to_owned)
            })]
        );
        assert!(
            finish(&session, &mut screen, &mut fonts, point)
                .effects
                .is_empty()
        );
    }
    begin(&session, &mut screen, &mut fonts);
    let dropped = finish(&session, &mut screen, &mut fonts, [80., 70.]);
    assert!(
        matches!(&dropped.effects[..], [ScreenEffect::Action(EntityAction::DropItemAt { item_id:7, owner_entity_id:Some(1), position })] if position.x.is_finite() && position.y.is_finite())
    );
}
#[test]
fn item_capture_rejects_replaced_deleted_rebound_or_hidden_source_even_if_world_id_matches() {
    for mode in 0..5 {
        let (mut session, mut screen, mut fonts, view) = inventory();
        let mut duplicate = Item::new();
        duplicate.id = 7;
        session.project.regions[0].map.items.push(duplicate);
        ready(&session, &mut screen, &mut fonts, view);
        begin(&session, &mut screen, &mut fonts);
        match mode {
            0 => {
                session.project.regions[0].map.entities[0].inventory[0] = None;
            }
            1 => {
                let mut other = Item::new();
                other.id = 8;
                session.project.regions[0].map.entities[0].inventory[0] = Some(other);
            }
            2 => {
                screen
                    .elements
                    .button_widgets
                    .get_mut(&10)
                    .unwrap()
                    .inventory_index = Some(1)
            }
            3 => screen.elements.widgets_to_hide.push("Slot10".into()),
            _ => {
                let mut other = actor(2, 2., 2.);
                other.set_attribute("party_index", Value::Int(1));
                let mut item = Item::new();
                item.id = 7;
                other.inventory.push(Some(item));
                session.project.regions[0].map.entities.push(other);
                screen.elements.button_widgets.get_mut(&10).unwrap().party = Some("party.1".into());
            }
        }
        assert!(
            finish(&session, &mut screen, &mut fonts, [40., 8.])
                .effects
                .is_empty()
        );
    }
}
#[test]
fn displayed_and_current_control_stacks_prevent_drop_through_covering_widgets() {
    for displayed in [false, true] {
        let (session, mut screen, mut fonts, view) = inventory();
        let overlay = || {
            let mut overlay = Widget::new();
            overlay.id = 99;
            overlay.name = "Cover".into();
            overlay.rect = Rect::new(32., 0., 32., 32.);
            overlay
        };
        if displayed {
            screen.elements.button_widgets.insert(99, overlay());
        }
        ready(&session, &mut screen, &mut fonts, view);
        if displayed {
            screen.elements.widgets_to_hide.push("Cover".into());
        } else {
            screen.elements.button_widgets.insert(99, overlay());
        }
        begin(&session, &mut screen, &mut fonts);
        assert!(
            finish(&session, &mut screen, &mut fonts, [40., 8.])
                .effects
                .is_empty()
        );
    }
}
#[test]
fn item_capture_cancels_on_leave_focus_escape_and_session_replacement() {
    for mode in 0..4 {
        let (session, mut screen, mut fonts, view) = inventory();
        ready(&session, &mut screen, &mut fonts, view);
        begin(&session, &mut screen, &mut fonts);
        if mode == 3 {
            let mut next = GameSession::new(session.project.clone());
            next.current_map = session.current_map.clone();
            next.server.state = ServerState::Running;
            assert!(
                finish(&next, &mut screen, &mut fonts, [40., 8.])
                    .effects
                    .is_empty()
            );
        } else {
            let event = match mode {
                0 => InputEvent::PointerLeft,
                1 => InputEvent::Focus(false),
                _ => InputEvent::Key {
                    physical: PhysicalKey::Escape,
                    logical: LogicalKey::Other,
                    pressed: true,
                    repeat: false,
                },
            };
            dispatch(&session, &mut screen, &mut fonts, event);
            assert!(
                finish(&session, &mut screen, &mut fonts, [40., 8.])
                    .effects
                    .is_empty()
            );
        }
    }
}
#[test]
fn paused_ui_can_move_owned_items_but_cannot_drop_them_into_an_inactive_world_view() {
    let (mut session, mut screen, mut fonts, view) = inventory();
    ready(&session, &mut screen, &mut fonts, view);
    session.server.state = ServerState::Paused;
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 1)
        .unwrap();
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved { position: [8., 8.] },
    );
    begin(&session, &mut screen, &mut fonts);
    assert!(matches!(
        &finish(&session, &mut screen, &mut fonts, [40., 8.]).effects[..],
        [ScreenEffect::Action(EntityAction::MoveItem {
            owner_entity_id: Some(1),
            to_inventory_index: Some(1),
            ..
        })]
    ));
    begin(&session, &mut screen, &mut fonts);
    assert!(
        finish(&session, &mut screen, &mut fonts, [80., 70.])
            .effects
            .is_empty()
    );
}

#[test]
fn keyboard_item_receipt_rejects_replaced_sources_and_commits_one_matching_release() {
    let (mut session, mut screen, mut fonts, view) = inventory();
    ready(&session, &mut screen, &mut fonts, view);
    screen.focus_control(&mut fonts, Some(ScreenFocus::Button(10)));
    let key = |pressed, repeat| InputEvent::Key {
        physical: PhysicalKey::Enter,
        logical: LogicalKey::Other,
        pressed,
        repeat,
    };
    assert!(
        dispatch(&session, &mut screen, &mut fonts, key(true, false))
            .effects
            .is_empty()
    );
    let old = session.project.regions[0].map.entities[0].inventory[0]
        .take()
        .unwrap();
    let mut other = Item::new();
    other.id = 8;
    session.project.regions[0].map.entities[0].inventory[0] = Some(other);
    assert!(
        dispatch(&session, &mut screen, &mut fonts, key(false, false))
            .effects
            .is_empty()
    );
    session.project.regions[0].map.entities[0].inventory[0] = Some(old);
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(&session, &mut screen, &mut fonts, key(true, false));
    assert!(
        dispatch(&session, &mut screen, &mut fonts, key(true, true))
            .effects
            .is_empty()
    );
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, key(false, false)).effects,
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            0.,
            None,
            Some(1)
        ))]
    );
    assert!(
        dispatch(&session, &mut screen, &mut fonts, key(false, false))
            .effects
            .is_empty()
    );
}
#[test]
fn keyboard_item_activation_requires_the_original_ack_and_operates_in_paused_ui() {
    let (mut session, mut screen, mut fonts, view) = inventory();
    stage(&session, &mut screen, &mut fonts, view);
    screen.focus_control(&mut fonts, Some(ScreenFocus::Button(10)));
    let key = |pressed| InputEvent::Key {
        physical: PhysicalKey::Space,
        logical: LogicalKey::Other,
        pressed,
        repeat: false,
    };
    dispatch(&session, &mut screen, &mut fonts, key(true));
    ack(&session, &mut screen);
    assert!(
        dispatch(&session, &mut screen, &mut fonts, key(false))
            .effects
            .is_empty(),
        "a new acknowledgement cannot rearm an unavailable press"
    );
    session.server.state = ServerState::Paused;
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    dispatch(&session, &mut screen, &mut fonts, key(true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, key(false)).effects,
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            0.,
            None,
            Some(1)
        ))]
    );
}
#[test]
fn non_draggable_item_buttons_still_retain_the_displayed_source_until_release() {
    let (mut session, mut screen, mut fonts, view) = inventory();
    screen
        .elements
        .button_widgets
        .get_mut(&10)
        .unwrap()
        .drag_drop = false;
    ready(&session, &mut screen, &mut fonts, view);
    begin(&session, &mut screen, &mut fonts);
    let mut other = Item::new();
    other.id = 8;
    session.project.regions[0].map.entities[0].inventory[0] = Some(other);
    assert!(
        finish(&session, &mut screen, &mut fonts, [8., 8.])
            .effects
            .is_empty()
    );
}
#[test]
fn owned_containers_toggle_by_default_and_explicit_intents_keep_their_original_item_address() {
    let (mut session, mut screen, mut fonts, view) = inventory();
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .container = Some(Vec::new());
    ready(&session, &mut screen, &mut fonts, view);
    begin(&session, &mut screen, &mut fonts);
    assert!(matches!(
        &finish(&session, &mut screen, &mut fonts, [8., 8.]).effects[..],
        [ScreenEffect::ToggleContainer {
            item: 7,
            owner: 1,
            ..
        }]
    ));
    screen.intent = "look".into();
    begin(&session, &mut screen, &mut fonts);
    assert_eq!(
        finish(&session, &mut screen, &mut fonts, [8., 8.]).effects,
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            0.,
            Some("look".into()),
            Some(1)
        ))]
    );
    assert!(
        screen.intent.is_empty(),
        "classic one-shot intent is consumed only by its committed item click"
    );
}

#[test]
fn pointer_item_receipt_uses_the_clicked_visible_part_instead_of_a_covered_center() {
    let (session, mut screen, mut fonts, view) = inventory();
    screen
        .elements
        .button_widgets
        .get_mut(&10)
        .unwrap()
        .drag_drop = false;
    let mut cover = Widget::new();
    cover.id = 99;
    cover.name = "Cover".into();
    cover.rect = Rect::new(12., 12., 8., 8.);
    screen.elements.button_widgets.insert(99, cover);
    ready(&session, &mut screen, &mut fonts, view);
    begin(&session, &mut screen, &mut fonts);
    assert_eq!(
        finish(&session, &mut screen, &mut fonts, [8., 8.]).effects,
        vec![ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            0.,
            None,
            Some(1)
        ))]
    );
}
