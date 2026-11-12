use super::*;
use crate::game_screen::{HudPart, ScreenCapture, hud};
use eldiron_runtime::RegionEvents;
use theframework::platform::{InputResponse, LogicalKey};
fn fixture() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, screen, fonts, key) = setup();
    session.assets.rules =
        "[ui.container_templates.default]\ncolumns=2\nslot_size=16\ngap=2\npadding=4\ntitle=true"
            .into();
    let mut bag = Item::new();
    bag.id = 7;
    bag.max_capacity = 4;
    bag.set_attribute("name", Value::Str("Bag 東京".into()));
    bag.container = Some(
        (8..10)
            .map(|id| {
                let mut item = Item::new();
                item.id = id;
                item
            })
            .collect(),
    );
    session.project.regions[0].map.entities[0].inventory = vec![Some(bag), None, None];
    (session, screen, fonts, key)
}
fn open(session: &GameSession, screen: &mut GameScreenController) {
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(0., 0., 32., 32.),
        }
    ));
}
fn layout(session: &GameSession, screen: &GameScreenController) -> hud::model::ContainerFrame {
    screen
        .hud
        .current(&session.view(), screen.elements.layout.viewport)
        .unwrap()
}
fn center(rect: Rect) -> [f32; 2] {
    [rect.x + rect.width / 2., rect.y + rect.height / 2.]
}
fn dispatch(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    event: InputEvent,
) -> ScreenInputResponse {
    let mut ui = if screen.route_world_first(&event) {
        ScreenInputResponse::default()
    } else {
        screen.input(&session.view(), fonts, &event).unwrap()
    };
    let blocked = ui.input.consumed || screen.captures_world_input();
    let world = screen.world_input(&session.view(), None, &event, blocked);
    ui.input = InputResponse {
        consumed: ui.input.consumed || world.input.consumed,
        redraw: ui.input.redraw || world.input.redraw,
    };
    ui.cancel_controls |= world.cancel_controls;
    ui.effects.extend(world.effects);
    ui
}
fn ready(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    key: Uuid,
) {
    stage(session, screen, fonts, key);
    ack(session, screen);
}
fn key(key: PhysicalKey, pressed: bool, repeat: bool) -> InputEvent {
    InputEvent::Key {
        physical: key,
        logical: LogicalKey::Other,
        pressed,
        repeat,
    }
}
#[test]
fn native_container_waits_for_presentation_and_commits_one_addressed_child_click() {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    stage(&session, &mut screen, &mut fonts, view);
    let point = center(layout(&session, &screen).slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    ack(&session, &mut screen);
    let down = dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert!(down.cancel_controls && down.effects.is_empty());
    assert!(matches!(
        screen.control_capture(),
        Some(ScreenCapture::Hud(_))
    ));
    let up = dispatch(&session, &mut screen, &mut fonts, button(point, false));
    assert_eq!(
        up.effects,
        vec![ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 8,
            container_item_id: 7,
            container_owner_entity_id: Some(1),
            target_entity_id: None,
            to_inventory_index: None,
            to_equipped_slot: None
        })]
    );
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
}
#[test]
fn container_child_drags_to_authored_inventory_and_owned_items_drop_into_the_container() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    let mut widget = Widget::new();
    widget.id = 40;
    widget.name = "Inventory".into();
    widget.rect = Rect::new(100., 0., 24., 24.);
    widget.inventory_index = Some(1);
    widget.drag_drop = true;
    screen.elements.button_widgets.insert(40, widget);
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let source = center(layout(&session, &screen).slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    let result = dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([112., 12.], false),
    );
    assert_eq!(
        result.effects,
        vec![ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 8,
            container_item_id: 7,
            container_owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(1),
            to_equipped_slot: None
        })]
    );
    let mut gem = Item::new();
    gem.id = 11;
    session.project.regions[0].map.entities[0].inventory[1] = Some(gem);
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(&session, &mut screen, &mut fonts, button([112., 12.], true));
    let result = dispatch(&session, &mut screen, &mut fonts, button(source, false));
    assert_eq!(
        result.effects,
        vec![ScreenEffect::Action(EntityAction::MoveItemToContainer {
            item_id: 11,
            owner_entity_id: Some(1),
            source_container_item_id: None,
            source_container_owner_entity_id: None,
            container_item_id: 7,
            container_owner_entity_id: Some(1)
        })]
    );
}
#[test]
fn container_captures_reject_replaced_children_reopened_panels_deleted_sources_and_focus_loss() {
    for mode in 0..5 {
        let (mut session, mut screen, mut fonts, view) = fixture();
        open(&session, &mut screen);
        ready(&session, &mut screen, &mut fonts, view);
        let point = center(layout(&session, &screen).slots[0].hit);
        dispatch(&session, &mut screen, &mut fonts, button(point, true));
        match mode {
            0 => {
                session.project.regions[0].map.entities[0].inventory[0]
                    .as_mut()
                    .unwrap()
                    .container
                    .as_mut()
                    .unwrap()[0]
                    .id = 55
            }
            1 => {
                open(&session, &mut screen);
                open(&session, &mut screen);
            }
            2 => session.project.regions[0].map.entities[0].inventory[0] = None,
            3 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::PointerLeft);
            }
            _ => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
            }
        }
        assert!(
            dispatch(&session, &mut screen, &mut fonts, button(point, false))
                .effects
                .is_empty(),
            "mode {mode}"
        );
    }
}
#[test]
fn close_capture_does_not_close_a_new_panel_and_header_drag_remembers_its_position() {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let point = center(layout(&session, &screen).close.unwrap());
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    open(&session, &mut screen);
    open(&session, &mut screen);
    dispatch(&session, &mut screen, &mut fonts, button(point, false));
    assert!(
        screen
            .hud
            .current(&session.view(), screen.elements.layout.viewport)
            .is_some()
    );
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    let point = [frame.rect.x + 3., frame.rect.y + 5.];
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [point[0] - 30., point[1] + 10.],
        },
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([point[0] - 30., point[1] + 10.], false),
    );
    let moved = layout(&session, &screen).rect;
    assert_ne!(moved, frame.rect);
    open(&session, &mut screen);
    open(&session, &mut screen);
    assert_eq!(layout(&session, &screen).rect, moved);
}
#[test]
fn container_keyboard_uses_the_original_receipt_and_closes_without_world_fallthrough() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    screen.hud.focused = Some(frame.control(HudPart::Slot(0)));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Space, true, false),
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Space, true, true)
        )
        .effects
        .is_empty()
    );
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Space, false, false)
        )
        .effects
        .len(),
        1
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Space, true, false),
    );
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .container
        .as_mut()
        .unwrap()[0]
        .id = 99;
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Space, false, false)
        )
        .effects
        .is_empty()
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Escape, true, false),
    );
    assert!(
        screen
            .hud
            .current(&session.view(), screen.elements.layout.viewport)
            .is_none()
    );
}
#[test]
fn overflow_scroll_stays_virtual_and_uses_absolute_slot_addresses() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    let bag = session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap();
    bag.max_capacity = u32::MAX;
    session.assets.rules = "[ui.container_templates.default]\ncolumns=2\nrows=1\nslot_size=16\ngap=2\npadding=4\ntitle=true".into();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    assert!(frame.slots.len() <= 4);
    screen.hud.focused = Some(frame.control(HudPart::Body));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::End, true, false),
    );
    let last = layout(&session, &screen);
    assert!(last.slots.len() <= 4);
    assert_eq!(last.slots.last().unwrap().index, u32::MAX as usize - 1);
    assert!(
        last.slots
            .iter()
            .all(|slot| slot.rect.y.is_finite() && slot.hit.height > 0.)
    );
}

