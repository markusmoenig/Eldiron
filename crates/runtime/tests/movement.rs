use eldiron_runtime::{
    GameSession, MovementInput, MovementInputError, MovementIntent, MovementSubmission, Project,
    SessionOptions,
};
use rusterix::{Entity, EntityAction, PlayerCamera, Value};
use std::time::Duration;

#[test]
fn movement_budget_is_independent_of_frame_rate_polls_input_volume_and_pause() {
    let mut project = Project::new();
    project.regions[0].name = "Movement".into();
    project.regions[0].map.name = "Movement".into();
    project.config = "[game]\nstart_region='Movement'\nauto_create_player=false\ncollision_mode='tile'\ngame_tick_ms=250".into();
    let mut host = GameSession::new(project);
    host.server.print_log_messages = false;
    host.start(SessionOptions {
        create_local_player: false,
        ..Default::default()
    })
    .unwrap();
    let id = host.server.local_connection();
    let region = host.project.regions[0].map.id;
    let context = host.server.instances[0].lock().unwrap().context().unwrap();
    context
        .lock()
        .unwrap()
        .entity_player_classes
        .insert("Mover".into());
    let mut actor = Entity::default();
    actor.set_attribute("class_name", Value::Str("Mover".into()));
    actor.set_attribute("player", Value::Bool(true));
    actor.set_attribute(
        "player_camera",
        Value::PlayerCamera(PlayerCamera::D3FirstPMouse),
    );
    host.server
        .create_connected_player(id, &region, actor)
        .unwrap();
    for _ in 0..3 {
        assert!(host.tick_once().replication_error.is_none());
    }
    let actor = host.server.player_control(id).unwrap().entity_id;
    context.lock().unwrap().mapmini = Default::default();
    let origin = host.view().player_entity().unwrap().position;
    for fps in [10, 30, 60, 144] {
        host.clock.reset_elapsed();
        {
            let mut ctx = context.lock().unwrap();
            let player = ctx.map.entities.iter_mut().find(|e| e.id == actor).unwrap();
            player.position = origin;
            player.orientation = theframework_core::prelude::Vec2::unit_x();
        }
        host.action(EntityAction::Forward);
        let before = host.movement_tick_sequence();
        for frame in 0..fps {
            let start = Duration::from_nanos(frame * 1_000_000_000 / fps);
            let end = Duration::from_nanos((frame + 1) * 1_000_000_000 / fps);
            host.advance(end - start);
            let position = host.view().player_entity().unwrap().position;
            for _ in 0..5 {
                assert_eq!(host.poll().movement_steps, 0);
            }
            assert_eq!(host.view().player_entity().unwrap().position, position);
        }
        assert_eq!(host.movement_tick_sequence() - before, 50);
        assert!(
            (host.view().player_entity().unwrap().position.x - origin.x - 4.).abs() < 1e-4,
            "{fps} FPS"
        );
    }
    host.action(EntityAction::Off);
    host.poll();
    let generation = host.client_world(id).unwrap().generation();
    let mut input = MovementInput {
        generation,
        sequence: 1,
        intent: MovementIntent::Forward,
    };
    // 10,000 requests occupy one pending movement state, without granting time.
    let before = host.view().player_entity().unwrap().position;
    for sequence in 1..=10_000 {
        input.sequence = sequence;
        assert_eq!(
            host.submit_movement_input(id, input),
            Ok(MovementSubmission::Accepted)
        );
    }
    input.intent = MovementIntent::Backward;
    assert_eq!(
        host.submit_movement_input(id, input),
        Ok(MovementSubmission::IgnoredStale)
    );
    assert_eq!(
        host.accepted_movement_input(id).unwrap().intent,
        MovementIntent::Forward
    );
    for _ in 0..10 {
        host.poll();
    }
    assert_eq!(host.view().player_entity().unwrap().position, before);
    let update = host.advance(Duration::from_millis(20));
    assert_eq!(update.movement_steps, 1);
    assert!((host.view().player_entity().unwrap().position.x - before.x - 0.08).abs() < 1e-4);

    input.sequence += 1;
    input.intent = MovementIntent::Backward;
    host.submit_movement_input(id, input).unwrap();
    host.request_baseline(id).unwrap();
    assert_eq!(
        host.submit_movement_input(id, input),
        Err(MovementInputError::NeedsBaseline)
    );
    let stopped = host.view().player_entity().unwrap().position;
    host.advance(Duration::from_millis(20));
    assert_eq!(host.view().player_entity().unwrap().position, stopped);
    assert!(host.accepted_movement_input(id).is_none());

    host.action(EntityAction::Forward);
    host.server.pause();
    assert_eq!(host.advance(Duration::from_secs(10)).movement_steps, 0);
    assert_eq!(host.view().player_entity().unwrap().position, stopped);
    host.server.continue_instances();
    assert_eq!(host.advance(Duration::ZERO).movement_steps, 0);
    let before = host.movement_tick_sequence();
    let first = host.advance(Duration::from_secs(3));
    assert!(first.movement_steps <= rusterix::simulation::MAX_MOVEMENT_STEPS);
    let snapshot = host.view().player_entity().unwrap().position;
    for _ in 0..10 {
        assert_eq!(host.poll().movement_steps, 0);
    }
    assert_eq!(host.view().player_entity().unwrap().position, snapshot);
    for _ in 0..4 {
        host.advance(Duration::ZERO);
    }
    assert_eq!(
        host.movement_tick_sequence() - before,
        150,
        "catch-up retains all host time"
    );
    host.action(EntityAction::Off);
    host.action(EntityAction::SetPlayerCamera(PlayerCamera::D3FirstPGrid));
    host.poll();
    {
        let mut ctx = context.lock().unwrap();
        let actor = ctx.map.entities.iter_mut().find(|e| e.id == actor).unwrap();
        actor.position = theframework_core::prelude::Vec3::new(2.5, 0., 2.5);
        actor.orientation = theframework_core::prelude::Vec2::new(0., -1.);
    }
    host.request_baseline(id).unwrap();
    let generation = host.client_world(id).unwrap().generation();
    host.submit_movement_input(
        id,
        MovementInput {
            generation,
            sequence: 1,
            intent: MovementIntent::Forward,
        },
    )
    .unwrap();
    host.submit_movement_input(
        id,
        MovementInput {
            generation,
            sequence: 2,
            intent: MovementIntent::Stop,
        },
    )
    .unwrap();
    for _ in 0..10 {
        host.poll();
    }
    assert_eq!(host.view().player_entity().unwrap().position.z, 2.5);
    host.advance(Duration::from_secs(1));
    assert!(
        (host.view().player_entity().unwrap().position.z - 1.5).abs() < 1e-4,
        "a tap between updates takes exactly one grid step"
    );
    for sequence in 3..35 {
        let intent = if sequence % 2 == 0 {
            MovementIntent::Stop
        } else {
            MovementIntent::Forward
        };
        host.submit_movement_input(
            id,
            MovementInput {
                generation,
                sequence,
                intent,
            },
        )
        .unwrap();
    }
    assert_eq!(
        host.submit_movement_input(
            id,
            MovementInput {
                generation,
                sequence: 35,
                intent: MovementIntent::Forward
            }
        ),
        Err(MovementInputError::Backpressure)
    );
    assert_eq!(host.accepted_movement_input(id).unwrap().sequence, 34);
    assert!(host.close_player_connection(id));
    assert_eq!(
        host.submit_movement_input(id, input),
        Err(MovementInputError::UnknownConnection)
    );
}
