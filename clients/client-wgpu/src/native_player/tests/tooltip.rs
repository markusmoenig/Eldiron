use super::*;
#[test]
#[ignore = "requires native GPU"]
fn tooltip_deadline_requests_one_frame_at_low_visual_fps() {
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(Arc::new(Mutex::new(Observation::default())))),
    )
    .unwrap();
    app.player.settings.target_fps = 1.;
    let mut host = HostContext::new((960, 540));
    host.resize((960, 540), 1.);
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 540)).unwrap();
    app.initialize(&mut host, &mut images).unwrap();
    assert!(app.update(&mut host, Duration::ZERO).redraw);
    let p = app.viewport().host_rect(Rect::new(400., 280., 1., 1.));
    app.route(
        &InputEvent::PointerMoved {
            position: [p.x, p.y],
        },
        &mut host,
    )
    .unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert_eq!(
        app.screen.presentation_wake_after(),
        Some(Duration::from_millis(650))
    );
    assert!(!app.update(&mut host, Duration::from_millis(649)).redraw);
    assert!(
        app.update(&mut host, Duration::from_millis(1)).redraw,
        "tooltip deadline precedes the one-second visual frame"
    );
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert!(app.screen.tooltip_bounds().is_some());
    assert!(
        !app.update(&mut host, Duration::from_millis(1)).redraw,
        "visible tooltip creates no repeated polling frames"
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(!app.player.runtime.server.has_local_player());
    app.player.runtime.stop();
}
