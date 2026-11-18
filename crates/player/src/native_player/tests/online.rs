use super::super::*;
use std::{sync::mpsc, time::Instant};
fn project() -> Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    project
}
struct ServerFixture {
    project: Project,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
    directory: std::path::PathBuf,
}
impl ServerFixture {
    fn start(saved: bool) -> Self {
        Self::start_with_content_delay(saved, Duration::ZERO)
    }
    fn start_with_content_delay(saved: bool, delay: Duration) -> Self {
        Self::start_project(saved, delay, project())
    }
    fn start_project(saved: bool, delay: Duration, mut project: Project) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let proxy = if delay.is_zero() {
            None
        } else {
            let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            proxy.set_nonblocking(true).unwrap();
            Some(proxy)
        };
        let client_address = proxy
            .as_ref()
            .map(|p| p.local_addr().unwrap())
            .unwrap_or(address);
        let mut config = project.config.parse::<toml::Table>().unwrap();
        config
            .get_mut("game")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert("mode".into(), "multiplayer".into());
        config.insert(
            "multiplayer".into(),
            toml::Value::Table(toml::Table::from_iter([
                (
                    "endpoint".into(),
                    format!("ws://{client_address}/play").into(),
                ),
                ("allow_insecure_loopback".into(), true.into()),
            ])),
        );
        project.config = toml::to_string(&config).unwrap();
        let source = project.clone();
        let directory =
            std::env::temp_dir().join(format!("eldiron-project-mode-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = saved.then(|| directory.join("world.save"));
        let (ready, rx) = mpsc::channel();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let join = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let host = eldiron_server::host::Host::open(source, path)
                    .await
                    .unwrap();
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let proxy =
                    proxy.map(|proxy| tokio::spawn(delayed_content_proxy(proxy, address, delay)));
                ready.send(()).unwrap();
                eldiron_server::transport::serve(
                    listener,
                    host,
                    eldiron_server::transport::TransportConfig {
                        tls: None,
                        origins: vec![],
                    },
                    async {
                        let _ = shutdown.await;
                    },
                )
                .await
                .unwrap();
                if let Some(proxy) = proxy {
                    proxy.abort();
                    let _ = proxy.await;
                }
            });
        });
        rx.recv_timeout(Duration::from_secs(180)).unwrap();
        Self {
            project,
            stop: Some(stop),
            join: Some(join),
            directory,
        }
    }
}
impl Drop for ServerFixture {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn wait(app: &mut PlayerApplication, condition: impl Fn(&PlayerApplication) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut host = HostContext::new(app.initial_size());
    loop {
        app.advance(Duration::from_millis(20), &mut host).unwrap();
        if condition(app) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "online state: {}",
            app.online.as_ref().unwrap().status
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn playing(app: &PlayerApplication) -> bool {
    app.player.view().player_entity().is_some() && app.online.as_ref().unwrap().playing
}
#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn multiplayer_project_routes_graphical_player_to_demo_without_local_simulation() {
    let server = ServerFixture::start(false);
    let mut app =
        PlayerApplication::new(server.project.clone(), false, Box::new(StandardServices)).unwrap();
    assert!(app.player.is_remote());
    assert!(!app.player.runtime.is_started());
    assert!(app.player.runtime.server.instances.is_empty());
    assert!(app.screen.elements.game_widgets.is_empty());
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Demo)
    });
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    // A fast redraw loop must not flood WSS with repeated/empty input batches.
    // Poll networking at 240Hz without advancing the authority locally.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut previous = Instant::now();
    while Instant::now() < deadline {
        let now = Instant::now();
        app.send_online_inputs(now.duration_since(previous))
            .unwrap();
        previous = now;
        app.poll_online().unwrap();
        assert!(
            playing(&app),
            "high refresh disconnected: {}",
            app.online.as_ref().unwrap().status
        );
        std::thread::sleep(Duration::from_millis(4));
    }
    assert!(!app.screen.elements.game_widgets.is_empty());
    assert!(app.player.runtime.server.instances.is_empty());
    assert_eq!(app.player.runtime.tick_sequence(), 0);
    app.player
        .submit_action(rusterix::EntityAction::Intent("attack".into()))
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .view()
            .player_entity()
            .unwrap()
            .attributes
            .get_str("intent")
            == Some("attack")
    });
    // Existing HUD semantic actions cross the real socket and update only the
    // private remote replica. No local engine or prediction grants equipment.
    let actor = app.player.view().player_entity().unwrap().id;
    let (slot, item) = app
        .player
        .view()
        .player_entity()
        .unwrap()
        .equipped
        .iter()
        .next()
        .map(|(slot, item)| (slot.clone(), item.id))
        .expect("starter equipment");
    let free = app
        .player
        .view()
        .player_entity()
        .unwrap()
        .inventory
        .iter()
        .position(Option::is_none)
        .expect("free inventory slot");
    app.player
        .submit_action(rusterix::EntityAction::MoveItem {
            item_id: item,
            owner_entity_id: Some(actor),
            target_entity_id: Some(actor),
            to_inventory_index: Some(free),
            to_equipped_slot: None,
        })
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .view()
            .player_entity()
            .unwrap()
            .get_item(item)
            .is_some()
    });
    assert!(
        !app.player
            .view()
            .player_entity()
            .unwrap()
            .equipped
            .contains_key(&slot)
    );
    app.player
        .submit_action(rusterix::EntityAction::MoveItem {
            item_id: item,
            owner_entity_id: Some(actor),
            target_entity_id: Some(actor),
            to_inventory_index: None,
            to_equipped_slot: Some(slot.clone()),
        })
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .view()
            .player_entity()
            .unwrap()
            .equipped
            .get(&slot)
            .is_some_and(|i| i.id == item)
    });
    assert_eq!(app.player.runtime.tick_sequence(), 0);
    app.player
        .submit_action(rusterix::EntityAction::Forward)
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .remote()
            .unwrap()
            .authority()
            .motion()
            .unwrap()
            .processed_sequence
            > 0
    });
    assert!(app.player.remote().unwrap().diagnostics().predicted_steps > 0);
}
#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn multiplayer_project_account_lobby_creates_and_rejoins_a_saved_character() {
    let server = ServerFixture::start(true);
    check_saved_account_lobby(server.project.clone());
}
fn check_saved_account_lobby(project: Project) {
    let mut app = PlayerApplication::new(project, false, Box::new(StandardServices)).unwrap();
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Persistent)
    });
    assert!(
        app.screen
            .elements
            .text_input_widgets
            .values()
            .any(|w| w.binding == "online.password" && w.secret)
    );
    app.screen
        .elements
        .ui_state
        .insert("online.username".into(), "tester".into());
    app.screen
        .elements
        .ui_state
        .insert("online.password".into(), "account test password".into());
    app.command(ClientCommandBinding::Game("register".into()))
        .unwrap();
    assert!(!app.screen.elements.ui_state.contains_key("online.password"));
    wait(&mut app, |a| a.online.as_ref().unwrap().account);
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    let body = app.player.view().player_entity().unwrap().id;
    app.command(ClientCommandBinding::Game("connect".into()))
        .unwrap();
    wait(&mut app, |a| a.online.as_ref().unwrap().account);
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    assert_eq!(app.player.view().player_entity().unwrap().id, body);
    assert_eq!(app.online.as_ref().unwrap().characters.len(), 1);
    assert!(app.player.runtime.server.instances.is_empty());
}