#[test]
fn world_container_ui_opens_only_from_the_addressed_server_request() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    let mut bag = session.project.regions[0].map.entities[0].inventory[0]
        .take()
        .unwrap();
    bag.id = 77;
    session.project.regions[0].map.items.push(bag);
    ready(&session, &mut screen, &mut fonts, view);
    assert!(
        screen
            .hud
            .current(&session.view(), screen.elements.layout.viewport)
            .is_none()
    );
    let mut events = RegionEvents::default();
    events.containers.push((77, None));
    screen.receive(&session.view(), &events);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    assert_eq!(frame.source().owner, None);
    let point = center(frame.slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false)).effects,
        vec![ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 8,
            container_item_id: 77,
            container_owner_entity_id: None,
            target_entity_id: None,
            to_inventory_index: None,
            to_equipped_slot: None
        })]
    );
}

#[test]
fn paused_container_ui_survives_resize_and_never_reuses_a_stale_header_or_item_frame() {
    let (mut session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    session.server.state = ServerState::Paused;
    ready(&session, &mut screen, &mut fonts, view);
    let point = center(layout(&session, &screen).slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .len(),
        1
    );
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    screen.resize((120, 90));
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    assert!(
        screen
            .hud
            .current(&session.view(), screen.elements.layout.viewport)
            .is_some()
    );
}

#[test]
fn leaving_hud_focus_for_a_world_press_restores_world_ownership_and_modified_keys_do_not_pick_up_items()
 {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    screen.hud.focused = Some(frame.control(HudPart::Slot(0)));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::Modifiers(theframework::platform::Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Space, true, false)
        )
        .effects
        .is_empty()
    );
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            key(PhysicalKey::Space, false, false)
        )
        .effects
        .is_empty()
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::Modifiers(Default::default()),
    );
    let empty = center(frame.slots[2].hit);
    dispatch(&session, &mut screen, &mut fonts, button(empty, true));
    dispatch(&session, &mut screen, &mut fonts, button(empty, false));
    assert!(screen.captures_world_input());
    dispatch(&session, &mut screen, &mut fonts, button([8., 80.], true));
    assert!(!screen.captures_world_input());
    assert!(matches!(
        screen.control_capture(),
        Some(ScreenCapture::World(_))
    ));
}

#[test]
fn small_viewports_clip_every_interactive_container_control_to_its_painted_panel() {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    screen.resize((10, 10));
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    for (_, control) in frame.controls() {
        assert!(control.x >= frame.rect.x && control.y >= frame.rect.y);
        assert!(control.x + control.width <= frame.rect.x + frame.rect.width);
        assert!(control.y + control.height <= frame.rect.y + frame.rect.height);
    }
}
