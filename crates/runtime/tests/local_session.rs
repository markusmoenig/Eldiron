use eldiron_runtime::{GameSession, PlayerStartOptions, Project, SessionOptions};
use std::time::{Duration, Instant};

#[test]
fn session_collects_each_message_once_and_keeps_clock_independent_of_polling() {
    let mut project = Project::new();
    let mut font = shared::asset::Asset::new();
    font.name = "Authored Font".into();
    font.buffer = shared::asset::AssetBuffer::Font(vec![1, 2, 3]);
    project.assets.insert(font.id, font);
    let mut screen = shared::screen::Screen::new();
    screen.map.name = "Main".into();
    screen.settings = "[layout]\nmode='responsive'".into();
    project.screens.insert(screen.id, screen);
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
    session.current_map = session.project.regions[0].map.name.clone();
    let initial_time = session.project.time;
    for _ in 0..3 {
        let view = session.view();
        assert_eq!(view.map.unwrap().id, id);
        assert_eq!(view.time, initial_time);
        let font = view.fonts().next().unwrap();
        assert_eq!(font.name, "Authored Font");
        assert_eq!(font.bytes, &[1, 2, 3]);
        assert!(view.assets.fonts.is_empty());
        let screen = view.screen("Main").unwrap();
        assert!(screen.responsive);
        assert_eq!(screen.map.name, "Main");
        assert!(view.screen("Missing").is_none());
    }
    assert_eq!(
        session.server.messages[&7].len(),
        1,
        "read-only views cannot drain addressed outputs"
    );
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
fn real_project_supports_automatic_and_deferred_idempotent_player_registration() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let bytes = std::fs::read(path).unwrap();
    // Engine routing is still process-wide; exercise both policies sequentially.
    for automatic in [true, false] {
        let mut project = shared::project_io::decode_project(&bytes).unwrap();
        project.migrate_default_ruleset();
        let original_ids = project.regions.iter().map(|r| r.id).collect::<Vec<_>>();
        let mut session = GameSession::new(project);
        let created_session = session.view().session_id;
        session.server.print_log_messages = false;
        session
            .start(SessionOptions {
                create_local_player: automatic,
                ..SessionOptions::terminal()
            })
            .unwrap();
        assert_eq!(session.has_started_player(), automatic);
        if !automatic {
            let started = session
                .start_player(PlayerStartOptions::default())
                .unwrap()
                .unwrap();
            assert!(!started.template.is_empty());
        }
        assert!(session.has_started_player());
        assert!(
            session
                .start_player(PlayerStartOptions::default())
                .unwrap()
                .is_none()
        );
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
        let active_session = session.view().session_id;
        assert_ne!(created_session, active_session);
        assert_eq!(session.view().session_id, active_session);
        session.stop();
        assert_ne!(session.view().session_id, active_session);
        assert!(session.server.state == rusterix::ServerState::Off);
        assert!(!session.has_started_player());
    }
}
