use super::*;
use eldiron_runtime::{GameSession, Project};
use rusterix::{Entity, Item, Value, client::widget::Widget};
mod catalog;
mod cursor;
mod feedback;
mod hud;
mod isometric;
mod item_pointer;
mod mouse_look;
mod preferences;
mod tooltip;
fn actor(id: u32, x: f32, z: f32) -> Entity {
    let mut actor = Entity::new();
    actor.id = id;
    actor.position = Vec3::new(x, 0., z);
    actor.set_attribute("visible", Value::Bool(true));
    actor
}
fn setup() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let mut session = GameSession::new(Project::new());
    session.current_map = session.project.regions[0].map.name.clone();
    session.server.state = ServerState::Running;
    let map = &mut session.project.regions[0].map;
    map.entities.clear();
    map.items.clear();
    let mut first = actor(1, 1.5, 1.5);
    first.set_attribute("party_index", Value::Int(0));
    map.entities.push(first);
    let layout = ScreenLayout {
        screen_responsive: true,
        reference_viewport: Vec2::new(128, 96),
        viewport: Vec2::new(128, 96),
        grid_size: 16.,
    };
    let mut screen =
        GameScreenController::region_fallback(&session.view(), layout, Default::default()).unwrap();
    let key = *screen.elements.game_widgets.keys().next().unwrap();
    let widget = screen.elements.game_widgets.get_mut(&key).unwrap();
    widget.camera = PlayerCamera::D2;
    widget.rect = Rect::new(0., 0., 128., 96.);
    widget.top_left = Vec2::zero();
    widget.grid_size = 16.;
    (session, screen, GameFonts::default(), key)
}
fn stage(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    key: Uuid,
) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    let widget = &screen.elements.game_widgets[&key];
    let frame = screen.world.staged.as_mut().unwrap();
    frame.cells = Some(Arc::new(CellTargets::capture_visible(
        session.view().map.unwrap(),
    )));
    frame.views.push(ViewFrame {
        key,
        rect: widget.rect,
        camera: widget.camera.clone(),
        source_size: [64., 48.],
        projection: Projection::Flat {
            origin: widget.top_left,
            grid: widget.grid_size,
            logical: [128., 96.],
        },
        look: None,
    });
}
fn ack(session: &GameSession, screen: &mut GameScreenController) {
    screen.acknowledge_world_frame(&session.view(), None);
}
fn button(point: [f32; 2], pressed: bool) -> InputEvent {
    InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed,
        position: point,
    }
}
fn send(
    session: &GameSession,
    screen: &mut GameScreenController,
    event: InputEvent,
) -> ScreenInputResponse {
    screen.world_input(&session.view(), None, &event, false)
}
fn move_to(session: &GameSession, screen: &mut GameScreenController, point: [f32; 2]) {
    send(
        session,
        screen,
        InputEvent::PointerMoved { position: point },
    );
}
fn click(
    session: &GameSession,
    screen: &mut GameScreenController,
    point: [f32; 2],
) -> Vec<ScreenEffect> {
    let mut effects = send(session, screen, button(point, true)).effects;
    effects.extend(send(session, screen, button(point, false)).effects);
    effects
}
fn drag_setup() -> (GameSession, GameScreenController, GameFonts, Uuid) {
    let (mut session, mut screen, fonts, key) = setup();
    let mut item = Item::new();
    item.id = 7;
    item.position = Vec3::new(3.5, 0., 1.5);
    item.set_attribute("visible", Value::Bool(true));
    session.project.regions[0].map.items.push(item);
    let mut target = Widget::new();
    target.id = 50;
    target.name = "Inventory".into();
    target.rect = Rect::new(96., 0., 32., 32.);
    target.inventory_index = Some(0);
    target.drag_drop = true;
    screen.elements.button_widgets.insert(50, target);
    (session, screen, fonts, key)
}
#[test]
fn only_acknowledged_frames_supply_targets_and_unpresented_preparation_cannot_retarget_a_click() {
    let (mut session, mut screen, mut fonts, key) = setup();
    assert!(click(&session, &mut screen, [24., 24.]).is_empty());
    stage(&session, &mut screen, &mut fonts, key);
    move_to(&session, &mut screen, [24., 24.]);
    assert!(screen.world_pointer_hit().is_none());
    ack(&session, &mut screen);
    assert_eq!(
        screen.world_pointer_hit().and_then(|hit| hit.entity),
        Some(1),
        "an input event before acknowledgement must not discard the pending frame"
    );
    let map = &mut session.project.regions[0].map;
    map.entities[0].position.x = 2.5;
    map.entities.push(actor(2, 1.5, 1.5));
    stage(&session, &mut screen, &mut fonts, key);
    assert_eq!(
        click(&session, &mut screen, [24., 24.]),
        vec![ScreenEffect::Action(EntityAction::EntityClicked(
            1, 0., None
        ))]
    );
    ack(&session, &mut screen);
    assert_eq!(
        screen.world_pointer_hit().and_then(|hit| hit.entity),
        Some(2),
        "stationary hover must follow a newly acknowledged frame"
    );
    assert!(matches!(
        &click(&session, &mut screen, [24., 24.])[..],
        [ScreenEffect::Action(EntityAction::EntityClicked(
            2,
            _,
            None
        ))]
    ));
}
#[test]
fn source_epoch_region_hidden_layout_camera_and_deleted_targets_cannot_reuse_old_hits() {
    let (mut session, mut screen, mut fonts, key) = setup();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    let mut another = GameSession::new(session.project.clone());
    another.current_map = session.current_map.clone();
    another.server.state = ServerState::Running;
    assert!(click(&another, &mut screen, [24., 24.]).is_empty());
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    screen
        .elements
        .widgets_to_hide
        .push(screen.elements.game_widgets[&key].name.clone());
    assert!(click(&session, &mut screen, [24., 24.]).is_empty());
    screen.elements.widgets_to_hide.clear();
    screen.elements.game_widgets.get_mut(&key).unwrap().camera = PlayerCamera::D3FirstP;
    assert!(click(&session, &mut screen, [24., 24.]).is_empty());
    screen.elements.game_widgets.get_mut(&key).unwrap().camera = PlayerCamera::D2;
    screen.elements.game_widgets.get_mut(&key).unwrap().rect.x = 10.;
    assert!(click(&session, &mut screen, [24., 24.]).is_empty());
    screen.elements.game_widgets.get_mut(&key).unwrap().rect.x = 0.;
    session.project.regions[0].map.entities.clear();
    assert!(
        click(&session, &mut screen, [24., 24.]).is_empty(),
        "deleted entities must not become terrain clicks"
    );
    session.project.regions[0].map.id = Uuid::new_v4();
    assert!(click(&session, &mut screen, [24., 24.]).is_empty());
    stage(&session, &mut screen, &mut fonts, key);
    screen.resize((64, 64));
    ack(&session, &mut screen);
    assert!(
        click(&session, &mut screen, [24., 24.]).is_empty(),
        "resize revokes pending as well as displayed frames"
    );
}
#[test]
fn world_item_drag_is_balanced_and_resolves_displayed_inventory_and_world_destinations_once() {
    let (session, mut screen, mut fonts, key) = drag_setup();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    let press = send(&session, &mut screen, button([56., 24.], true));
    assert!(press.effects.is_empty() && press.cancel_controls && screen.world_captures_pointer());
    assert!(screen.route_world_first(&button([108., 16.], true)));
    assert!(
        send(&session, &mut screen, button([108., 16.], true))
            .effects
            .is_empty()
    );
    move_to(&session, &mut screen, [108., 16.]);
    assert_eq!(
        send(&session, &mut screen, button([108., 16.], false)).effects,
        vec![ScreenEffect::Action(EntityAction::MoveItem {
            item_id: 7,
            owner_entity_id: None,
            target_entity_id: Some(1),
            to_inventory_index: Some(0),
            to_equipped_slot: None
        })]
    );
    assert!(!screen.world_captures_pointer());
    assert!(
        send(&session, &mut screen, button([108., 16.], false))
            .effects
            .is_empty()
    );
    send(&session, &mut screen, button([56., 24.], true));
    assert_eq!(
        send(&session, &mut screen, button([80., 64.], false)).effects,
        vec![ScreenEffect::Action(EntityAction::DropItemAt {
            item_id: 7,
            owner_entity_id: None,
            position: Vec2::new(5.0625, 4.0625)
        })]
    );
}
#[test]
fn world_gestures_cancel_on_escape_focus_leave_modal_and_rebound_drop_slots() {
    let (session, mut screen, mut fonts, key) = drag_setup();
    for cancellation in [
        InputEvent::PointerLeft,
        InputEvent::Focus(false),
        InputEvent::Key {
            physical: PhysicalKey::Escape,
            logical: theframework::platform::LogicalKey::Other,
            pressed: true,
            repeat: false,
        },
    ] {
        screen.resume_world_input();
        stage(&session, &mut screen, &mut fonts, key);
        ack(&session, &mut screen);
        send(&session, &mut screen, button([56., 24.], true));
        send(&session, &mut screen, cancellation);
        assert!(
            send(&session, &mut screen, button([108., 16.], false))
                .effects
                .is_empty()
        );
    }
    screen.resume_world_input();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    send(&session, &mut screen, button([56., 24.], true));
    screen
        .elements
        .button_widgets
        .get_mut(&50)
        .unwrap()
        .inventory_index = Some(4);
    assert!(
        send(&session, &mut screen, button([108., 16.], false))
            .effects
            .is_empty()
    );
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    send(&session, &mut screen, button([56., 24.], true));
    let response = screen.world_input(&session.view(), None, &button([108., 16.], false), true);
    assert!(response.effects.is_empty() && response.input.consumed);
}
#[test]
fn ui_occluders_and_source_pixel_sampling_are_retained_until_their_frame_is_replaced() {
    let (session, mut screen, mut fonts, key) = drag_setup();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    screen.elements.button_widgets.clear();
    assert!(
        click(&session, &mut screen, [108., 16.]).is_empty(),
        "a still-displayed slot cannot become a world click before presentation"
    );
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    assert!(matches!(
        &click(&session, &mut screen, [108., 16.])[..],
        [ScreenEffect::Action(EntityAction::TerrainClicked(_))]
    ));
    move_to(&session, &mut screen, [24., 24.]);
    let first = screen.world_pointer_hit().unwrap().world.unwrap();
    move_to(&session, &mut screen, [25.9, 25.9]);
    let second = screen.world_pointer_hit().unwrap().world.unwrap();
    assert_eq!(
        first, second,
        "points in the same upscaled nearest pixel query the same source location"
    );
    assert!(click(&session, &mut screen, [128., 12.]).is_empty());
}

