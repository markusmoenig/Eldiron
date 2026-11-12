use super::*;
use crate::screen::ScreenImages;
use rusterix::{Texture, Tile};
mod source {
    use super::*;
    #[derive(Default)]
    pub struct Images {
        pub keys: Vec<String>,
        pub pixels: Vec<[u8; 4]>,
    }
    impl ScreenImages for Images {
        fn rgba(
            &mut self,
            key: &str,
            _: u32,
            _: u32,
            pixels: &[u8],
        ) -> Result<theframework::paint::ImageHandle, String> {
            self.keys.push(key.into());
            self.pixels.push(pixels[..4].try_into().unwrap());
            Ok(theframework::paint::ImageHandle {
                owner: 1,
                slot: self.keys.len() as u32,
                generation: 1,
            })
        }
    }
}
fn tile(session: &mut GameSession, color: [u8; 4]) -> Uuid {
    let tile = Tile::from_texture(Texture::new(color.repeat(60), 10, 6));
    let id = tile.id;
    session.assets.tiles.insert(id, tile);
    id
}
fn config(session: &mut GameSession, cursor: Option<Uuid>) {
    let mut table = session.project.config.parse::<toml::Table>().unwrap();
    if let Some(cursor) = cursor {
        table
            .entry("viewport")
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .unwrap()
            .insert("cursor_id".into(), cursor.to_string().into());
    }
    table
        .entry("game")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap()
        .insert("persistent_intents".into(), true.into());
    session.project.config = toml::to_string(&table).unwrap();
}
fn prepare(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    // These CPU emission fixtures acknowledge synthetic view receipts. Supply
    // the same typed image output that a scene producer publishes; actual GPU
    // resource resolution is covered by the assembled-player/GPU fixtures.
    for (index, key) in screen.elements.game_widgets.keys().copied().enumerate() {
        screen.view_images.insert(
            key,
            ViewImage {
                view: ViewId::new(),
                image: renderkit::ImageHandle {
                    owner: 1,
                    slot: u32::MAX - index as u32,
                    generation: 1,
                },
                descriptor: renderkit::ImageDescriptor::rgba(64, 48),
                revision: 1,
                content_revision: 1,
            },
        );
    }
}
fn painted_cursor(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
) -> Option<Uuid> {
    let mut images = source::Images::default();
    screen.paint(&session.view(), fonts, &mut images).unwrap();
    images
        .keys
        .iter()
        .find_map(|key| key.strip_prefix("cursor/").and_then(|id| id.parse().ok()))
}
fn painted_mark(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
) -> [u8; 4] {
    let mut images = source::Images::default();
    let list = screen.paint(&session.view(), fonts, &mut images).unwrap();
    list.ops
        .iter()
        .find_map(|op| {
            if let PaintOp::ThresholdImage { image, .. } = op {
                Some(images.pixels[image.slot as usize - 1])
            } else {
                None
            }
        })
        .expect("intent mark")
}
#[test]
fn custom_cursor_uses_shared_target_and_pressed_fallback_and_restores_host_on_leave_focus_or_missing_art()
 {
    let (mut session, mut screen, mut fonts, key) = setup();
    let default = tile(&mut session, [255, 0, 0, 255]);
    let hover = tile(&mut session, [0, 255, 0, 255]);
    let pressed = tile(&mut session, [0, 0, 255, 255]);
    config(&mut session, Some(default));
    let mut button = Widget::new();
    button.id = 60;
    button.name = "Intent".into();
    button.rect = Rect::new(96., 64., 32., 32.);
    button.command = Some("intent.use".into());
    button.entity_cursor_id = Some(hover);
    button.entity_clicked_cursor_id = Some(pressed);
    screen.elements.button_widgets.insert(60, button);
    screen.elements.activated_widgets.push(60);
    screen.intent = "use".into();
    stage(&session, &mut screen, &mut fonts, key);
    move_to(&session, &mut screen, [24., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default),
        "unacknowledged target cannot select a hover cursor"
    );
    ack(&session, &mut screen);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
    send(&session, &mut screen, button_event(true));
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(pressed)
    );
    send(&session, &mut screen, button_event(false));
    screen
        .elements
        .button_widgets
        .get_mut(&60)
        .unwrap()
        .entity_clicked_cursor_id = None;
    move_to(&session, &mut screen, [24., 24.]);
    send(&session, &mut screen, button_event(true));
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default),
        "a missing clicked cursor does not retain its hover image"
    );
    send(&session, &mut screen, button_event(false));
    move_to(&session, &mut screen, [24., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    let mut images = source::Images::default();
    let list = screen
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap();
    assert!(
        images
            .keys
            .iter()
            .any(|key| key == &format!("cursor/{hover}"))
    );
    assert!(matches!(list.ops.last(), Some(PaintOp::PopClip)));
    assert!(screen.uses_custom_cursor());
    send(&session, &mut screen, InputEvent::PointerLeft);
    prepare(&session, &mut screen, &mut fonts);
    assert!(!screen.uses_custom_cursor());
    move_to(&session, &mut screen, [24., 24.]);
    send(&session, &mut screen, InputEvent::Focus(false));
    prepare(&session, &mut screen, &mut fonts);
    assert!(!screen.uses_custom_cursor());
    send(&session, &mut screen, InputEvent::Focus(true));
    session.assets.tiles.clear();
    move_to(&session, &mut screen, [24., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    assert!(!screen.uses_custom_cursor());
    let mut restored = Tile::from_texture(Texture::new([255, 0, 0, 255].repeat(60), 10, 6));
    restored.id = default;
    session.assets.tiles.insert(default, restored);
    move_to(&session, &mut screen, [24., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.uses_custom_cursor());
    screen.invalidate_world_input();
    assert!(
        !screen.uses_custom_cursor(),
        "host visibility is revoked immediately, before the next preparation"
    );
}
fn button_event(pressed: bool) -> InputEvent {
    button([24., 24.], pressed)
}

#[test]
fn intent_marks_use_threshold_source_images_and_world_drag_preview_is_a_retained_source_not_an_overlay()
 {
    let (mut session, mut screen, mut fonts, key) = drag_setup();
    config(&mut session, None);
    let item_tile = tile(&mut session, [230, 120, 10, 255]);
    session.project.regions[0].map.items[0].set_attribute(
        "source",
        Value::Source(rusterix::PixelSource::TileId(item_tile)),
    );
    session.assets.rules = "[intents.use.ui]\nicon='use'\n[intents.open.ui]\nicon='open'\n[intents.close.ui]\nicon='close'".into();
    for (name, color) in [
        ("use", [240, 100, 30, 255]),
        ("open", [0, 255, 0, 255]),
        ("close", [255, 0, 0, 255]),
    ] {
        session
            .assets
            .textures
            .insert(name.into(), Texture::new(color.repeat(16), 4, 4));
    }
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    screen.intent = "use".into();
    move_to(&session, &mut screen, [24., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    let mut images = source::Images::default();
    let list = screen
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap();
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::ThresholdImage { threshold: 100, .. }))
    );
    screen.intent = "open_close".into();
    session.project.regions[0].map.items[0].set_attribute("openable", Value::Bool(true));
    move_to(&session, &mut screen, [56., 24.]);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_mark(&session, &mut screen, &mut fonts),
        [0, 255, 0, 255]
    );
    session.project.regions[0].map.items[0].set_attribute("active", Value::Bool(true));
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_mark(&session, &mut screen, &mut fonts),
        [255, 0, 0, 255]
    );
    screen.intent.clear();
    send(&session, &mut screen, button([56., 24.], true));
    move_to(&session, &mut screen, [70., 40.]);
    prepare(&session, &mut screen, &mut fonts);
    let builds = screen.source_builds();
    let list = screen
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap();
    assert!(list.ops.iter().any(
        |op| matches!(op, PaintOp::Image { rect, .. } if rect.width == 28. && rect.height == 28.)
    ));
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        screen.source_builds(),
        builds,
        "unchanged drag artwork is not rebuilt"
    );
    for frame in [1, 2, 99, 100] {
        screen
            .prepare(&session.view(), &mut fonts, ContentRevision(1), frame)
            .unwrap();
        assert_eq!(
            screen.source_builds(),
            builds,
            "a static drag source ignores advancing presentation frames"
        );
    }
    session.project.regions[0].map.items.clear();
    prepare(&session, &mut screen, &mut fonts);
    let list = screen
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap();
    assert!(!list.ops.iter().any(
        |op| matches!(op, PaintOp::Image { rect, .. } if rect.width == 28. && rect.height == 28.)
    ));
}

