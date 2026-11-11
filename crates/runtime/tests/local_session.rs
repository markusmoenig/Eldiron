use eldiron_runtime::{GameSession, Project, SessionOptions};
use std::time::{Duration, Instant};

#[test]
fn session_collects_each_message_once_and_keeps_clock_independent_of_polling() {
    let project = Project::new();
    let id = project.regions[0].map.id;
    let mut session = GameSession::new(project);
    session.server.region_id_map.insert(id, 7);
    session.server.messages.insert(
        7,
        vec![(None, None, 1, "One delivery".into(), "status".into())],
    );
    session
        .server
        .says
        .insert(7, vec![(Some(1), None, "One speech".into(), "say".into())]);
    assert_eq!(session.poll().system_ticks, 0);
    assert_eq!(session.poll().system_ticks, 0);
    assert!(
        session.server.get_messages(&id).is_empty(),
        "only the session drains engine outputs"
    );
    let events = session.take_events(&id);
    assert_eq!(events.messages.len(), 1);
    assert_eq!(events.messages[0].3, "One delivery");
    assert_eq!(events.says.len(), 1);
    assert_eq!(events.says[0].2, "One speech");
    assert!(session.take_events(&id).messages.is_empty());
    session.server.state = rusterix::ServerState::Running;
    assert_eq!(session.advance(Duration::from_millis(100)).system_ticks, 0);
    session.server.state = rusterix::ServerState::Paused;
    assert_eq!(session.advance(Duration::from_secs(4)).system_ticks, 0);
    session.server.state = rusterix::ServerState::Running;
    assert_eq!(session.advance(Duration::from_millis(150)).system_ticks, 0);
    assert_eq!(session.advance(Duration::from_millis(100)).system_ticks, 1);
}

#[test]
fn real_project_starts_a_cpu_session_and_registers_its_local_player() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let original_ids = project.regions.iter().map(|r| r.id).collect::<Vec<_>>();
    let mut session = GameSession::new(project);
    session.server.print_log_messages = false;
    session.start(SessionOptions::terminal()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        session.tick_once();
        if session.project.regions.iter().any(|r| {
            r.map.name == session.current_map && r.map.entities.iter().any(|e| e.is_player())
        }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "local player did not register: {}",
            session.server.log
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!session.assets.rules.is_empty());
    assert!(
        session.assets.maps.contains_key(&session.current_map),
        "world lookup does not depend on setting up a graphical client"
    );
    assert!(
        session.assets.node_behaviors.is_some(),
        "terminal and graphical sessions must install the same behavior assets"
    );
    assert_eq!(
        session
            .project
            .regions
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        original_ids
    );
    let bytes = shared::project_io::encode_project(&session.project).unwrap();
    let restored = shared::project_io::decode_project(&bytes).unwrap();
    assert_eq!(
        restored.regions.iter().map(|r| r.id).collect::<Vec<_>>(),
        original_ids
    );
    session.stop();
    assert!(session.server.state == rusterix::ServerState::Off);
}
