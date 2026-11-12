//! Two documents/Play hosts must own independent CPU worlds in one process.
use eldiron_runtime::{GameSession, PlayerStartOptions, SessionOptions};
use std::time::{Duration, Instant};

fn named_session(name: &str) -> GameSession {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let mut session = GameSession::new(project);
    session.server.print_log_messages = false;
    session
        .start(SessionOptions {
            create_local_player: false,
            ..SessionOptions::terminal()
        })
        .unwrap();
    session
        .start_player(PlayerStartOptions {
            name: Some(name),
            ..Default::default()
        })
        .unwrap();
    session
}
fn wait_for(session: &mut GameSession, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        session.tick_once();
        if session.server.has_local_player()
            && session.view().map.is_some_and(|map| {
                map.entities.iter().any(|entity| {
                    entity.is_player() && entity.attributes.get_str("name") == Some(name)
                })
            })
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "session did not register {name}: {}",
            session.server.log
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn overlapping_sessions_keep_their_own_registration_pause_stop_and_restart() {
    let mut first = named_session("First document");
    wait_for(&mut first, "First document");
    let first_id = first.view().session_id;
    let mut second = named_session("Second document");
    // A submitted registration in another session cannot acknowledge this one.
    assert!(!second.server.has_local_player());
    wait_for(&mut second, "Second document");
    assert!(first.server.has_local_player());
    let first_context = first.server.instances[0].lock().unwrap().context().unwrap();
    let second_context = second.server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    assert!(!std::sync::Arc::ptr_eq(&first_context, &second_context));
    first_context
        .lock()
        .unwrap()
        .set_world_value("quest", rusterix::Value::Int(4));
    assert_eq!(
        second_context.lock().unwrap().get_world_value("quest"),
        None
    );
    first.server.pause();
    first.server.redraw_tick();
    second.tick_once();
    assert!(first_context.lock().unwrap().paused);
    assert!(
        !second_context.lock().unwrap().paused,
        "pausing the first session paused the second region"
    );
    assert!(second.server.has_local_player());
    assert!(
        second
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .any(|e| e.attributes.get_str("name") == Some("Second document"))
    );
    second.stop();
    assert_eq!(first.view().session_id, first_id);
    assert!(
        first.server.has_local_player(),
        "stopping the second document revoked the first player"
    );
    assert_eq!(
        first_context.lock().unwrap().get_world_value("quest"),
        Some(rusterix::Value::Int(4))
    );
    first.server.continue_instances();
    first.action(rusterix::EntityAction::LookYaw(0.75));
    wait_for(&mut first, "First document");
    second
        .start(SessionOptions {
            create_local_player: false,
            ..SessionOptions::terminal()
        })
        .unwrap();
    second
        .start_player(PlayerStartOptions {
            name: Some("Second restarted"),
            ..Default::default()
        })
        .unwrap();
    wait_for(&mut second, "Second restarted");
    let restarted = second.server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    assert!(!std::sync::Arc::ptr_eq(&second_context, &restarted));
    assert_eq!(restarted.lock().unwrap().get_world_value("quest"), None);
    assert_eq!(
        first_context.lock().unwrap().get_world_value("quest"),
        Some(rusterix::Value::Int(4))
    );
    first.stop();
    assert!(
        second.server.has_local_player(),
        "stopping the first document revoked the restarted player"
    );
    wait_for(&mut second, "Second restarted");
}