fn owned_cursor_fixture() -> (
    GameSession,
    GameScreenController,
    GameFonts,
    Uuid,
    Uuid,
    Uuid,
) {
    let (mut session, mut screen, fonts) = super::feedback::fixture_owned();
    let default = tile(&mut session, [255, 0, 0, 255]);
    let hover = tile(&mut session, [0, 255, 0, 255]);
    let pressed = tile(&mut session, [0, 0, 255, 255]);
    config(&mut session, Some(default));
    let mut intent = Widget::new();
    intent.id = 60;
    intent.name = "Drop intent".into();
    intent.rect = Rect::new(20., 20., 40., 40.);
    intent.command = Some("intent.drop".into());
    intent.item_cursor_id = Some(hover);
    intent.item_clicked_cursor_id = Some(pressed);
    screen.elements.button_widgets.insert(60, intent);
    screen
        .activate_binding(
            &session.view(),
            rusterix::client::command::ClientCommandBinding::Intent("drop".into()),
        )
        .unwrap();
    assert!(screen.elements.permanently_activated_widgets.contains(&60));
    (session, screen, fonts, default, hover, pressed)
}
#[test]
fn owned_drop_cursors_work_with_ui_only_input_and_retain_pressed_and_drag_source_ownership() {
    let (session, mut screen, mut fonts, default, hover, pressed) = owned_cursor_fixture();
    super::feedback::ready(&session, &mut screen, &mut fonts);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button([40., 100.], true))
            .unwrap()
            .effects
            .is_empty()
    );
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(pressed)
    );
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [120., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    assert_eq!(
        screen.drag_target_feedback(&session.view()),
        Some(Rect::new(100., 80., 40., 40.))
    );
    assert!(matches!(
        &screen
            .input(&session.view(), &mut fonts, &button([120., 100.], false))
            .unwrap()
            .effects[..],
        [ScreenEffect::Action(EntityAction::MoveItem {
            to_inventory_index: Some(1),
            ..
        })]
    ));
    screen
        .input(&session.view(), &mut fonts, &InputEvent::Focus(false))
        .unwrap();
    assert!(
        !screen.uses_custom_cursor(),
        "UI-only focus immediately restores the host pointer"
    );
}
#[test]
fn ui_drop_hover_rejects_unacknowledged_rebinding_covering_and_missing_pressed_art() {
    let (mut session, mut screen, mut fonts, default, hover, _) = owned_cursor_fixture();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    super::feedback::ready(&session, &mut screen, &mut fonts);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
    let item = session.project.regions[0].map.entities[0].inventory[0].take();
    session.project.regions[0].map.entities[0].inventory[1] = item;
    screen
        .elements
        .button_widgets
        .get_mut(&40)
        .unwrap()
        .inventory_index = Some(1);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    super::feedback::ready(&session, &mut screen, &mut fonts);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
    screen
        .elements
        .button_widgets
        .get_mut(&60)
        .unwrap()
        .item_clicked_cursor_id = None;
    screen
        .input(&session.view(), &mut fonts, &button([40., 100.], true))
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    screen
        .input(&session.view(), &mut fonts, &InputEvent::PointerLeft)
        .unwrap();
    assert!(!screen.uses_custom_cursor());
    // Fresh input after leave must not revive its cancelled source capture.
    screen
        .input(&session.view(), &mut fonts, &button([40., 100.], false))
        .unwrap();
    let mut cover = Widget::new();
    cover.id = 99;
    cover.rect = Rect::new(20., 80., 40., 40.);
    screen.elements.button_widgets.insert(99, cover);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
}
#[test]
fn owned_drop_marks_use_the_shared_threshold_icon_and_native_slots_keep_their_direct_item_namespace()
 {
    let (mut session, mut screen, mut fonts, _default, hover, _) = owned_cursor_fixture();
    screen
        .elements
        .button_widgets
        .get_mut(&60)
        .unwrap()
        .item_cursor_id = None;
    session.assets.rules = "[intents.drop.ui]\nicon='drop'".into();
    session
        .assets
        .textures
        .insert("drop".into(), Texture::from_color([200, 70, 20, 255]));
    super::feedback::ready(&session, &mut screen, &mut fonts);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_mark(&session, &mut screen, &mut fonts),
        [200, 70, 20, 255]
    );
    screen
        .elements
        .button_widgets
        .get_mut(&60)
        .unwrap()
        .item_cursor_id = Some(hover);
    screen.elements.custom_inventory_panel_config = Some(screen_model::InventoryPanelConfig {
        columns: 3,
        rows: 1,
        authored_rect: Some(Rect::new(20., 160., 360., 180.)),
        ..Default::default()
    });
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(rusterix::client::command::ClientCommandBinding::Ui(
            "inventory".into()
        ))
    ));
    super::feedback::ready(&session, &mut screen, &mut fonts);
    let frame = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = frame
        .slots
        .iter()
        .find(|slot| slot.source.is_some())
        .unwrap();
    let p = [
        slot.hit.x + slot.hit.width / 2.,
        slot.hit.y + slot.hit.height / 2.,
    ];
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved { position: p },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
}