#[test]
fn late_overlay_geometry_is_sealed_with_the_paint_list_and_stale_staging_is_discarded() {
    let (session, mut screen, mut fonts, key) = drag_setup();
    stage(&session, &mut screen, &mut fonts, key);
    // Overlay projections/layouts can complete after game camera preparation.
    screen.elements.button_widgets.get_mut(&50).unwrap().rect = Rect::new(48., 16., 32., 32.);
    screen.finish_world_frame(&session.view());
    ack(&session, &mut screen);
    screen.elements.button_widgets.clear();
    assert!(
        click(&session, &mut screen, [56., 24.]).is_empty(),
        "the final painted overlay, rather than its earlier preparation, blocks the world"
    );
    assert!(matches!(
        &click(&session, &mut screen, [108., 16.])[..],
        [ScreenEffect::Action(EntityAction::TerrainClicked(_))]
    ));
    stage(&session, &mut screen, &mut fonts, key);
    let mut replaced = GameSession::new(session.project.clone());
    replaced.current_map = session.current_map.clone();
    replaced.server.state = ServerState::Running;
    screen.finish_world_frame(&replaced.view());
    assert!(
        screen.world.staged.is_none(),
        "a different world cannot seal an old frame"
    );
}

#[test]
fn world_intents_use_the_acknowledged_policy_and_invalid_configuration_cannot_publish_a_frame() {
    let (mut session, mut screen, mut fonts, key) = setup();
    session.project.config =
        "[viewport]\nwidth=128\nheight=96\n[game]\npersistent_intents=false\n".into();
    screen.intent = "look".into();
    stage(&session, &mut screen, &mut fonts, key);
    session.project.config =
        "[viewport]\nwidth=128\nheight=96\n[game]\npersistent_intents=true\n".into();
    ack(&session, &mut screen);
    assert!(matches!(&click(&session, &mut screen, [24., 24.])[..],
        [ScreenEffect::Action(EntityAction::EntityClicked(1, _, Some(intent)))] if intent == "look"));
    assert!(
        screen.intent.is_empty(),
        "one-shot consumption belongs to the displayed policy"
    );
    screen.intent = "look".into();
    stage(&session, &mut screen, &mut fonts, key);
    ack(&session, &mut screen);
    click(&session, &mut screen, [24., 24.]);
    assert_eq!(screen.intent, "look", "persistent intents remain selected");
    stage(&session, &mut screen, &mut fonts, key);
    session.project.config = "[broken".into();
    assert!(screen.begin_world_frame(&session.view()).is_err());
    assert!(screen.world.staged.is_none());
}