#[test]
fn built_in_connection_screen_keeps_controls_inside_project_viewport() {
    let mut project = Project::default();
    project.config = "[viewport]\nwidth=960\nheight=540\ngrid_size=32".into();
    lobby::install(&mut project);
    let runtime = eldiron_runtime::GameSession::new(project);
    let layout = ScreenLayout {
        screen_responsive: true,
        reference_viewport: Vec2::new(960, 540),
        viewport: Vec2::new(960, 540),
        grid_size: 32.,
    };
    let screen = GameScreenController::from_screen(lobby::NAME, &runtime.view(), layout).unwrap();
    assert_eq!(screen.elements.text_input_widgets.len(), 3);
    for widget in screen.elements.text_input_widgets.values() {
        assert!(
            widget.rect.x >= 0.
                && widget.rect.y >= 0.
                && widget.rect.x + widget.rect.width <= 960.
                && widget.rect.y + widget.rect.height <= 540.,
            "{} {:?}",
            widget.name,
            widget.rect
        );
    }
    for widget in screen.elements.button_widgets.values() {
        assert!(widget.rect.y + widget.rect.height <= 540.);
    }
    assert!(
        screen
            .elements
            .text_input_widgets
            .values()
            .any(|w| w.binding == "online.password" && w.secret)
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn online_lobby_paints_masked_credentials_without_starting_a_world() {
    let mut project = Project::default();
    project.config="[game]\nmode='multiplayer'\n[multiplayer]\nendpoint='wss://example.invalid/play'\n[viewport]\nwidth=960\nheight=540\ngrid_size=32".into();
    let mut app = PlayerApplication::new(project, false, Box::new(StandardServices)).unwrap();
    app.screen
        .elements
        .ui_state
        .insert("online.password".into(), "private-password".into());
    let field = app
        .screen
        .elements
        .text_input_widgets
        .values_mut()
        .find(|w| w.binding == "online.password")
        .unwrap();
    field.text = "private-password".into();
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = theframework::render_wgpu::PaintRenderer::new(gpu.clone()).unwrap();
    let target =
        renderkit::GpuImage::new(&gpu, renderkit::ImageDescriptor::rgba(960, 540)).unwrap();
    let mut host = HostContext::new(app.initial_size());
    app.initialize(&mut host, &mut images).unwrap();
    super::render(&mut app, &mut host, &mut images, &mut renderer, &target);
    assert!(app.player.runtime.server.instances.is_empty());
    assert!(!app.player.runtime.is_started());
    super::capture(&gpu, target, "/tmp/eldiron-project-online-lobby.png");
}

#[test]
#[ignore = "requires localhost sockets and native GPU"]
fn multiplayer_streamed_region_draws_from_private_replica_without_local_server() {
    let server = ServerFixture::start(false);
    let mut app =
        PlayerApplication::new(server.project.clone(), false, Box::new(StandardServices)).unwrap();
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = theframework::render_wgpu::PaintRenderer::new(gpu.clone()).unwrap();
    let size = app.initial_size();
    let target =
        renderkit::GpuImage::new(&gpu, renderkit::ImageDescriptor::rgba(size.0, size.1)).unwrap();
    let mut host = HostContext::new(size);
    app.initialize(&mut host, &mut images).unwrap();
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Demo)
    });
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    super::render(&mut app, &mut host, &mut images, &mut renderer, &target);
    assert!(app.player.runtime.server.instances.is_empty());
    assert!(!app.player.runtime.is_started());
    assert!(
        !app.player.view().map.unwrap().geometry_objects.is_empty()
            || !app
                .player
                .view()
                .map
                .unwrap()
                .block_prop_instances
                .is_empty()
    );
    super::capture(&gpu, target, "/tmp/eldiron-spatial-online-scene.png");
}