#[test]
fn ui_only_focus_loss_revokes_item_receipts_until_a_fresh_focused_frame_is_acknowledged() {
    let (session, mut screen, mut fonts, default, hover, _) = owned_cursor_fixture();
    super::feedback::ready(&session, &mut screen, &mut fonts);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
    screen
        .input(&session.view(), &mut fonts, &InputEvent::Focus(false))
        .unwrap();
    let down = screen
        .input(&session.view(), &mut fonts, &button([40., 100.], true))
        .unwrap();
    assert!(down.effects.is_empty());
    assert!(
        !screen.uses_custom_cursor(),
        "an unfocused UI-only host must not reuse an acknowledged item cursor"
    );
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button([40., 100.], false))
            .unwrap()
            .effects
            .is_empty(),
        "unfocused UI-only input submitted an item action"
    );
    screen
        .input(&session.view(), &mut fonts, &InputEvent::Focus(true))
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 100.],
            },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    super::feedback::ready(&session, &mut screen, &mut fonts);
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(hover)
    );
}

#[test]
fn container_children_keep_pickup_policy_and_never_borrow_the_carried_drop_cursor() {
    let (mut session, mut screen, mut fonts, default, _hover, _) = owned_cursor_fixture();
    let mut child = Item::new();
    child.id = 10;
    let mut bag = Item::new();
    bag.id = 9;
    bag.max_capacity = 2;
    bag.container = Some(vec![child]);
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
    super::feedback::ready(&session, &mut screen, &mut fonts);
    let frame = screen
        .hud
        .frames(&session.view(), &screen.elements)
        .pop()
        .unwrap();
    let slot = frame.slots[0].hit;
    let p = [slot.x + slot.width / 2., slot.y + slot.height / 2.];
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved { position: p },
        )
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    screen
        .input(&session.view(), &mut fonts, &button(p, true))
        .unwrap();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(
        painted_cursor(&session, &mut screen, &mut fonts),
        Some(default)
    );
    assert!(matches!(
        &screen
            .input(&session.view(), &mut fonts, &button(p, false))
            .unwrap()
            .effects[..],
        [ScreenEffect::Action(EntityAction::MoveContainerItem {
            item_id: 10,
            container_item_id: 9,
            container_owner_entity_id: Some(1),
            ..
        })]
    ));
    assert_eq!(
        screen.intent, "drop",
        "container pickup does not consume the carried-item intent"
    );
}
