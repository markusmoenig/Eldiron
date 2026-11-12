use super::*;
use eldiron_runtime::RegionEvents;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket, wgpu};
use rusterix::{Map, Rect as ScreenRect, Value};
use rusterix::{
    audio::{
        AudioError,
        output::{AudioDevice, AudioLimits, AudioOutput},
    },
    server::message::AudioCommand,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use theframework::render_wgpu::PaintRenderer;
#[derive(Default)]
struct AudioObservation {
    created: usize,
    loads: Vec<String>,
    commands: Vec<AudioCommand>,
    suspensions: Vec<bool>,
    resets: usize,
}
struct RecordedAudio(Arc<Mutex<AudioObservation>>);
impl AudioDevice for RecordedAudio {
    fn healthy(&self) -> bool {
        true
    }
    fn ready(&self) -> bool {
        true
    }
    fn unlock(&mut self) -> std::result::Result<(), String> {
        Ok(())
    }
    fn set_suspended(&mut self, value: bool) -> std::result::Result<(), String> {
        self.0.lock().unwrap().suspensions.push(value);
        Ok(())
    }
    fn install(
        &mut self,
        name: &str,
        _: &rusterix::audio::PreparedClip,
        _: usize,
    ) -> std::result::Result<(), AudioError> {
        self.0.lock().unwrap().loads.push(name.into());
        Ok(())
    }
    fn unload(&mut self, _: &str) {}
    fn stop_loops(&mut self, _: &str) {}
    fn reset(&mut self) {
        self.0.lock().unwrap().resets += 1;
    }
    fn command(&mut self, command: &AudioCommand, _: usize) -> bool {
        self.0.lock().unwrap().commands.push(command.clone());
        true
    }
}
#[derive(Default)]
struct Observation {
    batches: usize,
    messages: usize,
    audio_forwarded: usize,
    effects: Vec<ScreenEffect>,
}
struct Services(Arc<Mutex<Observation>>);
impl PlayerServices for Services {
    fn region_events(&mut self, _: Uuid, _: &RuntimeView<'_>, events: RegionOutputs<'_>) {
        let mut o = self.0.lock().unwrap();
        o.batches += 1;
        o.messages += events.messages.len();
        o.audio_forwarded += events.audio.len();
    }
    fn screen_effect(
        &mut self,
        _: &RuntimeView<'_>,
        effect: &ScreenEffect,
    ) -> std::result::Result<(), String> {
        self.0.lock().unwrap().effects.push(effect.clone());
        Ok(())
    }
}
fn sector(map: &mut Map, id: u32, r: ScreenRect, data: &str) {
    let (sx, sy) = rusterix::utils::align_screen_to_grid(960., 540., 32.);
    let (x, y, w, h) = (
        r.x / 32. + sx,
        r.y / 32. + sy,
        r.width / 32.,
        r.height / 32.,
    );
    let points =
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|(x, y)| map.add_vertex_at(x, y));
    let lines = (0..4)
        .map(|i| map.create_linedef_manual(points[i], points[(i + 1) % 4]))
        .collect();
    let mut sector = rusterix::Sector::new(id, lines);
    sector.name = format!("Widget{id}");
    sector.properties.set("data", Value::Str(data.into()));
    map.sectors.push(sector);
}
fn starter_with_menu() -> Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let mut config = project.config.parse::<toml::Table>().unwrap();
    let game = config.get_mut("game").unwrap().as_table_mut().unwrap();
    game.insert("start_screen".into(), "Setup".into());
    game.insert("play_screen".into(), "Dungeon".into());
    project.config = toml::to_string(&config).unwrap();
    let id = Uuid::new_v4();
    let screen = project.screens.entry(id).or_default();
    screen.id = id;
    screen.map = Map::default();
    screen.map.name = "Setup".into();
    sector(
        &mut screen.map,
        1,
        ScreenRect::new(240., 200., 480., 40.),
        "[ui]\nrole='input'\nbind='start.name'\ndefault='Hero'\nfont_size=24.0",
    );
    sector(
        &mut screen.map,
        2,
        ScreenRect::new(240., 260., 480., 40.),
        "[ui]\nrole='button'\ncommand='game.start'\nlabel='Start'\nshow_icon=false",
    );
    project
}
fn render(
    app: &mut PlayerApplication,
    host: &mut HostContext,
    images: &mut ImageRegistry,
    renderer: &mut PaintRenderer,
    target: &GpuImage,
) {
    let list = app.frame(host, images).unwrap();
    list.validate().unwrap();
    renderer
        .render(
            &list,
            images,
            &target.view,
            target.descriptor.format,
            host.physical_size,
            host.density,
            [0., 0., 0., 1.],
        )
        .unwrap();
}
fn key(physical: PhysicalKey) -> InputEvent {
    InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    }
}

