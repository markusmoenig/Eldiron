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

#[test]
fn cpu_session_startup_publishes_authored_tiles_with_coherent_indices() {
    let mut project = Project::new();
    let tile = rusterix::Tile::from_texture(rusterix::Texture::new(vec![30, 90, 150, 255], 1, 1));
    let id = tile.id;
    project.tiles.insert(id, tile.clone());
    let map = project.regions[0].map.id;
    let mut chunk = shared::iso_paint::IsoPaintBakedChunk::new(
        shared::iso_paint::IsoPaintOwner::Sector(7),
        [4, 7, 0, 1],
        [-64, 0],
    );
    chunk.color_rgba[..4].copy_from_slice(&[210, 50, 160, 192]);
    chunk.material_rgba[..4].copy_from_slice(&[254, 0, 128, 255]);
    project.regions[0]
        .iso_paint
        .baked_chunks
        .insert("fixture".into(), chunk);
    let authored_paint = project.regions[0].iso_paint.clone();
    let mut session = GameSession::new(project);
    session.server.print_log_messages = false;
    session.server.state = rusterix::ServerState::Running;
    session.tick_once();
    assert_eq!(session.tick_sequence(), 1);
    session
        .start(SessionOptions {
            create_local_player: false,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(session.tick_sequence(), 0);
    let index = session
        .assets
        .tile_index(&id)
        .expect("startup must publish stable source indices");
    assert_eq!(&session.assets.tile_list[index as usize], &tile);
    assert_eq!(&session.assets.tiles[&id], &tile);
    assert_eq!(
        &session.project.tiles[&id], &tile,
        "publishing source assets cannot alter authored tile data"
    );
    assert!(session.assets.tiles_revision > 0);
    let source = session.assets.surface_paint[&map].clone();
    assert_eq!((source.width, source.height), (64, 64));
    assert_eq!(&source.color_rgba[..4], &[210, 50, 160, 192]);
    assert_eq!(
        source.alpha_geo_ids.len(),
        1,
        "translucent replacement must route its owner through alpha rendering"
    );
    let mut persistent = session.project.regions[0].iso_paint.clone();
    // Source-image cache revision is explicitly serde(skip), unlike paint data.
    persistent.source_revision = authored_paint.source_revision;
    assert!(
        persistent == authored_paint,
        "CPU publication must retain the complete persistent paint description"
    );
    assert!(
        !shared::surface_paint::sync(
            &mut session.assets,
            map,
            &mut session.project.regions[0].iso_paint
        )
        .unwrap()
    );
    assert!(std::sync::Arc::ptr_eq(
        &source,
        &session.assets.surface_paint[&map]
    ));
    session.project.regions[0].iso_paint.visible = false;
    assert!(
        shared::surface_paint::sync(
            &mut session.assets,
            map,
            &mut session.project.regions[0].iso_paint
        )
        .unwrap()
    );
    assert!(!session.assets.surface_paint.contains_key(&map));
    assert_eq!(
        &source.color_rgba[..4],
        &[210, 50, 160, 192],
        "retired publications stay immutable"
    );
    session.stop();
}

#[test]
fn local_tick_sequence_counts_steps_and_catchup_without_render_or_poll_dependence() {
    let mut frequent = GameSession::new(Project::new());
    frequent.server.state = rusterix::ServerState::Running;
    for _ in 0..100 {
        frequent.advance(Duration::from_millis(10));
        frequent.poll();
        frequent.poll();
    }
    assert_eq!(frequent.tick_sequence(), 4);
    let mut sparse = GameSession::new(Project::new());
    sparse.server.state = rusterix::ServerState::Running;
    sparse.advance(Duration::from_secs(1));
    assert_eq!(sparse.tick_sequence(), 4);
    sparse.advance(Duration::from_secs(3));
    assert_eq!(sparse.tick_sequence(), 12);
    sparse.advance(Duration::ZERO);
    assert_eq!(sparse.tick_sequence(), 16);
    sparse.server.state = rusterix::ServerState::Paused;
    sparse.advance(Duration::from_secs(30));
    sparse.tick_once();
    assert_eq!(sparse.tick_sequence(), 16);
    sparse.server.state = rusterix::ServerState::Running;
    sparse.advance(Duration::from_millis(250));
    sparse.tick_once();
    assert_eq!(sparse.tick_sequence(), 18);
}
