use super::*;
#[test]
#[ignore = "requires native GPU"]
fn pregame_preferences_survive_navigation_without_registering_a_player_or_advancing_the_session_clock()
 {
    let observed = Arc::new(Mutex::new(Observation::default()));
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(observed.clone())),
    )
    .unwrap();
    // PlayerSession intentionally starts the CPU server before the menu; only
    // local-player registration is deferred. Paint/input must preserve it.
    let session_id = app.player.runtime.view().session_id;
    let state = app.player.runtime.server.state.clone();
    let until_tick = app.player.runtime.clock.until_tick();
    let time = app.player.runtime.project.time;
    let mut host = HostContext::new((960, 540));
    host.resize((960, 540), 1.);
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 540)).unwrap();
    app.initialize(&mut host, &mut images).unwrap();
    app.command(ClientCommandBinding::Ui("preferences".into()))
        .unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    let geometry = rusterix::client::screen_model::preferences_panel_geometry(
        &app.screen.elements.toolbar_preferences_panel_config,
        None,
        Vec2::new(960, 540),
    );
    let p = [
        geometry.tooltips.x + geometry.tooltips.width / 2.,
        geometry.tooltips.y + geometry.tooltips.height / 2.,
    ];
    let pointer =
        |app: &mut PlayerApplication, host: &mut HostContext, point: [f32; 2], pressed| {
            let bounds = app
                .viewport()
                .host_rect(Rect::new(point[0], point[1], 1., 1.));
            assert!(
                app.route(
                    &InputEvent::PointerButton {
                        button: PointerButton::Primary,
                        pressed,
                        position: [bounds.x, bounds.y]
                    },
                    host
                )
                .unwrap()
                .consumed
            );
        };
    pointer(&mut app, &mut host, p, true);
    pointer(&mut app, &mut host, p, false);
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    let off = [p[0], geometry.tooltips.y + geometry.tooltips.height * 2.5];
    pointer(&mut app, &mut host, off, true);
    pointer(&mut app, &mut host, off, false);
    assert!(!app.screen.ui_preferences().tooltips_enabled);
    app.navigate("Setup").unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert!(!app.screen.ui_preferences().tooltips_enabled);
    assert!(app.player.runtime.server.state == state);
    assert_eq!(app.player.runtime.view().session_id, session_id);
    assert_eq!(app.player.runtime.clock.until_tick(), until_tick);
    assert!(app.player.runtime.project.time == time);
    assert!(!app.player.runtime.server.has_local_player());
    assert!(observed.lock().unwrap().effects.is_empty());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    app.player.runtime.stop();
}
