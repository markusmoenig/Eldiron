//! Connection-specific CPU presentation policies must not depend on a GPU.
use eldiron_presentation::GameScreenController;
use eldiron_runtime::{GameSession, Project, RegionEvents};
use rusterix::client::{screen_model::ScreenLayout, widget::game::GameWidget};
use rusterix::{
    Entity, Value,
    party::PlayerView,
    world_text::{TextAnchor, WorldText},
};
use scenevm::prelude::{Vec2, Vec3};

fn world() -> GameSession {
    let mut world = GameSession::new(Project::new());
    world.project.config = "[viewport]\nwidth=400\nheight=240".into();
    world.project.regions[0].map.name = "Shared world".into();
    world.project.regions[0].map.entities.clear();
    world.current_map = "Shared world".into();
    for (id, x) in [(1, 1.), (2, 12.)] {
        let mut player = Entity::default();
        player.id = id;
        player.position = Vec3::new(x, 0., 2.);
        player.set_attribute("player", Value::Bool(true));
        world.project.regions[0].map.entities.push(player);
    }
    world
}

#[test]
fn game_camera_and_text_context_follow_the_selected_character_in_one_shared_map() {
    let world = world();
    let view = world.view();
    let map = view.map.unwrap();
    let mut alice = GameWidget::new();
    let mut bob = GameWidget::new();
    alice.player_view = PlayerView::Controlled(1);
    bob.player_view = PlayerView::Controlled(2);
    alice.prepare_text_view((400, 240), map).unwrap();
    bob.prepare_text_view((400, 240), map).unwrap();
    assert_eq!(alice.player_pos, Vec2::new(1., 2.));
    assert_eq!(bob.player_pos, Vec2::new(12., 2.));
    assert_eq!(
        view.player.resolve(map, None).unwrap().id,
        1,
        "authored previews keep explicit legacy behavior"
    );
    let mut pending = GameWidget::new();
    pending.player_view = PlayerView::Uncontrolled;
    assert_eq!(
        pending.prepare_text_view((400, 240), map).unwrap(),
        ["No current player."]
    );
}

#[test]
fn private_descriptions_follow_the_controlled_player_and_character_switch_revokes_old_ui() {
    let world = world();
    let mut view = world.view();
    view.player = PlayerView::Controlled(1);
    let mut screen = GameScreenController::region_fallback(
        &view,
        ScreenLayout {
            screen_responsive: true,
            reference_viewport: Vec2::new(400, 240),
            viewport: Vec2::new(400, 240),
            grid_size: 32.,
        },
        Default::default(),
    )
    .unwrap();
    let events = || RegionEvents {
        world_text: vec![
            (
                1,
                WorldText {
                    anchor: TextAnchor::Entity(1),
                    text: "Alice private description".into(),
                },
            ),
            (
                2,
                WorldText {
                    anchor: TextAnchor::Entity(2),
                    text: "Bob private description".into(),
                },
            ),
        ],
        ..Default::default()
    };
    screen.receive(&view, &events());
    assert_eq!(screen.world_texts.passages().len(), 1);
    assert_eq!(
        screen.world_texts.passages()[0].message.text,
        "Alice private description"
    );
    view.player = PlayerView::Controlled(2);
    screen.receive(&view, &events());
    assert_eq!(screen.world_texts.passages().len(), 1);
    assert_eq!(
        screen.world_texts.passages()[0].message.text,
        "Bob private description"
    );
    view.player = PlayerView::Uncontrolled;
    screen.receive(&view, &events());
    assert!(screen.world_texts.passages().is_empty());
}

#[test]
fn predicted_view_drives_camera_and_ground_picking_without_mutating_authority() {
    use eldiron_runtime::{
        ClientFrame, MotionClient, MotionSettings, MovementIntent, replication::*,
    };
    use rusterix::{PlayerCamera, ServerState, collision_world::CollisionWorld, movement::*};
    use std::time::Duration;
    use theframework_core::Uuid;

    let world = world();
    let map = &world.project.regions[0].map;
    let content = RegionContentRef {
        map: map.id,
        map_revision: map.changed,
        runtime_revision: 0,
    };
    let mut actor = map.entities[0].clone();
    actor.set_attribute("player_camera", Value::PlayerCamera(PlayerCamera::D2));
    let frame = ClientFrame {
        version: REPLICATION_VERSION,
        world: Uuid::new_v4(),
        connection: Uuid::new_v4(),
        generation: 1,
        sequence: 1,
        tick: 0,
        movement_tick: 0,
        controlled_entity: Some(1),
        current_region: Some(map.id),
        state: ServerState::Running,
        motion: Some(MotionSnapshot {
            generation: 1,
            processed_sequence: 0,
            revision: 0,
            controller: Default::default(),
            parameters: MotionParameters {
                camera: PlayerCamera::D2,
                units_per_second: 4.,
                turn_degrees_per_second: 120.,
                entity_block_mode: 1,
                collision_mode: CollisionMode::Tile,
                turn_period: 0.,
            },
            predictable: true,
        }),
        update: WorldUpdate::Baseline(vec![RegionSnapshot {
            content,
            entities: vec![EntitySnapshot {
                id: 1,
                creator_id: actor.creator_id,
                position: [1., 0., 2.],
                orientation: [1., 0.],
                tilt: 0.,
                motion_revision: 0,
                attributes: actor
                    .attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                equipped: Default::default(),
                inventory: Some(vec![]),
                wallet: Some(Default::default()),
            }],
            items: vec![],
            props: vec![],
            time: Default::default(),
            render: Default::default(),
        }]),
        events: vec![],
    };
    let mut client = MotionClient::new(frame.world, frame.connection, MotionSettings::embedded());
    client.install_content(content, map).unwrap();
    client.install_collision(content, Default::default(), CollisionWorld::new(10));
    client.receive(&frame, &world.assets).unwrap();
    client.set_intent(MovementIntent::Right);
    client.advance(Duration::from_millis(10), &world.assets);
    let view = client.view(&world.project, &world.assets, None).unwrap();
    let actor = view.player_entity().unwrap();
    assert!((actor.position.x - 1.04).abs() < 1e-5);
    assert_eq!(
        client.authority().current_map().unwrap().entities[0]
            .position
            .x,
        1.
    );
    let mut camera = GameWidget::new();
    camera.player_view = view.player;
    camera
        .prepare_text_view((400, 240), view.map.unwrap())
        .unwrap();
    assert_eq!(camera.player_pos, actor.get_pos_xz());
    let screen = camera.ground_to_screen(actor.get_pos_xz());
    assert!((camera.screen_to_ground(screen) - actor.get_pos_xz()).magnitude() < 1e-5);
    // UI/resource recreation consumes the same live view; it owns no movement history.
    let scope = view.session_id;
    let mut recreated = GameWidget::new();
    recreated.player_view = view.player;
    recreated
        .prepare_text_view((400, 240), view.map.unwrap())
        .unwrap();
    assert_eq!(recreated.player_pos, camera.player_pos);
    assert_eq!(
        client
            .view(&world.project, &world.assets, None)
            .unwrap()
            .session_id,
        scope
    );
    assert!(client.ready());
}