// Only HTTP content requests wait; WebSocket bytes and input/heartbeats keep flowing.
async fn delayed_content_proxy(
    listener: std::net::TcpListener,
    target: std::net::SocketAddr,
    delay: Duration,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let mut jobs = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            peer = listener.accept() => {
                let (mut socket, _) = peer.unwrap();
                jobs.spawn(async move {
                    let mut header = Vec::new();
                    loop {
                        let mut byte = [0];
                        if socket.read_exact(&mut byte).await.is_err() {return;}
                        header.push(byte[0]);
                        if header.ends_with(b"\r\n\r\n") {break;}
                        if header.len() > 8192 {return;}
                    }
                    if header.starts_with(b"GET /content/") {
                        let mut unexpected = [0];
                        tokio::select! {
                            _ = tokio::time::sleep(delay) => {},
                            _ = socket.read(&mut unexpected) => return,
                        }
                    }
                    if let Ok(mut upstream) = tokio::net::TcpStream::connect(target).await {
                        if upstream.write_all(&header).await.is_ok() {let _ = tokio::io::copy_bidirectional(&mut socket, &mut upstream).await;}
                    }
                });
            },
            _ = jobs.join_next(), if !jobs.is_empty() => {},
        }
    }
}
#[test]
#[ignore = "60 second real graphical-client soak with localhost sockets; run with --ignored"]
fn multiplayer_project_delayed_content_soak_preserves_remote_authority_across_reconnects() {
    let server = ServerFixture::start_with_content_delay(false, Duration::from_secs(3));
    let mut app =
        PlayerApplication::new(server.project.clone(), false, Box::new(StandardServices)).unwrap();
    let started = Instant::now();
    let mut host = HostContext::new(app.initial_size());
    let mut joins = 0;
    let mut views = Vec::new();
    let mut ids = Vec::new();
    let mut joined = false;
    let mut peak_packets = 0;
    let mut peak_bytes = 0;
    let mut next_reconnect = Duration::from_secs(20);
    let mut requested = false;
    let mut previous = Instant::now();
    let mut last_status = String::new();
    while started.elapsed() < Duration::from_secs(60) {
        let now = Instant::now();
        app.advance(now.duration_since(previous), &mut host)
            .unwrap();
        previous = now;
        let status = &app.online.as_ref().unwrap().status;
        if status != &last_status {
            eprintln!(
                "SOAK {:.2}s status={status} joins={joins} diagnostics={:?}",
                started.elapsed().as_secs_f64(),
                app.online_content_diagnostics()
            );
            last_status = status.clone();
        }
        if let Some(d) = app.online_content_diagnostics() {
            peak_packets = peak_packets.max(d.peak_packets);
            peak_bytes = peak_bytes.max(d.peak_bytes);
            assert!(d.queued_packets <= 256 && d.queued_bytes <= 4 * 1024 * 1024);
        }
        if !requested && app.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Demo)
        {
            app.command(ClientCommandBinding::Game("start".into()))
                .unwrap();
            requested = true;
        }
        if playing(&app) {
            let id = app.player.view().player_entity().unwrap().id;
            if !joined {
                let view = app.player.view().session_id;
                assert!(
                    !views.contains(&view),
                    "reconnect retained old input/view incarnation"
                );
                views.push(view);
                // An empty demo resets its transient world, so numeric actor
                // IDs may repeat. Uniqueness is checked within each live view.
                ids.push(id);
                joins += 1;
                joined = true;
                app.player
                    .submit_action(rusterix::EntityAction::Intent("look".into()))
                    .unwrap();
            }
            let view = app.player.view();
            let actor = view.player_entity().unwrap();
            let name = actor.attributes.get_str("name").unwrap();
            assert_eq!(
                view.map
                    .as_ref()
                    .unwrap()
                    .entities
                    .iter()
                    .filter(|e| e.attributes.get_str("name") == Some(name))
                    .count(),
                1
            );
            assert_eq!(app.player.runtime.tick_sequence(), 0);
            assert!(app.player.runtime.server.instances.is_empty());
            if started.elapsed() >= next_reconnect && joins < 3 {
                app.command(ClientCommandBinding::Game("connect".into()))
                    .unwrap();
                requested = false;
                joined = false;
                next_reconnect += Duration::from_secs(20);
            }
        } else {
            let status = &app.online.as_ref().unwrap().status;
            assert!(
                !status.contains("fell behind")
                    && !status.contains("Disconnected")
                    && !status.contains("timed out"),
                "{status}"
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(playing(&app));
    assert_eq!(joins, 3);
    assert!(
        peak_packets > 64,
        "delayed content did not exercise a full old socket inbox: {peak_packets}"
    );
    println!(
        "NATIVE_CONTENT_SOAK {}",
        serde_json::json!({"elapsed_seconds":started.elapsed().as_secs_f64(),"joins":joins,"demo_entity_ids":ids,"distinct_view_incarnations":views.len(),"content_delay_ms":3000,"peak_packets":peak_packets,"peak_bytes":peak_bytes,"local_authority_ticks":app.player.runtime.tick_sequence()})
    );
}

#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn public_client_package_joins_demo_with_frozen_assets_and_no_authority() {
    let server = ServerFixture::start(false);
    let revision = eldiron_network::project_content_revision(&server.project).unwrap();
    let bytes = eldiron_network::public_client::export(&server.project).unwrap();
    let project = eldiron_network::public_client::load(&bytes).unwrap();
    assert_eq!(
        project
            .client_release
            .as_ref()
            .unwrap()
            .manifest
            .content_revision,
        revision
    );
    assert!(project.world_source.is_empty());
    assert!(
        project
            .regions
            .iter()
            .all(|r| r.characters.is_empty() && r.items.is_empty())
    );
    assert!(PlayerSession::new_local_test(project.clone(), true).is_err());
    let mut app = PlayerApplication::new(project, false, Box::new(StandardServices)).unwrap();
    assert!(app.player.is_remote());
    assert!(!app.player.runtime.is_started());
    let rules = app.player.runtime.assets.rules.clone();
    assert!(
        !rusterix::client::rules_ui::action_catalog(&app.player.runtime.assets, None).is_empty()
    );
    assert!(!app.player.runtime.assets.tiles.is_empty());
    assert!(!app.player.runtime.assets.procedural_avatars.is_empty());
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Demo)
    });
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    assert!(!app.screen.elements.game_widgets.is_empty());
    assert!(
        !app.player
            .view()
            .player_entity()
            .unwrap()
            .equipped
            .is_empty()
    );
    assert!(app.player.runtime.server.instances.is_empty());
    assert_eq!(app.player.runtime.tick_sequence(), 0);
    app.player
        .submit_action(rusterix::EntityAction::Intent("".into()))
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .view()
            .player_entity()
            .unwrap()
            .attributes
            .get_str("intent")
            == Some("")
    });
    assert_eq!(app.player.runtime.assets.rules, rules);
    let acknowledged = app
        .player
        .remote()
        .unwrap()
        .authority()
        .motion()
        .unwrap()
        .processed_sequence;
    app.player
        .submit_action(rusterix::EntityAction::Forward)
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .remote()
            .unwrap()
            .authority()
            .motion()
            .unwrap()
            .processed_sequence
            > acknowledged
    });
    assert!(app.player.remote().unwrap().diagnostics().predicted_steps > 0);
}

