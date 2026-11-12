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
fn layout(session: &GameSession, screen: &GameScreenController) -> hud::model::PanelFrame {
    screen
        .hud
        .frames(&session.view(), &screen.elements)
        .into_iter()
        .next()
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
fn focus_first_slot(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
) {
    let frame = layout(session, screen);
    let point = [frame.rect.x + 3., frame.rect.y + 5.];
    dispatch(session, screen, fonts, button(point, true));
    dispatch(session, screen, fonts, button(point, false));
    dispatch(session, screen, fonts, key(PhysicalKey::Tab, true, false));
    dispatch(session, screen, fonts, key(PhysicalKey::Tab, false, false));
    assert!(
        matches!(screen.control_capture(), Some(ScreenCapture::Hud(id)) if id.part == HudPart::Slot(0))
    );
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
            .frames(&session.view(), &screen.elements)
            .into_iter()
            .next()
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
    focus_first_slot(&session, &mut screen, &mut fonts);
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
            .frames(&session.view(), &screen.elements)
            .into_iter()
            .next()
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
    focus_first_slot(&session, &mut screen, &mut fonts);
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
            .frames(&session.view(), &screen.elements)
            .into_iter()
            .next()
            .is_none()
    );
    let mut events = RegionEvents::default();
    events.containers.push((77, None));
    screen.receive(&session.view(), &events);
    ready(&session, &mut screen, &mut fonts, view);
    let frame = layout(&session, &screen);
    assert_eq!(
        match frame.id {
            hud::HudPanelId::Container { source, .. } => source.owner,
            _ => panic!("expected container"),
        },
        None
    );
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
            .frames(&session.view(), &screen.elements)
            .into_iter()
            .next()
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
    focus_first_slot(&session, &mut screen, &mut fonts);
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

#[test]
fn screen_handoff_retains_panel_sources_and_positions_but_revokes_old_input_and_rejects_other_sessions()
 {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let before = layout(&session, &screen);
    let point = center(before.slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    let builds = screen.source_builds();
    let mut next = GameScreenController::region_fallback(
        &session.view(),
        screen.elements.layout,
        Default::default(),
    )
    .unwrap();
    next.retain_session_presentation_from(&mut screen, &mut fonts);
    let next_view = *next.elements.game_widgets.keys().next().unwrap();
    ready(&session, &mut next, &mut fonts, next_view);
    assert_eq!(next.source_builds(), builds);
    assert_eq!(layout(&session, &next).id, before.id);
    assert_eq!(layout(&session, &next).rect, before.rect);
    assert!(
        dispatch(&session, &mut next, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    dispatch(&session, &mut next, &mut fonts, button(point, true));
    assert_eq!(
        dispatch(&session, &mut next, &mut fonts, button(point, false))
            .effects
            .len(),
        1
    );
    let (foreign, mut other, mut other_fonts, foreign_view) = fixture();
    other.retain_session_presentation_from(&mut next, &mut other_fonts);
    ready(&foreign, &mut other, &mut other_fonts, foreign_view);
    assert!(
        other
            .hud
            .frames(&foreign.view(), &other.elements)
            .is_empty()
    );
    assert!(!next.hud.frames(&session.view(), &next.elements).is_empty());
}

#[test]
fn rejected_screen_preparation_can_return_presentation_owners_without_losing_panel_state() {
    let (session, mut screen, mut fonts, view) = fixture();
    open(&session, &mut screen);
    ready(&session, &mut screen, &mut fonts, view);
    let before = layout(&session, &screen);
    let builds = screen.source_builds();
    let mut staged = GameScreenController::region_fallback(
        &session.view(),
        screen.elements.layout,
        Default::default(),
    )
    .unwrap();
    staged.retain_session_presentation_from(&mut screen, &mut fonts);
    // This is the ownership return path used by the host on preparation error.
    screen.retain_session_presentation_from(&mut staged, &mut fonts);
    assert_eq!(layout(&session, &screen).id, before.id);
    assert_eq!(layout(&session, &screen).rect, before.rect);
    assert_eq!(screen.source_builds(), builds);
    assert!(
        screen.world.displayed.is_none(),
        "a returned owner needs a fresh presentation acknowledgement"
    );
    assert!(
        staged
            .hud
            .frames(&session.view(), &staged.elements)
            .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    let point = center(before.slots[0].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .len(),
        1
    );
}

fn actor_fixture() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, view) = fixture();
    screen.resize((640, 480));
    screen.elements.custom_inventory_panel_config = Some(screen_model::InventoryPanelConfig {
        columns: 2,
        rows: 1,
        cell_size: 24.,
        spacing: 2.,
        padding: 4.,
        title_height: 24.,
        tab_height: 20.,
        footer_height: 24.,
        font_size: 10.,
        title_font_size: 14.,
        authored_rect: Some(Rect::new(8., 8., 280., 180.)),
        ..Default::default()
    });
    screen.elements.custom_equipment_panel_config = Some(screen_model::EquipmentPanelConfig {
        slot_size: 24.,
        padding: 4.,
        title_height: 24.,
        label_width: 60.,
        avatar_width: 50.,
        avatar_height: 70.,
        font_size: 10.,
        title_font_size: 14.,
        authored_rect: Some(Rect::new(8., 8., 250., 120.)),
        ..Default::default()
    });
    session.assets.rules.push_str(
        "\n[equipment]\nweapon_slots=['main_hand','off_hand']\narmor_slots=['head','body']",
    );
    let mut alpha = Item::new();
    alpha.id = 11;
    alpha.set_attribute("name", Value::Str("Alpha".into()));
    alpha.set_attribute("slot", Value::Str("main_hand".into()));
    let mut zinc = Item::new();
    zinc.id = 12;
    zinc.set_attribute("name", Value::Str("Zinc".into()));
    session.project.regions[0].map.entities[0]
        .inventory
        .extend([Some(alpha), None, Some(zinc)]);
    (session, screen, fonts, view)
}
fn toggle_actor(session: &GameSession, screen: &mut GameScreenController, command: &str) {
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(rusterix::client::command::ClientCommandBinding::Ui(
            command.into()
        ))
    ));
}
fn actor_layout(session: &GameSession, screen: &GameScreenController) -> hud::model::PanelFrame {
    screen
        .hud
        .frames(&session.view(), &screen.elements)
        .into_iter()
        .last()
        .unwrap()
}
fn part_point(frame: &hud::model::PanelFrame, part: HudPart) -> [f32; 2] {
    center(
        frame
            .controls()
            .into_iter()
            .find(|(id, _)| id.part == part)
            .unwrap()
            .1,
    )
}
fn hud_click(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    part: HudPart,
) -> Vec<ScreenEffect> {
    let point = part_point(&actor_layout(session, screen), part);
    dispatch(session, screen, fonts, button(point, true));
    dispatch(session, screen, fonts, button(point, false)).effects
}
#[test]
fn native_inventory_pages_sort_popup_and_categories_keep_sparse_addresses() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    assert_eq!(
        frame
            .slots
            .iter()
            .map(|slot| slot.inventory_index)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(3)]
    );
    assert!(hud_click(&session, &mut screen, &mut fonts, HudPart::NextPage).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    assert_eq!(
        frame
            .slots
            .iter()
            .map(|slot| slot.inventory_index)
            .collect::<Vec<_>>(),
        vec![Some(5), Some(1)]
    );
    assert!(hud_click(&session, &mut screen, &mut fonts, HudPart::Sort).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(screen.pointer_modal());
    assert!(actor_layout(&session, &screen).popup.is_some());
    assert!(hud_click(&session, &mut screen, &mut fonts, HudPart::SortOption(1)).is_empty());
    ready(&session, &mut screen, &mut fonts, view);
    assert!(!screen.pointer_modal());
    assert_eq!(screen.elements.ui_state["inventory.sort"], "name");
    assert_eq!(
        actor_layout(&session, &screen).slots[0].inventory_index,
        Some(3)
    );
    hud_click(&session, &mut screen, &mut fonts, HudPart::Category(1));
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    assert_eq!(screen.elements.ui_state["inventory.category"], "equipment");
    assert_eq!(frame.slots[0].inventory_index, Some(3));
    assert_eq!(frame.slots[1].inventory_index, Some(1));
    assert!(
        !frame
            .controls()
            .iter()
            .any(|(id, _)| id.part == HudPart::NextPage)
    );
}
#[test]
fn native_inventory_action_release_rejects_reauthored_values_and_requires_displayed_controls() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    let point = part_point(&actor_layout(&session, &screen), HudPart::Category(1));
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    dispatch(&session, &mut screen, &mut fonts, button(point, false));
    assert!(!screen.elements.ui_state.contains_key("inventory.category"));
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    screen
        .elements
        .custom_inventory_panel_config
        .as_mut()
        .unwrap()
        .categories[1]
        .value = "materials".into();
    dispatch(&session, &mut screen, &mut fonts, button(point, false));
    assert!(!screen.elements.ui_state.contains_key("inventory.category"));
    ready(&session, &mut screen, &mut fonts, view);
    hud_click(&session, &mut screen, &mut fonts, HudPart::Category(1));
    assert_eq!(screen.elements.ui_state["inventory.category"], "materials");
}
#[test]
fn native_inventory_keyboard_bag_and_modal_popup_are_balanced() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    let point = [frame.rect.x + 3., frame.rect.y + 5.];
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    dispatch(&session, &mut screen, &mut fonts, button(point, false));
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Tab, true, false),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Tab, false, false),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true, false),
    );
    let response = dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, false, false),
    );
    assert!(matches!(
        response.effects.as_slice(),
        [ScreenEffect::ToggleContainer {
            item: 7,
            owner: 1,
            ..
        }]
    ));
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
    hud_click(&session, &mut screen, &mut fonts, HudPart::Sort);
    ready(&session, &mut screen, &mut fonts, view);
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Down, true, false),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, true, false),
    );
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        key(PhysicalKey::Enter, false, false),
    );
    assert_eq!(screen.elements.ui_state["inventory.sort"], "name");
    ready(&session, &mut screen, &mut fonts, view);
    hud_click(&session, &mut screen, &mut fonts, HudPart::Sort);
    ready(&session, &mut screen, &mut fonts, view);
    let response = dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([600., 400.], true),
    );
    assert!(response.input.consumed && response.effects.is_empty());
    let response = dispatch(
        &session,
        &mut screen,
        &mut fonts,
        button([600., 400.], false),
    );
    assert!(response.input.consumed && response.effects.is_empty());
    assert!(!screen.pointer_modal());
    assert_eq!(
        screen.hud.frames(&session.view(), &screen.elements).len(),
        1
    );
}
#[test]
fn container_children_transfer_to_native_sparse_inventory_slots_and_native_equipment_slots() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(450., 240., 20., 20.),
        },
    );
    ready(&session, &mut screen, &mut fonts, view);
    let frames = screen.hud.frames(&session.view(), &screen.elements);
    assert_eq!(frames.len(), 2);
    let source = center(frames[0].slots[0].hit);
    hud_click(&session, &mut screen, &mut fonts, HudPart::NextPage);
    ready(&session, &mut screen, &mut fonts, view);
    let target = center(actor_layout(&session, &screen).slots[1].hit);
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(target, false)).effects,
        vec![ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 8,
            container_item_id: 7,
            container_owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(1),
            to_equipped_slot: None
        })]
    );
    toggle_actor(&session, &mut screen, "gear");
    screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(450., 240., 20., 20.),
        },
    );
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    let target = center(
        frame
            .slots
            .iter()
            .find(|slot| slot.equipped_slot.as_deref() == Some("main_hand"))
            .unwrap()
            .hit,
    );
    dispatch(&session, &mut screen, &mut fonts, button(source, true));
    assert_eq!(
        dispatch(&session, &mut screen, &mut fonts, button(target, false)).effects,
        vec![ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 8,
            container_item_id: 7,
            container_owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: None,
            to_equipped_slot: Some("main_hand".into())
        })]
    );
}
#[test]
fn native_equipment_sources_and_destinations_use_canonical_slots_and_cancel_when_rebound() {
    let (mut session, mut screen, mut fonts, view) = actor_fixture();
    let mut sword = Item::new();
    sword.id = 22;
    session.project.regions[0].map.entities[0]
        .equipped
        .insert("main_hand".into(), sword);
    screen
        .elements
        .custom_equipment_panel_config
        .as_mut()
        .unwrap()
        .left_slots = vec!["MAIN_HAND".into()];
    let mut destination = Widget::new();
    destination.id = 40;
    destination.rect = Rect::new(500., 300., 24., 24.);
    destination.inventory_index = Some(2);
    destination.drag_drop = true;
    screen.elements.button_widgets.insert(40, destination);
    toggle_actor(&session, &mut screen, "character");
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    let slot = frame
        .slots
        .iter()
        .find(|slot| slot.equipped_slot.as_deref() == Some("main_hand"))
        .unwrap();
    assert_eq!(slot.source.unwrap().item(), 22);
    let point = center(slot.hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([512., 312.], false)
        )
        .effects,
        vec![ScreenEffect::Action(EntityAction::MoveItem {
            item_id: 22,
            owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(2),
            to_equipped_slot: None
        })]
    );
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    session.project.regions[0].map.entities[0]
        .equipped
        .get_mut("main_hand")
        .unwrap()
        .id = 23;
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([512., 312.], false)
        )
        .effects
        .is_empty()
    );
}
#[test]
fn native_actor_panels_remain_virtual_when_constrained_and_closing_revokes_old_controls() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    screen.resize((128, 96));
    let config = screen
        .elements
        .custom_inventory_panel_config
        .as_mut()
        .unwrap();
    config.columns = usize::MAX;
    config.rows = usize::MAX;
    toggle_actor(&session, &mut screen, "bag");
    ready(&session, &mut screen, &mut fonts, view);
    let frame = actor_layout(&session, &screen);
    assert!(frame.slots.len() < 100);
    assert!(frame.max_scroll[0] > 0. && frame.max_scroll[1] > 0.);
    assert!(
        frame
            .controls()
            .iter()
            .all(|(_, bounds)| bounds.x >= frame.rect.x
                && bounds.y >= frame.rect.y
                && bounds.x + bounds.width <= frame.rect.x + frame.rect.width
                && bounds.y + bounds.height <= frame.rect.y + frame.rect.height)
    );
    let point = part_point(&frame, HudPart::Close);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    toggle_actor(&session, &mut screen, "gear");
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    assert!(matches!(
        actor_layout(&session, &screen).id,
        hud::HudPanelId::Equipment { .. }
    ));
}
#[test]
fn covered_container_close_cannot_activate_behind_a_later_native_panel() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(450., 240., 20., 20.),
        },
    );
    ready(&session, &mut screen, &mut fonts, view);
    let before = layout(&session, &screen);
    let point = part_point(&before, HudPart::Close);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    // Enlarge the existing inventory over the captured close without a new
    // displayed frame. The old container identity and close bounds survive.
    screen
        .elements
        .custom_inventory_panel_config
        .as_mut()
        .unwrap()
        .authored_rect = Some(Rect::new(8., 8., 600., 400.));
    dispatch(&session, &mut screen, &mut fonts, button(point, false));
    assert_eq!(layout(&session, &screen).id, before.id);
    assert_eq!(
        screen.hud.frames(&session.view(), &screen.elements).len(),
        2
    );
}
#[test]
fn inventory_item_receipts_reject_reordering_and_select_the_original_sparse_item() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts, view);
    let point = center(actor_layout(&session, &screen).slots[1].hit);
    dispatch(&session, &mut screen, &mut fonts, button(point, true));
    assert!(actor_layout(&session, &screen).slots[1].selected);
    screen
        .elements
        .ui_state
        .insert("inventory.sort".into(), "name".into());
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    ready(&session, &mut screen, &mut fonts, view);
    assert!(actor_layout(&session, &screen).slots[0].selected);
}
#[test]
fn inventory_sort_popup_consumes_a_press_on_another_floating_panel() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    toggle_actor(&session, &mut screen, "inventory");
    screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: Uv::new(450., 240., 20., 20.),
        },
    );
    ready(&session, &mut screen, &mut fonts, view);
    let container = layout(&session, &screen);
    hud_click(&session, &mut screen, &mut fonts, HudPart::Sort);
    ready(&session, &mut screen, &mut fonts, view);
    let point = part_point(&container, HudPart::Close);
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, true))
            .input
            .consumed
    );
    assert!(
        dispatch(&session, &mut screen, &mut fonts, button(point, false))
            .effects
            .is_empty()
    );
    assert!(!screen.pointer_modal());
    assert_eq!(
        screen.hud.frames(&session.view(), &screen.elements).len(),
        2,
        "dismissing the dropdown cannot also close a different floating panel"
    );
    assert_eq!(layout(&session, &screen).id, container.id);
}
#[test]
fn native_inventory_choices_synchronize_authored_binding_owners() {
    let (session, mut screen, mut fonts, view) = actor_fixture();
    for (id, value) in [(40, "all"), (41, "equipment")] {
        let mut widget = Widget::new();
        widget.id = id;
        widget.rect = Rect::new(500., (id - 40) as f32 * 32., 24., 24.);
        widget.binding = Some("inventory.category".into());
        widget.value = Some(value.into());
        widget.selection = Some("single".into());
        screen.elements.button_widgets.insert(id, widget);
    }
    screen.elements.set_binding("inventory.category", "all");
    assert!(screen.elements.activated_widgets.contains(&40));
    toggle_actor(&session, &mut screen, "inventory");
    ready(&session, &mut screen, &mut fonts, view);
    assert!(screen.geometry.is_some());
    hud_click(&session, &mut screen, &mut fonts, HudPart::Category(1));
    assert!(
        screen.geometry.is_none(),
        "bound authored paint is invalidated too"
    );
    assert!(!screen.elements.activated_widgets.contains(&40));
    assert!(screen.elements.activated_widgets.contains(&41));
    assert!(screen.elements.permanently_activated_widgets.contains(&41));
}
