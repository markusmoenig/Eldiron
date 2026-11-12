use super::hud::dispatch;
use super::*;
use rusterix::client::{
    command::ClientCommandBinding,
    widget::choice::{ChoiceWidget, ChoiceWidgetKind},
};
pub(super) fn fixture_owned() -> (GameSession, GameScreenController, GameFonts) {
    let (mut session, mut screen, fonts, _) = setup();
    screen.resize((640, 480));
    screen.elements.game_widgets.clear();
    let mut item = Item::new();
    item.id = 7;
    item.set_attribute("name", Value::Str("Owned source".into()));
    item.set_attribute("slot", Value::Str("main_hand".into()));
    session.project.regions[0].map.entities[0].inventory = vec![Some(item), None, None];
    for (id, x, index, equipped) in [
        (40, 20., Some(0), None),
        (41, 100., Some(1), None),
        (42, 160., None, Some("main_hand")),
    ] {
        let mut w = Widget::new();
        w.id = id;
        w.name = format!("Control {id}");
        w.rect = Rect::new(x, 80., 40., 40.);
        w.inventory_index = index;
        w.equipped_slot = equipped.map(str::to_string);
        w.drag_drop = true;
        screen.elements.button_widgets.insert(id, w);
    }
    (session, screen, fonts)
}
pub(super) fn ready(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    screen.finish_world_frame(&session.view());
    ack(session, screen);
}
fn drag(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    point: [f32; 2],
) {
    assert!(
        dispatch(session, screen, fonts, button([40., 100.], true))
            .effects
            .is_empty()
    );
    assert!(
        dispatch(
            session,
            screen,
            fonts,
            InputEvent::PointerMoved { position: point }
        )
        .effects
        .is_empty()
    );
}
#[test]
fn item_drag_highlight_uses_the_acknowledged_destination_then_balances_one_transfer() {
    let (session, mut screen, mut fonts) = fixture_owned();
    ready(&session, &mut screen, &mut fonts);
    drag(&session, &mut screen, &mut fonts, [120., 100.]);
    assert_eq!(
        screen.drag_target_feedback(&session.view()),
        Some(Rect::new(100., 80., 40., 40.))
    );
    assert_eq!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([120., 100.], false)
        )
        .effects,
        vec![ScreenEffect::Action(EntityAction::MoveItem {
            item_id: 7,
            owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(1),
            to_equipped_slot: None
        })]
    );
    assert_eq!(screen.drag_target_feedback(&session.view()), None);
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([120., 100.], false)
        )
        .effects
        .is_empty()
    );
}
#[test]
fn changed_covered_hidden_unpresented_and_modal_destinations_never_highlight_or_retarget() {
    for change in 0..6 {
        let (session, mut screen, mut fonts) = fixture_owned();
        ready(&session, &mut screen, &mut fonts);
        drag(&session, &mut screen, &mut fonts, [120., 100.]);
        assert!(screen.drag_target_feedback(&session.view()).is_some());
        match change {
            0 => {
                screen
                    .elements
                    .button_widgets
                    .get_mut(&41)
                    .unwrap()
                    .inventory_index = Some(2)
            }
            1 => {
                let mut cover = Widget::new();
                cover.id = 99;
                cover.rect = Rect::new(100., 80., 40., 40.);
                screen.elements.button_widgets.insert(99, cover);
            }
            2 => screen.elements.widgets_to_hide.push("Control 41".into()),
            3 => {
                screen.elements.button_widgets.remove(&41);
            }
            4 => {
                dispatch(&session, &mut screen, &mut fonts, InputEvent::Focus(false));
            }
            _ => {
                let ui = "bind='modal'\noptions=['one','two']".parse().unwrap();
                let choice = ChoiceWidget::from_ui(
                    "Modal".into(),
                    90,
                    Rect::new(400., 20., 100., 24.),
                    ChoiceWidgetKind::Dropdown,
                    &ui,
                )
                .unwrap();
                screen.elements.choice_widgets.insert(90, choice);
                assert!(
                    screen
                        .choice_controller
                        .activate(
                            90,
                            &mut screen.elements.choice_widgets,
                            &screen.elements.widgets_to_hide,
                            &screen.elements.ui_state
                        )
                        .consumed
                );
            }
        }
        assert_eq!(
            screen.drag_target_feedback(&session.view()),
            None,
            "change {change}"
        );
        assert!(
            dispatch(
                &session,
                &mut screen,
                &mut fonts,
                button([120., 100.], false)
            )
            .effects
            .is_empty(),
            "change {change}"
        );
    }
    let (session, mut screen, mut fonts) = fixture_owned();
    screen.elements.button_widgets.remove(&41);
    ready(&session, &mut screen, &mut fonts);
    let mut w = Widget::new();
    w.id = 41;
    w.rect = Rect::new(100., 80., 40., 40.);
    w.inventory_index = Some(1);
    w.drag_drop = true;
    screen.elements.button_widgets.insert(41, w);
    drag(&session, &mut screen, &mut fonts, [120., 100.]);
    assert!(
        screen.drag_target_feedback(&session.view()).is_none(),
        "new geometry waits for its own frame"
    );
}
#[test]
fn equipment_feedback_uses_shared_item_slot_policy_and_replaced_sources_cannot_continue() {
    let (mut session, mut screen, mut fonts) = fixture_owned();
    ready(&session, &mut screen, &mut fonts);
    drag(&session, &mut screen, &mut fonts, [180., 100.]);
    assert_eq!(
        screen.drag_target_feedback(&session.view()),
        Some(Rect::new(160., 80., 40., 40.))
    );
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .set_attribute("slot", Value::Str("head".into()));
    assert!(screen.drag_target_feedback(&session.view()).is_none());
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .set_attribute("slot", Value::Str("MAIN_HAND".into()));
    assert!(screen.drag_target_feedback(&session.view()).is_some());
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .id = 8;
    assert!(screen.drag_target_feedback(&session.view()).is_none());
    assert!(
        dispatch(
            &session,
            &mut screen,
            &mut fonts,
            button([180., 100.], false)
        )
        .effects
        .is_empty()
    );
}
#[test]
fn native_inventory_drop_feedback_keeps_sparse_absolute_addresses_and_ignores_its_window_chrome() {
    let (session, mut screen, mut fonts) = fixture_owned();
    screen.elements.custom_inventory_panel_config = Some(screen_model::InventoryPanelConfig {
        columns: 3,
        rows: 1,
        authored_rect: Some(Rect::new(20., 160., 360., 180.)),
        ..Default::default()
    });
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("inventory".into()))
    ));
    ready(&session, &mut screen, &mut fonts);
    let frame = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = frame
        .slots
        .iter()
        .find(|slot| slot.inventory_index == Some(2))
        .unwrap();
    let p = [
        slot.hit.x + slot.hit.width / 2.,
        slot.hit.y + slot.hit.height / 2.,
    ];
    drag(&session, &mut screen, &mut fonts, p);
    assert_eq!(screen.drag_target_feedback(&session.view()), Some(slot.hit));
    let title = frame.title_rect.unwrap();
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [title.x + 2., title.y + title.height / 2.],
        },
    );
    assert!(screen.drag_target_feedback(&session.view()).is_none());
}
#[test]
fn world_item_drop_feedback_uses_the_same_ui_target_and_rejects_a_late_source_deletion() {
    let (mut session, mut screen, mut fonts, key) = drag_setup();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    send(&session, &mut screen, button([56., 24.], true));
    move_to(&session, &mut screen, [108., 16.]);
    assert_eq!(
        screen.drag_target_feedback(&session.view()),
        Some(screen.elements.button_widgets[&50].rect)
    );
    session.project.regions[0].map.items.clear();
    assert!(screen.drag_target_feedback(&session.view()).is_none());
}

