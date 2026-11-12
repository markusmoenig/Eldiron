use super::*;
use theframework::platform::{InputEvent, PointerButton};

#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn native_custom_cursor_composes_above_the_screen_at_fractional_dpi_and_reuses_its_source() {
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    let tile = Tile::from_texture(Texture::new([0, 255, 0, 255].repeat(60), 10, 6));
    let cursor = tile.id;
    session.assets.tiles.insert(cursor, tile);
    let mut config = session.project.config.parse::<toml::Table>().unwrap();
    config
        .get_mut("viewport")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("cursor_id".into(), cursor.to_string().into());
    session.project.config = toml::to_string(&config).unwrap();
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    let mut uploaded = 0;
    for frame in 0..3 {
        if frame > 0 {
            let point = [50. + frame as f32 - 1., 80.];
            screen
                .input(
                    &session.view(),
                    &mut fonts,
                    &InputEvent::PointerMoved { position: point },
                )
                .unwrap();
            screen.world_input(
                &session.view(),
                Some(&views),
                &InputEvent::PointerMoved { position: point },
                false,
            );
        }
        screen
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
        screen
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let list = {
            let mut source_frame = images.begin_frame(&mut registry).unwrap();
            screen
                .paint(&session.view(), &mut fonts, &mut source_frame)
                .unwrap()
        };
        renderer
            .render(
                &list,
                &registry,
                &target.view,
                target.descriptor.format,
                (600, 360),
                1.5,
                [0.; 4],
            )
            .unwrap();
        screen.acknowledge_world_frame(&session.view(), Some(&views));
        assert_eq!(screen.uses_custom_cursor(), frame > 0);
        if frame == 1 {
            uploaded = gpu.transfers().uploaded_bytes;
        }
        if frame == 2 {
            assert_eq!(
                gpu.transfers().uploaded_bytes,
                uploaded,
                "moving a cursor cannot re-upload source artwork"
            );
        }
    }
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(screen.elements.game_widgets.values().all(
        |widget| widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty()
    ));
    let pixels = capture(&gpu, target.clone());
    let point = (77, 120); // logical (51,80), rendered at density 1.5.
    let offset = (point.1 * 600 + point.0) * 4;
    assert_eq!(&pixels[offset..offset + 4], &[0, 255, 0, 255]);
    let file = std::fs::File::create("/private/tmp/eldiron-native-pointer-cursor.png").unwrap();
    let mut encoder = png::Encoder::new(file, 600, 360);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: [51., 80.],
            },
        )
        .unwrap();
    screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::PointerLeft,
        false,
    );
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let list = {
        let mut source_frame = images.begin_frame(&mut registry).unwrap();
        screen
            .paint(&session.view(), &mut fonts, &mut source_frame)
            .unwrap()
    };
    renderer
        .render(
            &list,
            &registry,
            &target.view,
            target.descriptor.format,
            (600, 360),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert!(!screen.uses_custom_cursor());
    let cleared = capture(&gpu, target);
    assert_ne!(
        &cleared[offset..offset + 4],
        &[0, 255, 0, 255],
        "leaving the screen removes the custom image"
    );
}
