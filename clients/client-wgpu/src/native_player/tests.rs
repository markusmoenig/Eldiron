use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket, wgpu};
use rusterix::{Map, Rect as ScreenRect, Value};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use theframework::render_wgpu::PaintRenderer;
#[derive(Default)]
struct Observation {
    batches: usize,
    messages: usize,
    effects: Vec<ScreenEffect>,
}
struct Services(Arc<Mutex<Observation>>);
impl PlayerServices for Services {
    fn region_events(&mut self, _: Uuid, _: &RuntimeView<'_>, events: &RegionEvents) {
        let mut o = self.0.lock().unwrap();
        o.batches += 1;
        o.messages += events.messages.len();
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
#[ignore = "requires native GPU and real starter runtime"]
fn assembled_player_keeps_runtime_edits_and_clock_across_gpu_replacement() {
    let observed = Arc::new(Mutex::new(Observation::default()));
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(observed.clone())),
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
    assert_eq!(
        app.update(&mut host, Duration::from_secs(300)).wake_after,
        None
    );
    assert_eq!(app.player.runtime.project.time, time);
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
    let time = app.player.runtime.project.time;
    let fonts = app.fonts.version();
    let input_before_recreation = app.player.input_stats();
    let gpu2 = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images2 = ImageRegistry::new(gpu2.clone());
    let mut renderer2 = PaintRenderer::new(gpu2.clone()).unwrap();
    let target2 = GpuImage::new(&gpu2, ImageDescriptor::rgba(1200, 900)).unwrap();
    app.gpu_recreated(&mut host, &mut images2).unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    assert_eq!(app.player.runtime.project.time, time);
    assert_eq!(app.fonts.version(), fonts);
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
    // Repeated off-frame navigation retains view IDs, never old CPU screens.
    app.navigate("Setup").unwrap();
    assert_eq!(app.screen.world_texts.passages()[0].id, passage);
    assert!(app.screen.world_texts.visible_bounds().next().is_none());
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