#[test]
fn native_container_feedback_tracks_live_capacity_stacks_and_original_parent_identity() {
    let (mut session, mut screen, mut fonts) = fixture_owned();
    let mut bag = Item::new();
    bag.id = 9;
    bag.max_capacity = 1;
    bag.container = Some(Vec::new());
    session.project.regions[0].map.entities[0].inventory[2] = Some(bag);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 9,
            owner: 1,
            anchor: Uv::new(250., 20., 20., 20.)
        }
    ));
    ready(&session, &mut screen, &mut fonts);
    let frame = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = frame.slots[0].hit;
    let p = [slot.x + slot.width / 2., slot.y + slot.height / 2.];
    drag(&session, &mut screen, &mut fonts, p);
    assert_eq!(screen.drag_target_feedback(&session.view()), Some(slot));
    let mut child = Item::new();
    child.id = 10;
    session.project.regions[0].map.entities[0].inventory[2]
        .as_mut()
        .unwrap()
        .container = Some(vec![child]);
    assert!(
        screen.drag_target_feedback(&session.view()).is_none(),
        "full nonstacking parent is not a preview target"
    );
    let actor = &mut session.project.regions[0].map.entities[0];
    actor.inventory[0]
        .as_mut()
        .unwrap()
        .set_attribute("stackable", Value::Bool(true));
    actor.inventory[0]
        .as_mut()
        .unwrap()
        .set_attribute("name", Value::Str("Coin".into()));
    let child = &mut actor.inventory[2]
        .as_mut()
        .unwrap()
        .container
        .as_mut()
        .unwrap()[0];
    child.set_attribute("stackable", Value::Bool(true));
    child.set_attribute("name", Value::Str("Coin".into()));
    assert_eq!(screen.drag_target_feedback(&session.view()), Some(slot));
    session.project.regions[0].map.entities[0].inventory[2]
        .as_mut()
        .unwrap()
        .id = 11;
    assert!(
        screen.drag_target_feedback(&session.view()).is_none(),
        "a replacement bag cannot reinterpret the displayed parent address"
    );
}
#[test]
fn native_equipment_feedback_uses_canonical_project_slots_and_current_item_type() {
    let (mut session, mut screen, mut fonts) = fixture_owned();
    session.assets.rules = "[equipment]\nweapon_slots=['main_hand']\narmor_slots=['head']".into();
    screen.elements.custom_equipment_panel_config = Some(screen_model::EquipmentPanelConfig {
        left_slots: vec!["MAIN_HAND".into()],
        right_slots: vec!["HEAD".into()],
        avatar_width: 60.,
        avatar_height: 120.,
        slot_size: 32.,
        authored_rect: Some(Rect::new(250., 20., 320., 220.)),
        ..Default::default()
    });
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("equipment".into()))
    ));
    ready(&session, &mut screen, &mut fonts);
    let frame = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = frame
        .slots
        .iter()
        .find(|slot| slot.equipped_slot.as_deref() == Some("main_hand"))
        .unwrap()
        .hit;
    let p = [slot.x + slot.width / 2., slot.y + slot.height / 2.];
    drag(&session, &mut screen, &mut fonts, p);
    assert_eq!(screen.drag_target_feedback(&session.view()), Some(slot));
    session.project.regions[0].map.entities[0].inventory[0]
        .as_mut()
        .unwrap()
        .set_attribute("slot", Value::Str("head".into()));
    assert!(screen.drag_target_feedback(&session.view()).is_none());
    let head = frame
        .slots
        .iter()
        .find(|slot| slot.equipped_slot.as_deref() == Some("head"))
        .unwrap()
        .hit;
    dispatch(
        &session,
        &mut screen,
        &mut fonts,
        InputEvent::PointerMoved {
            position: [head.x + head.width / 2., head.y + head.height / 2.],
        },
    );
    assert_eq!(screen.drag_target_feedback(&session.view()), Some(head));
}
