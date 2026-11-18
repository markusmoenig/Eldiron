use eldiron_runtime::{GameSession, Project, SessionOptions, requests::*};
use rusterix::{Entity, Value};
use theframework_core::Uuid;

#[test]
fn requests_require_current_connection_world_generation_region_and_live_actor() {
    let mut project = Project::new();
    project.regions[0].name = "Requests".into();
    project.regions[0].map.name = "Requests".into();
    project.config = "[game]\nstart_region='Requests'\nauto_create_player=false".into();
    let mut game = GameSession::new(project);
    game.start(SessionOptions {
        create_local_player: false,
        ..Default::default()
    })
    .unwrap();
    let connection = game.open_player_connection().unwrap();
    let region = game.project.regions[0].map.id;
    let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
    let actor_id;
    {
        let mut ctx = ctx.lock().unwrap();
        let mut actor = Entity::default();
        actor_id = ctx.world.next_id();
        actor.id = actor_id;
        actor.set_attribute("player", Value::Bool(true));
        ctx.map.entities.push(actor);
    }
    game.server
        .attach_saved_character(connection, region, actor_id)
        .unwrap();
    for _ in 0..3 {
        game.poll();
    }
    game.subscribe_replication(connection).unwrap();
    let frame = game.take_client_frames(connection).unwrap().pop().unwrap();
    let request = GameRequest::Intent("attack".into());
    let prepared = game.prepare_game_request(
        connection,
        frame.world,
        frame.generation,
        region,
        &request,
        None,
    );
    assert!(
        prepared.is_ok(),
        "{prepared:?}; frame region {:?}; live {:?}",
        frame.current_region,
        ctx.lock().unwrap().map.id
    );
    assert_eq!(
        game.prepare_game_request(
            connection,
            Uuid::new_v4(),
            frame.generation,
            region,
            &request,
            None
        ),
        Err(RequestError::StaleContext)
    );
    assert_eq!(
        game.prepare_game_request(
            connection,
            frame.world,
            frame.generation + 1,
            region,
            &request,
            None
        ),
        Err(RequestError::StaleContext)
    );
    assert_eq!(
        game.prepare_game_request(
            connection,
            frame.world,
            frame.generation,
            Uuid::new_v4(),
            &request,
            None
        ),
        Err(RequestError::StaleContext)
    );
    let spectator = game.open_player_connection().unwrap();
    assert_eq!(
        game.prepare_game_request(
            spectator,
            frame.world,
            frame.generation,
            region,
            &request,
            None
        ),
        Err(RequestError::NotControlled)
    );
    ctx.lock().unwrap().map.entities[0].set_attribute("mode", Value::Str("dead".into()));
    assert_eq!(
        game.prepare_game_request(
            connection,
            frame.world,
            frame.generation,
            region,
            &request,
            None
        ),
        Err(RequestError::Forbidden)
    );
    game.close_player_connection(connection);
    assert_eq!(
        game.prepare_game_request(
            connection,
            frame.world,
            frame.generation,
            region,
            &request,
            None
        ),
        Err(RequestError::NotControlled)
    );
}