#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn acknowledged_ray_frames_route_actual_geometry_and_reject_newer_dirty_or_unacknowledged_vms() {
    use renderkit::{GpuContext, ImageRegistry};
    use scenevm::{Atom, Camera3D, Chunk, GeoId, Poly3D, RenderMode, SharedAtlas};
    let (mut session, mut screen, mut fonts, key) = setup();
    session.project.regions[0].map.entities[0].set_attribute("player", Value::Bool(true));
    let mut item = Item::new();
    item.id = 7;
    item.position = Vec3::zero();
    item.set_attribute("visible", Value::Bool(true));
    session.project.regions[0].map.items.push(item);
    screen.elements.game_widgets.get_mut(&key).unwrap().camera = PlayerCamera::D3FirstPMouse;
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let atlas = SharedAtlas::new(64, 64);
    let tile = Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![255; 4]], vec![]);
    let mut views = SceneViews::new(&images, atlas);
    let view = ViewId::new();
    views.create(view, (64, 48)).unwrap();
    let scene = views.scene_mut(view).unwrap();
    scene.vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    scene.vm.execute(Atom::SetCamera3D {
        camera: Camera3D::default(),
    });
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(Poly3D::cube(GeoId::Item(7), tile, Vec3::zero(), 1.));
    scene.vm.execute(Atom::AddChunk {
        id: Uuid::new_v4(),
        chunk,
    });
    let output = views.render(view, 1, &mut images).unwrap().unwrap();
    screen.begin_world_frame(&session.view()).unwrap();
    screen
        .world
        .push_view(
            key,
            &screen.elements.game_widgets[&key],
            output,
            session.view().map.unwrap(),
            None,
        )
        .unwrap();
    let press = button([64., 48.], true);
    assert!(
        screen
            .world_input(&session.view(), Some(&views), &press, false)
            .effects
            .is_empty()
    );
    screen.acknowledge_world_frame(&session.view(), Some(&views));
    let response = screen.world_input(&session.view(), Some(&views), &press, false);
    assert!(matches!(
        &response.effects[..],
        [ScreenEffect::Action(EntityAction::ItemClicked(
            7,
            _,
            None,
            None
        ))]
    ));
    assert!(
        screen
            .world_input(
                &session.view(),
                Some(&views),
                &button([64., 48.], false),
                false
            )
            .effects
            .is_empty()
    );
    screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::PointerMoved {
            position: [64., 48.],
        },
        false,
    );
    let look = screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::RelativeMotion { delta: [10., -10.] },
        false,
    );
    assert_eq!(
        look.effects,
        vec![ScreenEffect::Action(EntityAction::LookYaw(2.))]
    );
    assert!((screen.elements.game_widgets[&key].mouse_look_pitch - 2.).abs() < 0.001);
    views
        .scene_mut(view)
        .unwrap()
        .vm
        .execute(Atom::SetCamera3D {
            camera: Camera3D::default().with_pos(Vec3::new(20., 0., 5.)),
        });
    assert!(
        screen
            .world_input(&session.view(), Some(&views), &press, false)
            .effects
            .is_empty()
    );
    let newer = views.render(view, 2, &mut images).unwrap().unwrap();
    assert!(
        screen
            .world_input(&session.view(), Some(&views), &press, false)
            .effects
            .is_empty(),
        "a newer completed VM does not replace the acknowledged camera"
    );
    screen.begin_world_frame(&session.view()).unwrap();
    screen
        .world
        .push_view(
            key,
            &screen.elements.game_widgets[&key],
            newer,
            session.view().map.unwrap(),
            None,
        )
        .unwrap();
    screen.acknowledge_world_frame(&session.view(), Some(&views));
    assert!(
        screen
            .world_input(&session.view(), Some(&views), &press, false)
            .effects
            .is_empty()
    );
    assert!(!views.has_cpu_target_storage(view).unwrap());
    assert_eq!(gpu.transfers().capture_requests, 0);
    assert_eq!(gpu.transfers().readback_bytes, 0);
}
