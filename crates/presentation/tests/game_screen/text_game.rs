use super::*;
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_text_game_views_use_retained_swash_rows_and_retire_old_scene_handles() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&images, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    let mut sources = ScreenImageCache::default();
    let mut session = fixture(false);
    session.server.state = rusterix::ServerState::Running;
    let screen_map = &mut session.project.screens.values_mut().next().unwrap().map;
    let text = screen_map.sectors.iter_mut().find(|s| s.id == 4).unwrap();
    text.properties.set(
        "data",
        Value::Str(
            "[ui]\nrole='game'\npresentation='text'\nfont_size=18.0\ncolor='#e8d8b8'".into(),
        ),
    );
    let key = text.creator_id;
    let map = &mut session.project.regions[0].map;
    map.sectors[0].name = "Harbor Room".into();
    map.sectors[0].properties.set("data",Value::Str("[text]\ntitle='The Lantern Room'\ndescription='A long room description é 👩‍👩‍👧‍👦 東京 العربية with enough words to require trailing ellipsis.'".into()));
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let mut fonts = GameFonts::default();
    let mut counts = None;
    for frame in 0..3 {
        screen
            .prepare(&session.view(), &mut fonts, ContentRevision(1), frame)
            .unwrap();
        screen
            .prepare_game_views(&session.view(), &mut views, &mut images)
            .unwrap();
        assert!(screen.view_images().is_empty());
        let widget = &screen.elements.game_widgets[&key];
        assert!(widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty());
        assert!(widget.text_font.is_none());
        let mut source_frame = sources.begin_frame(&mut images).unwrap();
        let list = screen
            .paint(&session.view(), &mut fonts, &mut source_frame)
            .unwrap();
        drop(source_frame);
        list.validate().unwrap();
        assert!(
            list.ops
                .iter()
                .any(|p| matches!(p, PaintOp::GlyphRun { .. }))
        );
        renderer
            .render(
                &list,
                &images,
                &target.view,
                target.descriptor.format,
                (600, 360),
                1.5,
                [0.; 4],
            )
            .unwrap();
        let current = (
            screen.text_game_layout_builds(),
            renderer.glyphs.rasterizations,
        );
        if frame == 0 {
            counts = Some(current);
        } else {
            assert_eq!(Some(current), counts);
        }
        assert_eq!(gpu.transfers().readback_bytes, 0);
    }
    assert_eq!(gpu.transfers().capture_requests, 0);
    let pixels = capture(&gpu, target.clone());
    let at = (80 * 600 + 26) * 4;
    assert_eq!(&pixels[at..at + 4], &[0, 0, 0, 255]);
    let visible = (90..240)
        .flat_map(|y| (42..560).map(move |x| (y * 600 + x) * 4))
        .filter(|at| pixels[*at..*at + 3].iter().copied().max().unwrap() > 180)
        .count();
    assert!(
        visible > 100,
        "the actual room glyphs must be visible: {visible} bright pixels"
    );

    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-text-game.png").unwrap(),
        600,
        360,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    let font = theframework::Embedded::get("fonts/Roboto-Bold.ttf").unwrap();
    fonts
        .text_mut()
        .register_font_named("Roboto", font.data.to_vec());
    screen.elements.game_widgets.get_mut(&key).unwrap().upscale = 1.5;
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 3)
        .unwrap();
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    let mut source_frame = sources.begin_frame(&mut images).unwrap();
    let enlarged = screen
        .paint(&session.view(), &mut fonts, &mut source_frame)
        .unwrap();
    drop(source_frame);
    enlarged.validate().unwrap();
    renderer
        .render(
            &enlarged,
            &images,
            &target.view,
            target.descriptor.format,
            (600, 360),
            1.5,
            [0.; 4],
        )
        .unwrap();
    let enlarged = capture(&gpu, target.clone());
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-text-game-upscaled.png").unwrap(),
        600,
        360,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&enlarged)
        .unwrap();
    assert!(screen.text_game_layout_builds() > counts.unwrap().0);
    screen
        .elements
        .game_widgets
        .get_mut(&key)
        .unwrap()
        .toml_str
        .push_str("\n[say]\nwarning='#22dd88ff'\nbackground_enabled=false\n");
    screen.receive(
        &session.view(),
        &RegionEvents {
            says: vec![(Some(1), None, "A lamp is burning.".into(), "warning".into())],
            ..Default::default()
        },
    );
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 4)
        .unwrap();
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    let list = {
        let mut frame = sources.begin_frame(&mut images).unwrap();
        screen
            .paint(&session.view(), &mut fonts, &mut frame)
            .unwrap()
    };
    let green = [34., 221., 136., 255.].map(|v| v / 255.);
    assert!(
        list.ops
            .iter()
            .any(|p| matches!(p,PaintOp::GlyphRun{color,..} if *color==green)),
        "reference speech must remain above text game views"
    );
    renderer
        .render(
            &list,
            &images,
            &target.view,
            target.descriptor.format,
            (600, 360),
            1.5,
            [0.; 4],
        )
        .unwrap();
    let speech = capture(&gpu, target);
    assert!(
        speech
            .chunks_exact(4)
            .filter(|p| p[1] > 150 && u16::from(p[1]) > u16::from(p[0]) * 2 && p[2] > 80)
            .count()
            > 20
    );
    screen
        .elements
        .game_widgets
        .get_mut(&key)
        .unwrap()
        .backend_name = "graphical".into();
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    let old = *screen.view_images().get(&key).unwrap();
    assert!(images.get(old.image).is_some());
    screen
        .elements
        .game_widgets
        .get_mut(&key)
        .unwrap()
        .backend_name = "text".into();
    screen
        .elements
        .game_widgets
        .get_mut(&key)
        .unwrap()
        .rect
        .width = 20.;
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    assert!(screen.view_images().is_empty());
    assert!(images.get(old.image).is_none());
    let mut source_frame = sources.begin_frame(&mut images).unwrap();
    screen
        .paint(&session.view(), &mut fonts, &mut source_frame)
        .unwrap()
        .validate()
        .unwrap();
}