#[test]
fn audio_owner_revokes_old_session_loops_without_replacing_a_healthy_device_or_replaying_commands()
{
    let trace = Arc::new(Mutex::new(AudioObservation::default()));
    let factory = trace.clone();
    let audio = AudioOutput::with_preparer(
        AudioLimits::default(),
        move || {
            factory.lock().unwrap().created += 1;
            Ok(RecordedAudio(factory.clone()))
        },
        Box::new(rusterix::audio::preparation::InlinePreparation::default()),
    );
    let mut services = AudioServices::with_output(Services(Default::default()), audio);
    let mut first = eldiron_runtime::GameSession::new(Project::new());
    first.current_map = first.project.regions[0].map.name.clone();
    first.server.state = ServerState::Running;
    first.assets.audio.insert(
        "song".into(),
        rusterix::audio::synthesize_audio_fx_wav("[sfx.note]\nduration=0.01\n", "note")
            .unwrap()
            .into(),
    );
    let region = first.project.regions[0].map.id;
    let events = RegionEvents {
        audio: vec![AudioCommand::Play {
            name: "song".into(),
            bus: "music".into(),
            gain: 0.5,
            looping: true,
        }],
        ..Default::default()
    };
    services.update(&first.view(), 1, Duration::ZERO);
    services.region_events(region, &first.view(), events.outputs());
    assert_eq!(services.audio.desired_loops(), 1);
    let mut next = eldiron_runtime::GameSession::new(first.project.clone());
    next.current_map = first.current_map.clone();
    next.server.state = ServerState::Running;
    next.assets.audio = first.assets.audio.clone();
    assert_ne!(next.view().session_id, first.view().session_id);
    services.update(&next.view(), 1, Duration::ZERO);
    services.update(&next.view(), 1, Duration::ZERO);
    assert_eq!(services.audio.desired_loops(), 0);
    assert_eq!(trace.lock().unwrap().resets, 1);
    assert_eq!(
        trace
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| matches!(c, AudioCommand::Play { .. }))
            .count(),
        1
    );
    services.region_events(region, &next.view(), events.outputs());
    assert_eq!(trace.lock().unwrap().created, 1);
    assert_eq!(trace.lock().unwrap().loads.len(), 2);
    assert_eq!(
        trace
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| matches!(c, AudioCommand::Play { .. }))
            .count(),
        2
    );
}

#[test]
fn clipboard_result_before_next_frame_cannot_edit_a_stopped_runtime_incarnation() {
    use theframework::text_edit::EditorClipboard;
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(Arc::new(Mutex::new(Observation::default())))),
    )
    .unwrap();
    let mut host = HostContext::new(app.initial_size());
    app.screen.focus_input(&mut app.fonts, Some(1));
    let target = app.screen.text_input_state(&mut app.fonts).unwrap().target;
    app.screen.text_input_event(
        &mut app.fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    let result = app.screen.text_input_event(
        &mut app.fonts,
        &InputEvent::Key {
            physical: PhysicalKey::Letter(b'v'),
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        },
    );
    let Some(EditorClipboard::Read { request }) = result.editor.clipboard else {
        panic!("clipboard request")
    };
    app.pending_clipboard = Some((ScreenTextTarget::Input(1), request));
    app.player.runtime.stop();
    assert_ne!(
        app.player.runtime.view().session_id,
        app.screen.runtime_session()
    );
    assert!(
        !app.route(
            &InputEvent::Clipboard {
                request,
                result: theframework::platform::ClipboardResult::Text(Some("stale".into()))
            },
            &mut host
        )
        .unwrap()
        .consumed
    );
    assert!(app.pending_clipboard.is_none());
    assert_eq!(app.screen.elements.text_input_widgets[&1].text, "Hero");
    assert!(app.screen.text_input_state(&mut app.fonts).is_none());
    app.screen.focus_input(&mut app.fonts, Some(1));
    assert_ne!(
        app.screen.text_input_state(&mut app.fonts).unwrap().target,
        target
    );
}

