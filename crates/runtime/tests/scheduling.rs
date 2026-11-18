use eldiron_runtime::{GameSession, PlayerSession, Project, SessionOptions};
use std::time::Duration;

fn project() -> Project {
    let mut project = Project::new();
    project.regions[0].name = "Quiet".into();
    project.regions[0].map.name = "Quiet".into();
    project.config = "[game]\nmode='single_player'\nstart_region='Quiet'\nauto_create_player=false\n[multiplayer.simulation]\npause_unoccupied_regions=['Quiet']\nidle_after_ms=20".into();
    project
}
#[test]
fn embedded_single_player_and_creator_do_not_enable_dedicated_region_policy() {
    let mut session = GameSession::new(project());
    session.start(SessionOptions::default()).unwrap();
    for _ in 0..5 {
        session.advance(Duration::from_millis(100));
    }
    assert!(!session.server.has_region_scheduling());
    assert!(
        session
            .server
            .region_schedule()
            .iter()
            .all(|r| !r.suspended)
    );
    let mut p = project();
    p.config = p.config.replace("single_player", "multiplayer");
    let mut creator = PlayerSession::new_local_test(p, true).unwrap();
    creator.advance(Duration::from_secs(1));
    assert!(!creator.runtime.server.has_region_scheduling());
    assert!(
        creator
            .runtime
            .server
            .region_schedule()
            .iter()
            .all(|r| !r.suspended)
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn suspended_world_checkpoints_and_restores_without_catchup_or_duplicate_bodies() {
    use eldiron_runtime::persistence::*;
    use rusterix::{Entity, Value, server::scheduling::RegionSchedulingPolicy};
    use theframework_core::Uuid;
    pollster::block_on(async {
        let p = project();
        let dir = std::env::temp_dir().join(format!("eldiron-idle-{}", Uuid::new_v4()));
        let path = dir.join("world.save");
        let mut session = DurableSession::open(
            p.clone(),
            FileWorldStore::open_journaled(&path).unwrap(),
            SavePurpose::Dedicated,
            SessionOptions {
                create_local_player: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let ctx = session.game.server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap();
        {
            let mut ctx = ctx.lock().unwrap();
            let mut actor = Entity::default();
            actor.id = ctx.world.next_id();
            actor.set_attribute("player", Value::Bool(true));
            actor.inventory = vec![None];
            ctx.map.entities.push(actor);
        }
        session
            .game
            .server
            .set_region_scheduling(RegionSchedulingPolicy {
                pause_unoccupied_regions: [p.regions[0].map.id].into(),
                idle_after: Duration::from_millis(20),
            })
            .unwrap();
        session.advance(Duration::from_millis(20)).await.unwrap();
        assert!(session.game.server.region_schedule()[0].suspended);
        let tick = ctx.lock().unwrap().ticks;
        for _ in 0..20 {
            session.advance(Duration::from_millis(20)).await.unwrap();
        }
        assert_eq!(ctx.lock().unwrap().ticks, tick);
        session.checkpoint().await.unwrap();
        let character = session.saved().characters[0].id;
        drop(session);
        let mut session = DurableSession::open(
            p,
            FileWorldStore::open_journaled(&path).unwrap(),
            SavePurpose::Dedicated,
            SessionOptions {
                create_local_player: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(session.saved().characters.len(), 1);
        assert_eq!(session.saved().world_state.regions[0].ticks, tick);
        let connection = session.game.server.open_player_connection();
        session.join_character(connection, character).unwrap();
        session.advance(Duration::from_millis(20)).await.unwrap();
        let captured = session.game.server.capture_durable_world().unwrap();
        assert_eq!(
            captured.regions[0]
                .entities
                .iter()
                .filter(|e| e.attributes.get_bool_default("player", false))
                .count(),
            1
        );
        assert_eq!(captured.regions[0].entities[0].inventory.len(), 1);
        assert!(!session.game.server.region_schedule()[0].suspended);
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    });
}