#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn public_client_package_preserves_saved_account_registration_and_rejoin() {
    let server = ServerFixture::start(true);
    let bytes = eldiron_network::public_client::export(&server.project).unwrap();
    check_saved_account_lobby(eldiron_network::public_client::load(&bytes).unwrap());
}

#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn streamed_public_release_defers_regions_and_keeps_remote_prediction() {
    let server = ServerFixture::start(false);
    let bundle = eldiron_network::public_client::export_streamed(&server.project).unwrap();
    let root = server.directory.join("public");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("packs")).unwrap();
    for (path, bytes) in &bundle.packs {
        std::fs::write(root.join(path), bytes).unwrap();
    }
    let path = root.join("game.eldiron-client");
    std::fs::write(&path, &bundle.bootstrap).unwrap();
    let project = eldiron_network::public_assets::load_path(&path).unwrap();
    assert!(
        project
            .regions
            .iter()
            .all(|r| r.map.geometry_objects.is_empty() && r.iso_paint.chunks.is_empty())
    );
    assert!(PlayerSession::new_local_test(project.clone(), true).is_err());
    let mut app = PlayerApplication::new(project, false, Box::new(StandardServices)).unwrap();
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Demo)
    });
    assert!(
        app.player
            .runtime
            .project
            .regions
            .iter()
            .all(|r| r.map.geometry_objects.is_empty())
    );
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    assert!(
        app.player
            .runtime
            .project
            .regions
            .iter()
            .any(|r| !r.map.geometry_objects.is_empty())
    );
    assert!(app.player.runtime.server.instances.is_empty());
    app.player
        .submit_action(rusterix::EntityAction::Intent("".into()))
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .view()
            .player_entity()
            .unwrap()
            .attributes
            .get_str("intent")
            == Some("")
    });
    let sequence = app
        .player
        .remote()
        .unwrap()
        .authority()
        .motion()
        .unwrap()
        .processed_sequence;
    app.player
        .submit_action(rusterix::EntityAction::Forward)
        .unwrap();
    wait(&mut app, |a| {
        a.player
            .remote()
            .unwrap()
            .authority()
            .motion()
            .unwrap()
            .processed_sequence
            > sequence
    });
    assert!(app.player.remote().unwrap().diagnostics().predicted_steps > 0);
}