#[test]
fn custom_cursor_host_state_restores_system_pointer_on_leave_and_missing_artwork() {
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(Arc::new(Mutex::new(Observation::default())))),
    )
    .unwrap();
    let mut host = HostContext::new(app.initial_size());
    let tile =
        rusterix::Tile::from_texture(rusterix::Texture::new([0, 255, 0, 255].repeat(4), 2, 2));
    let id = tile.id;
    app.player.runtime.assets.tiles.insert(id, tile);
    let mut config = app
        .player
        .runtime
        .project
        .config
        .parse::<toml::Table>()
        .unwrap();
    config
        .get_mut("viewport")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("cursor_id".into(), id.to_string().into());
    app.player.runtime.project.config = toml::to_string(&config).unwrap();
    app.revision.0 += 1;
    // Initialization establishes metrics before the host can deliver input.
    // This service-only case needs that lifecycle step but no GPU presenter.
    app.sync_host(&mut host).unwrap();
    app.route(
        &InputEvent::PointerMoved {
            position: [10., 10.],
        },
        &mut host,
    )
    .unwrap();
    assert!(!host.cursor_visible && app.screen.uses_custom_cursor());
    app.route(&InputEvent::PointerLeft, &mut host).unwrap();
    assert!(host.cursor_visible && !app.screen.uses_custom_cursor());
    app.player.runtime.assets.tiles.shift_remove(&id);
    app.route(
        &InputEvent::PointerMoved {
            position: [10., 10.],
        },
        &mut host,
    )
    .unwrap();
    assert!(host.cursor_visible && !app.screen.uses_custom_cursor());
    app.player.runtime.stop();
}

