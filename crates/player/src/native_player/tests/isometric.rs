use super::*;
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn assembled_isometric_player_renders_shipped_sources_and_rebuilds_only_gpu_ownership() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/Hideout2D-MZ.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    let mut config = project.config.parse::<toml::Table>().unwrap();
    let game = config
        .entry("game")
        .or_insert(toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap();
    game.insert("start_screen".into(), "".into());
    game.insert("play_screen".into(), "".into());
    game.insert("auto_create_player".into(), false.into());
    let viewport = config
        .entry("viewport")
        .or_insert(toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap();
    viewport.insert("width".into(), 320.into());
    viewport.insert("height".into(), 240.into());
    viewport.insert("unit".into(), "pixel".into());
    viewport.insert("grid_size".into(), 24.into());
    project.config = toml::to_string(&config).unwrap();
    for region in &mut project.regions {
        region.map.camera = rusterix::MapCamera::TwoDIso;
    }
    let mut app = PlayerApplication::new(
        project,
        false,
        Box::new(Services(Arc::new(Mutex::new(Observation::default())))),
    )
    .unwrap();
    app.player.runtime.server.print_log_messages = false;
    // No player is registered in this offscreen source fixture. Aim at an
    // authored surface instead of the controller's empty-world origin.
    let map = app.player.runtime.view().map.unwrap();
    let focus = map
        .sectors
        .iter()
        .filter(|s| s.properties.get_default_source().is_some())
        .find_map(|s| s.center(map))
        .unwrap_or_else(|| map.bbox().center());
    for widget in app.screen.elements.game_widgets.values_mut() {
        widget.follow_player_camera = false;
        widget.camera = rusterix::PlayerCamera::D2Iso;
        widget.player_pos = focus;
        widget.clamp_camera_to_map = false;
    }

    assert!(
        !app.player.runtime.assets.tile_list.is_empty(),
        "session startup must publish indexed authored source tiles"
    );
    let incarnation = app.player.runtime.view().session_id;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let mut host = HostContext::new(app.initial_size());
    host.resize((480, 360), 1.5);
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(480, 360)).unwrap();
    app.initialize(&mut host, &mut images).unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host); // explicit offscreen-host acknowledgement
    assert!(app.failure().is_none());
    assert!(!app.screen.view_images().is_empty());
    assert!(
        app.screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.camera.is_isometric_2d()
                && w.buffer.pixels().is_empty()
                && w.upscale_buffer.pixels().is_empty())
    );
    let state = app.gpu.as_ref().unwrap();
    for output in app.screen.view_images().values() {
        assert!(!state.views.has_cpu_target_storage(output.view).unwrap());
        assert!(state.views.isometric_targets(*output).is_some());
    }
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu.transfers().capture_requests, 0);
    capture(
        &gpu,
        target.clone(),
        "/private/tmp/eldiron-native-isometric-player.png",
    );
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open("/private/tmp/eldiron-native-isometric-player.png").unwrap(),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    let colors = bytes[..info.buffer_size()]
        .chunks_exact(4)
        .map(|p| u32::from_le_bytes(p.try_into().unwrap()))
        .collect::<std::collections::HashSet<_>>();
    assert!(
        colors.len() > 8,
        "shipped scene must contain authored content, not just its background: {} colors",
        colors.len()
    );
    // Exercise the same assembled host through text presentation and back.
    let old_images = app
        .screen
        .view_images()
        .values()
        .map(|o| o.image)
        .collect::<Vec<_>>();
    let transfers = gpu.transfers();
    for widget in app.screen.elements.game_widgets.values_mut() {
        widget.backend_name = "text".into();
    }
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert!(app.screen.view_images().is_empty());
    assert!(old_images.into_iter().all(|id| images.get(id).is_none()));
    assert!(
        app.screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.text_font.is_none() && w.buffer.pixels().is_empty())
    );
    assert_eq!(gpu.transfers().readback_bytes, transfers.readback_bytes);
    assert_eq!(gpu.transfers().capture_requests, transfers.capture_requests);
    capture(
        &gpu,
        target.clone(),
        "/private/tmp/eldiron-native-text-game-player.png",
    );
    for widget in app.screen.elements.game_widgets.values_mut() {
        widget.backend_name = "graphical".into();
    }
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert!(!app.screen.view_images().is_empty());
    let second = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images2 = ImageRegistry::new(second.clone());
    let mut renderer2 = PaintRenderer::new(second.clone()).unwrap();
    let target2 = GpuImage::new(&second, ImageDescriptor::rgba(480, 360)).unwrap();
    app.gpu_recreated(&mut host, &mut images2).unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    assert_eq!(app.player.runtime.view().session_id, incarnation);
    assert!(app.failure().is_none());
    assert!(app.screen.view_images().values().all(|o| {
        app.gpu
            .as_ref()
            .unwrap()
            .views
            .isometric_targets(*o)
            .is_some()
    }));
    assert_eq!(second.transfers().readback_bytes, 0);
    assert_eq!(second.transfers().capture_requests, 0);
    capture(
        &second,
        target2,
        "/private/tmp/eldiron-native-isometric-player-recovered.png",
    );
    let mut recovered = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open("/private/tmp/eldiron-native-isometric-player-recovered.png").unwrap(),
    ))
    .read_info()
    .unwrap();
    let mut pixels = vec![0; recovered.output_buffer_size().unwrap()];
    let restored = recovered.next_frame(&mut pixels).unwrap();
    assert_eq!(
        &pixels[..restored.buffer_size()],
        &bytes[..info.buffer_size()],
        "rebuilding GPU ownership must reproduce the same shipped scene"
    );
}