#[test]
#[ignore = "requires localhost sockets; run with --ignored"]
fn account_settings_change_password_clear_secrets_and_rejoin_saved_body() {
    let mut p = Project::new();
    p.regions[0].name = "Account UI".into();
    p.regions[0].map.name = "Account UI".into();
    p.config="[viewport]\nwidth=960\nheight=540\ngrid_size=32\n[game]\nstart_region='Account UI'\nauto_create_player=false\n[ruleset]\nsource='project'".into();
    p.rules_document.current = String::new();
    p.rules_document.original = String::new();
    let mut c = shared::character::Character::new();
    c.name = "Hero".into();
    c.data = "[attributes]\nplayer=true\ninventory_slots=8".into();
    let mut instance = shared::character::Character::new();
    instance.character_id = c.id;
    p.regions[0].characters.insert(instance.id, instance);
    p.characters.insert(c.id, c);
    let server = ServerFixture::start_project(true, Duration::ZERO, p);
    let mut app =
        PlayerApplication::new(server.project.clone(), false, Box::new(StandardServices)).unwrap();
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Persistent)
    });
    app.screen
        .elements
        .ui_state
        .insert("online.username".into(), "ui_tester".into());
    app.screen
        .elements
        .ui_state
        .insert("online.password".into(), "original account password".into());
    app.command(ClientCommandBinding::Game("register".into()))
        .unwrap();
    wait(&mut app, |a| a.online.as_ref().unwrap().account);
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    let body = app.player.view().player_entity().unwrap().id;
    app.command(ClientCommandBinding::Game("account".into()))
        .unwrap();
    assert_eq!(app.screen.elements.text_input_widgets.len(), 3);
    assert!(
        app.screen
            .elements
            .text_input_widgets
            .values()
            .all(|w| w.secret)
    );
    app.screen.elements.ui_state.insert("online.current_password".into(), "discard on navigation".into());
    app.navigate("").unwrap();
    assert!(!app.screen.elements.ui_state.contains_key("online.current_password"));
    app.command(ClientCommandBinding::Game("account".into())).unwrap();
    for (key, value) in [
        ("online.current_password", "original account password"),
        ("online.new_password", "replacement account password"),
        ("online.confirm_password", "different account password"),
    ] {
        app.screen
            .elements
            .ui_state
            .insert(key.into(), value.into());
    }
    app.command(ClientCommandBinding::Game("change_password".into()))
        .unwrap();
    assert!(app.online.as_ref().unwrap().status.contains("do not match"));
    for key in [
        "online.current_password",
        "online.new_password",
        "online.confirm_password",
    ] {
        assert!(!app.screen.elements.ui_state.contains_key(key));
    }
    for (key, value) in [
        ("online.current_password", "original account password"),
        ("online.new_password", "replacement account password"),
        ("online.confirm_password", "replacement account password"),
    ] {
        app.screen
            .elements
            .ui_state
            .insert(key.into(), value.into());
    }
    app.command(ClientCommandBinding::Game("change_password".into()))
        .unwrap();
    wait(&mut app, |a| !a.online.as_ref().unwrap().account);
    assert!(
        app.online
            .as_ref()
            .unwrap()
            .status
            .contains("Password changed")
    );
    assert!(app.player.remote().is_none());
    app.command(ClientCommandBinding::Game("connect".into()))
        .unwrap();
    wait(&mut app, |a| {
        a.online.as_ref().unwrap().mode == Some(eldiron_network::HostMode::Persistent)
    });
    app.screen
        .elements
        .ui_state
        .insert("online.username".into(), "ui_tester".into());
    app.screen.elements.ui_state.insert(
        "online.password".into(),
        "replacement account password".into(),
    );
    app.command(ClientCommandBinding::Game("login".into()))
        .unwrap();
    wait(&mut app, |a| a.online.as_ref().unwrap().account);
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    wait(&mut app, playing);
    assert_eq!(app.player.view().player_entity().unwrap().id, body);
    assert_eq!(app.online.as_ref().unwrap().characters.len(), 1);
    app.command(ClientCommandBinding::Game("account".into()))
        .unwrap();
    app.command(ClientCommandBinding::Game("logout_all".into()))
        .unwrap();
    wait(&mut app, |a| !a.online.as_ref().unwrap().account);
    assert!(app.online.as_ref().unwrap().status.contains("All sessions"));
    assert!(app.player.runtime.server.instances.is_empty());
}