#[test]
#[ignore = "requires native GPU and real starter runtime"]
fn assembled_player_keeps_runtime_edits_and_clock_across_gpu_replacement() {
    let observed = Arc::new(Mutex::new(Observation::default()));
    let audio_trace = Arc::new(Mutex::new(AudioObservation::default()));
    let factory = audio_trace.clone();
    let audio = AudioOutput::with_preparer(
        AudioLimits::default(),
        move || {
            factory.lock().unwrap().created += 1;
            Ok(RecordedAudio(factory.clone()))
        },
        Box::new(rusterix::audio::preparation::InlinePreparation::default()),
    );
    let mut project = starter_with_menu();
    project
        .audio_fx
        .push_str("\n[sfx._rewrite_test_loop]\nwave='sine'\nduration=0.01\n");
    let mut imported = shared::asset::Asset::new();
    imported.name = "_rewrite_imported".into();
    imported.buffer = shared::asset::AssetBuffer::Audio(
        rusterix::audio::synthesize_audio_fx_wav("[sfx.sample]\nduration=0.01\n", "sample")
            .unwrap(),
    );
    project.assets.insert(imported.id, imported);
    let mut app = PlayerApplication::new(
        project,
        false,
        Box::new(AudioServices::with_output(
            Services(observed.clone()),
            audio,
        )),
    )
    .unwrap();
    app.player.runtime.server.print_log_messages = false;
    assert_eq!(app.screen.name, "Setup");
    assert!(!app.player.runtime.has_started_player());
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(1200, 900)).unwrap();
    let mut host = HostContext::new(app.initial_size());
    host.resize((1200, 900), 1.5);
    app.initialize(&mut host, &mut images).unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    let bounds = app.viewport().host_rect(Rect::new(250., 210., 1., 1.));
    app.input(
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position: [bounds.x, bounds.y],
        },
        &mut host,
    );
    app.input(
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: [bounds.x, bounds.y],
        },
        &mut host,
    );
    assert_eq!(app.screen.focused_input(), Some(1));
    app.input(
        &InputEvent::Modifiers(Modifiers {
            super_key: true,
            ..Default::default()
        }),
        &mut host,
    );
    app.input(&key(PhysicalKey::Letter(b'a')), &mut host);
    app.input(&InputEvent::Modifiers(Modifiers::default()), &mut host);
    app.input(&InputEvent::Text("Clæria 東京".into()), &mut host);
    assert_eq!(app.screen.elements.ui_state["start.name"], "Clæria 東京");
    assert!(host.text_input.is_some());
    // Clipboard completion is addressed to the originating document selection.
    app.input(
        &InputEvent::Modifiers(Modifiers {
            super_key: true,
            ..Default::default()
        }),
        &mut host,
    );
    app.input(&key(PhysicalKey::Letter(b'v')), &mut host);
    let request = host
        .requests
        .iter()
        .find_map(|r| {
            if let HostRequest::ClipboardRead { request } = r {
                Some(*request)
            } else {
                None
            }
        })
        .unwrap();
    app.input(
        &InputEvent::Clipboard {
            request,
            result: ClipboardResult::Text(Some("!".into())),
        },
        &mut host,
    );
    assert_eq!(app.screen.elements.ui_state["start.name"], "Clæria 東京!");
    app.input(&InputEvent::Modifiers(Modifiers::default()), &mut host);
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    // Suspend twice must not lose the original resume policy or advance time.
    app.suspended(&mut host);
    app.suspended(&mut host);
    let world = app.player.runtime.view();
    let player = world
        .map
        .unwrap()
        .entities
        .iter()
        .find(|e| e.is_player())
        .unwrap();
    app.screen.world_texts.receive(
        &world,
        &RegionEvents {
            world_text: vec![(
                player.id,
                rusterix::world_text::WorldText {
                    anchor: rusterix::world_text::TextAnchor::Position(player.position),
                    text: "Retained runtime description.".into(),
                },
            )],
            ..Default::default()
        },
    );
    let passage = app.screen.world_texts.passages()[0].id;
    let time = app.player.runtime.project.time;
    let session_id = app.player.runtime.view().session_id;
    assert_eq!(
        app.update(&mut host, Duration::from_secs(300)).wake_after,
        None
    );
    assert_eq!(app.player.runtime.project.time, time);
    assert_eq!(app.player.runtime.view().session_id, session_id);
    assert_eq!(app.screen.world_texts.passages()[0].id, passage);
    app.resumed(&mut host);
    assert!(app.player.runtime.server.state == ServerState::Running);
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    assert_eq!(app.screen.name, "Dungeon");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.update(&mut host, Duration::from_millis(20));
        assert!(app.failure().is_none(), "{:?}", app.failure());
        if app.player.runtime.server.has_local_player()
            && app
                .player
                .runtime
                .view()
                .map
                .unwrap()
                .entities
                .iter()
                .any(|e| e.is_player() && e.attributes.get_str("name") == Some("Clæria 東京!"))
        {
            break;
        }
        assert!(Instant::now() < deadline, "registration timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.player.binding_for_key("w").is_some());
    assert!(app.player.binding_for_key("a").is_some());
    app.input(&key(PhysicalKey::Letter(b'w')), &mut host);
    app.input(
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
        &mut host,
    );
    app.input(
        &InputEvent::Key {
            physical: PhysicalKey::Letter(b'w'),
            logical: LogicalKey::Other,
            pressed: false,
            repeat: false,
        },
        &mut host,
    );
    assert_eq!(
        app.player.input_stats().held_controls,
        0,
        "a modifier must not leave W held after release"
    );
    app.input(&InputEvent::Modifiers(Modifiers::default()), &mut host);
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    assert!(!app.screen.view_images().is_empty());
    let old = app
        .screen
        .view_images()
        .values()
        .copied()
        .collect::<Vec<_>>();
    app.input(&key(PhysicalKey::Letter(b'w')), &mut host);
    let receiver = app
        .player
        .runtime
        .view()
        .map
        .unwrap()
        .entities
        .iter()
        .find(|e| e.is_player())
        .unwrap()
        .id;
    let events = RegionEvents {
        messages: vec![(None, None, receiver, "".into(), "continue".into())],
        ..Default::default()
    };
    app.screen.receive(&app.player.runtime.view(), &events);
    app.update(&mut host, Duration::ZERO);
    assert!(app.screen.captures_world_input());
    assert_eq!(
        app.player.input_stats().held_controls,
        0,
        "an arriving modal must release movement even without another input event"
    );
    app.input(&key(PhysicalKey::Space), &mut host);
    assert!(!app.screen.captures_world_input());
    let forward = app
        .screen
        .elements
        .button_widgets
        .values()
        .find(|button| {
            matches!(
                button.command_binding(),
                Some(ClientCommandBinding::Control(
                    rusterix::EntityAction::Forward
                ))
            )
        })
        .unwrap()
        .rect;
    let pointer = app.viewport().host_rect(Rect::new(
        forward.x + forward.width / 2.,
        forward.y + forward.height / 2.,
        1.,
        1.,
    ));
    let before = app.player.input_stats().submitted_actions;
    app.input(
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position: [pointer.x, pointer.y],
        },
        &mut host,
    );
    assert!(app.screen.captures_world_input());
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before + 2,
        "press stops previous controls, then starts its held action without a second capture stop"
    );
    app.input(&key(PhysicalKey::Letter(b'w')), &mut host);
    assert_eq!(
        app.player.input_stats().held_controls,
        0,
        "a held mouse control captures keyboard movement"
    );
    app.input(
        &InputEvent::Key {
            physical: PhysicalKey::Letter(b'w'),
            logical: LogicalKey::Other,
            pressed: false,
            repeat: false,
        },
        &mut host,
    );
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before + 2,
        "an unaccepted keyboard release cannot interrupt a held mouse control"
    );
    app.input(
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: [-10., -10.],
        },
        &mut host,
    );
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before + 3,
        "outside release stops its held mouse control exactly once"
    );
    // The same runtime receives keyboard button actions exactly once, and a
    // prepare-time hide cancels its held source before the late key release.
    let forward_id = *app
        .screen
        .elements
        .button_widgets
        .iter()
        .find(|(_, b)| b.rect == forward)
        .unwrap()
        .0;
    app.screen.focus_control(
        &mut app.fonts,
        Some(eldiron_presentation::ScreenFocus::Button(forward_id)),
    );
    let before_keyboard = app.player.input_stats().submitted_actions;
    app.input(&key(PhysicalKey::Space), &mut host);
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before_keyboard + 2
    );
    app.input(
        &InputEvent::Key {
            physical: PhysicalKey::Space,
            logical: LogicalKey::Other,
            pressed: false,
            repeat: false,
        },
        &mut host,
    );
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before_keyboard + 3
    );
    app.input(&key(PhysicalKey::Space), &mut host);
    let before_hide = app.player.input_stats().submitted_actions;
    let forward_name = app.screen.elements.button_widgets[&forward_id].name.clone();
    app.screen
        .elements
        .widgets_to_hide
        .push(forward_name.clone());
    app.update(&mut host, Duration::ZERO);
    assert_eq!(app.player.input_stats().submitted_actions, before_hide + 1);
    app.update(&mut host, Duration::ZERO);
    assert_eq!(app.player.input_stats().submitted_actions, before_hide + 1);
    app.input(
        &InputEvent::Key {
            physical: PhysicalKey::Space,
            logical: LogicalKey::Other,
            pressed: false,
            repeat: false,
        },
        &mut host,
    );
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before_hide + 1,
        "cancelled source cannot stop twice or activate on its late release"
    );
    app.screen
        .elements
        .widgets_to_hide
        .retain(|name| name != &forward_name);
    let expected_geometry = app
        .screen
        .view_images()
        .values()
        .map(|output| {
            let vm = app
                .gpu
                .as_ref()
                .unwrap()
                .views
                .scene(output.view)
                .unwrap()
                .vm
                .active_vm();
            let count = vm.debug_stats().polys3d;
            assert!(
                count > 0,
                "real starter view must contain world polygons before device replacement"
            );
            (output.view, count)
        })
        .collect::<Vec<_>>();
    let fonts = app.fonts.version();
    let region = app.player.runtime.view().map.unwrap().id;
    let instance = app.player.runtime.server.region_id_map[&region];
    app.player
        .runtime
        .server
        .audio_commands
        .entry(instance)
        .or_default()
        .push(AudioCommand::Play {
            name: "_rewrite_test_loop".into(),
            bus: "music".into(),
            gain: 0.25,
            looping: true,
        });
    app.advance(Duration::ZERO, &mut host).unwrap();
    app.player
        .runtime
        .server
        .audio_commands
        .entry(instance)
        .or_default()
        .push(AudioCommand::Play {
            name: "_rewrite_imported".into(),
            bus: "sfx".into(),
            gain: 0.25,
            looping: false,
        });
    app.advance(Duration::ZERO, &mut host).unwrap();
    assert!(
        audio_trace
            .lock()
            .unwrap()
            .loads
            .iter()
            .any(|name| name == "_rewrite_imported")
    );
    let time = app.player.runtime.project.time;
    assert_eq!(
        audio_trace
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|c| matches!(c,AudioCommand::Play {name,..} if name=="_rewrite_test_loop"))
            .count(),
        1
    );
    assert_eq!(
        observed.lock().unwrap().audio_forwarded,
        0,
        "the inner service cannot consume the audio owner's commands a second time"
    );
    let audio_before_recreation = audio_trace.lock().unwrap().commands.clone();
    let speech = {
        let world = app.player.runtime.view();
        let player = world
            .map
            .unwrap()
            .entities
            .iter()
            .find(|e| e.is_player())
            .unwrap();
        app.screen.speech.receive(
            &world,
            &RegionEvents {
                says: vec![(
                    Some(player.id),
                    None,
                    "Speech retained through GPU recovery".into(),
                    "status".into(),
                )],
                ..Default::default()
            },
        );
        app.screen.speech.messages()[0].id
    };
    let input_before_recreation = app.player.input_stats();
    let gpu2 = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images2 = ImageRegistry::new(gpu2.clone());
    let mut renderer2 = PaintRenderer::new(gpu2.clone()).unwrap();
    let target2 = GpuImage::new(&gpu2, ImageDescriptor::rgba(1200, 900)).unwrap();
    app.gpu_recreated(&mut host, &mut images2).unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    assert_eq!(
        audio_trace.lock().unwrap().commands,
        audio_before_recreation
    );
    assert_eq!(
        audio_trace.lock().unwrap().created,
        1,
        "GPU recovery cannot replace the audio device"
    );
    assert_eq!(app.player.runtime.project.time, time);
    assert_eq!(app.fonts.version(), fonts);
    assert_eq!(app.screen.speech.messages()[0].id, speech);
    assert_eq!(
        app.player.input_stats(),
        input_before_recreation,
        "GPU recreation cannot replay input"
    );
    assert_eq!(app.screen.elements.ui_state["start.name"], "Clæria 東京!");
    assert!(app.player.runtime.server.has_local_player());
    assert!(old.iter().all(|o| images2.get(o.image).is_none()));
    let new = app
        .screen
        .view_images()
        .values()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(new.len(), old.len());
    for (view, count) in expected_geometry {
        let vm = app
            .gpu
            .as_ref()
            .unwrap()
            .views
            .scene(view)
            .unwrap()
            .vm
            .active_vm();
        assert_eq!(
            vm.debug_stats().polys3d,
            count,
            "device recovery must rebuild all loaded world geometry, not just a sky target"
        );
    }

    for output in &old {
        assert!(
            new.iter()
                .any(|n| n.view == output.view && n.image.owner != output.image.owner)
        );
    }
    assert_eq!(gpu2.transfers().readback_bytes, 0);
    assert!(
        app.screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.buffer.pixels().is_empty() && w.upscale_buffer.pixels().is_empty())
    );
    capture(
        &gpu2,
        target2.clone(),
        "/private/tmp/eldiron-native-player-world.png",
    );
    // Input uses an acknowledged scene receipt in the same host mapping as
    // paint. Isolate one visible actor in the real runtime's 2D GPU projection.
    let saved_texts = std::mem::take(&mut app.screen.world_texts);
    let saved_speech = std::mem::take(&mut app.screen.speech);
    app.screen.elements.button_widgets.clear();
    app.screen.elements.text_input_widgets.clear();
    app.screen.elements.choice_widgets.clear();
    app.screen.elements.messages_widgets.clear();
    app.screen.elements.profile_widgets.clear();
    app.screen.elements.avatar_widgets.clear();
    app.screen.elements.widgets_to_hide.clear();
    app.screen.intent.clear();
    let map = app
        .player
        .runtime
        .project
        .regions
        .iter_mut()
        .find(|r| r.map.name == app.player.runtime.current_map)
        .map(|r| &mut r.map)
        .unwrap();
    let actor = map
        .entities
        .iter()
        .find(|entity| entity.is_player())
        .unwrap()
        .id;
    map.entities.retain(|entity| entity.id == actor);
    map.items.clear();
    map.entities[0].set_attribute("visible", Value::Bool(true));
    let cell = map.entities[0].get_pos_xz().map(f32::floor) + Vec2::broadcast(0.5);
    let size = app.screen.elements.layout.viewport;
    for widget in app.screen.elements.game_widgets.values_mut() {
        widget.follow_player_camera = false;
        widget.set_camera_mode(rusterix::PlayerCamera::D2);
        widget.rect = ScreenRect::new(0., 0., size.x as f32, size.y as f32);
    }
    app.revision = ContentRevision(app.revision.0 + 1);
    app.screen.invalidate_world_input();
    let before_readback = gpu2.transfers().readback_bytes;
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    let widget = app.screen.elements.game_widgets.values().next().unwrap();
    let local = widget.ground_to_screen(cell);
    let point = app.viewport().host_rect(Rect::new(
        local.x + widget.rect.x,
        local.y + widget.rect.y,
        1.,
        1.,
    ));
    let point = [point.x, point.y];
    let before = app.player.input_stats().submitted_actions;
    for pressed in [true, false] {
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position: point,
            },
            &mut host,
        );
    }
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before,
        "preparation is not presentation acknowledgement"
    );
    app.presented(&mut host);
    app.input(&InputEvent::PointerMoved { position: point }, &mut host);
    assert_eq!(
        app.screen.world_pointer_hit().and_then(|hit| hit.entity),
        Some(actor)
    );
    for pressed in [true, false, false] {
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position: point,
            },
            &mut host,
        );
    }
    assert_eq!(
        app.player.input_stats().submitted_actions,
        before + 2,
        "one balanced world gesture submits one control cancellation and one click"
    );
    assert!(
        app.screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.buffer.pixels().is_empty() && w.upscale_buffer.pixels().is_empty())
    );
    assert_eq!(gpu2.transfers().readback_bytes, before_readback);
    // The common application also routes host relative motion once, through
    // its viewport, into the acknowledged first-person view and CPU session.
    // Device deltas retain their units regardless of that viewport's scale.
    for widget in app.screen.elements.game_widgets.values_mut() {
        widget.set_camera_mode(rusterix::PlayerCamera::D3FirstPMouse);
        widget.mouse_sensitivity = 0.5;
        widget.mouse_invert_y = false;
        widget.mouse_look_pitch = 0.;
    }
    app.revision.0 += 1;
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    let point = app.viewport.host_rect(Rect::new(400., 200., 1., 1.));
    app.input(
        &InputEvent::PointerMoved {
            position: [point.x, point.y],
        },
        &mut host,
    );
    let before = app.player.input_stats().submitted_actions;
    app.input(
        &InputEvent::RelativeMotion { delta: [10., -10.] },
        &mut host,
    );
    assert_eq!(app.player.input_stats().submitted_actions, before);
    app.presented(&mut host);
    let look = app
        .screen
        .mouse_look_view()
        .expect("acknowledged mouse-look view");
    let camera = app.screen.elements.game_widgets[&look]
        .camera_d3
        .view_matrix();
    app.input(
        &InputEvent::RelativeMotion { delta: [10., -10.] },
        &mut host,
    );
    assert_eq!(app.player.input_stats().submitted_actions, before + 1);
    let expected_pitch = 5.;
    assert!(
        (app.screen.elements.game_widgets[&look].mouse_look_pitch - expected_pitch).abs() < 0.001
    );
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    assert_ne!(
        app.screen.elements.game_widgets[&look]
            .camera_d3
            .view_matrix(),
        camera,
        "pitch reaches the prepared per-view camera"
    );
    assert_eq!(gpu2.transfers().readback_bytes, before_readback);
    app.screen.world_texts = saved_texts;
    app.screen.speech = saved_speech;
    // Repeated off-frame navigation retains view IDs, never old CPU screens.
    app.navigate("Setup").unwrap();
    assert_eq!(app.screen.world_texts.passages()[0].id, passage);
    assert!(app.screen.world_texts.visible_bounds().next().is_none());
    assert_eq!(app.screen.speech.messages()[0].id, speech);
    assert!(app.screen.speech.visible_bounds().next().is_none());
    assert_eq!(app.retired_views.len(), old.len());
    app.navigate("Dungeon").unwrap();
    app.navigate("Setup").unwrap();
    assert_eq!(app.screen.world_texts.passages()[0].id, passage);
    assert_eq!(app.retired_views.len(), old.len());
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    assert!(app.retired_views.is_empty());
    assert!(new.iter().all(|n| images2.get(n.image).is_none()));
    assert!(observed.lock().unwrap().batches > 0);
    // Explicit evidence capture after normal rendering's zero-readback checks.
    capture(
        &gpu2,
        target2,
        "/private/tmp/eldiron-native-player-menu.png",
    );
    // Fatal input/navigation errors must reach the host even when it cannot
    // acquire another drawable; presentation cannot be the only error channel.
    host.requests.clear();
    app.screen
        .elements
        .button_widgets
        .get_mut(&2)
        .unwrap()
        .command = Some("screen.goto.MissingScreen".into());
    let point = app.viewport().host_rect(Rect::new(300., 280., 1., 1.));
    for pressed in [true, false] {
        app.input(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position: [point.x, point.y],
            },
            &mut host,
        );
    }
    assert!(
        app.failure()
            .is_some_and(|error| error.contains("MissingScreen"))
    );
    assert_eq!(
        host.requests
            .iter()
            .filter(|request| matches!(request, HostRequest::Fail(_)))
            .count(),
        1
    );
    let frames = app.frame_count();
    host.resize((0, 0), 1.);
    app.update(&mut host, Duration::ZERO);
    assert_eq!(app.frame_count(), frames);
    assert_eq!(
        host.requests
            .iter()
            .filter(|request| matches!(request, HostRequest::Fail(_)))
            .count(),
        1
    );
    app.player.runtime.stop();
}

fn capture(gpu: &GpuContext, target: Arc<GpuImage>, path: &str) {
    let mut ticket = ReadbackTicket::capture(gpu, target.clone()).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = ticket.try_finish(gpu).unwrap().unwrap().into_pixels();
    let visible = pixels
        .chunks_exact(4)
        .filter(|p| p[0] > 30 || p[1] > 30 || p[2] > 30)
        .count();
    assert!(
        visible > 1000,
        "native player output is empty: {visible} nonblack pixels"
    );
    let mut png = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        target.descriptor.width,
        target.descriptor.height,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}
