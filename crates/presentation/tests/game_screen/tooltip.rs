use super::item_pointer::render;
use super::*;
use eldiron_presentation::ScreenEffect;
use rusterix::client::command::ClientCommandBinding;
use theframework::platform::{InputEvent, PointerButton};
fn route(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    views: &SceneViews,
    event: InputEvent,
) {
    let response = screen.input(&session.view(), fonts, &event).unwrap();
    let world = screen.world_input(
        &session.view(),
        Some(views),
        &event,
        response.input.consumed || screen.captures_world_input(),
    );
    assert!(response.effects.is_empty() && world.effects.is_empty());
}
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn tooltip_swash_wraps_multilingual_command_details_and_real_preferences_disable_it_without_cpu_composition()
 {
    let mut session = fixture(false);
    session.current_map = "No world".into();
    session.assets.rules=r#"
        [actions.arc]
        name = 'Arc 東京'
        kind = 'spell'
        description = 'A carefully aimed spell with a long explanation that must wrap using shared glyph metrics and preserve multilingual text.'
    "#.into();
    sector(
        &mut session.project.screens.values_mut().next().unwrap().map,
        20,
        Rect::new(16., 176., 180., 48.),
        "[ui]\nrole='button'\ncommand='rules.arc'\nlabel='Arc 東京'\nshow_icon=false",
    );
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen.set_tooltip_shortcuts(
        [("rules.arc".into(), vec!["A".into()])]
            .into_iter()
            .collect(),
    );
    let mut fonts = GameFonts::default();
    let time = session.project.time;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    macro_rules! frame {
        () => {
            render(
                &session,
                &mut screen,
                &mut fonts,
                &mut views,
                &mut registry,
                &mut images,
                &mut renderer,
                &target,
            )
        };
    }
    frame!();
    route(
        &session,
        &mut screen,
        &mut fonts,
        &views,
        InputEvent::PointerMoved {
            position: [90., 200.],
        },
    );
    frame!();
    assert!(screen.tooltip_bounds().is_none());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let before = capture(&gpu, target.clone());
    let readback = gpu.transfers().readback_bytes;
    assert!(!screen.advance_presentation(Duration::from_millis(649)));
    assert!(screen.advance_presentation(Duration::from_millis(1)));
    frame!();
    let bounds = screen.tooltip_bounds().unwrap();
    assert!(
        bounds.width <= 396.
            && bounds.height <= 236.
            && bounds.x >= 0.
            && bounds.y >= 0.
            && bounds.x + bounds.width <= 400.
            && bounds.y + bounds.height <= 240.
    );
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.tooltip_layout_builds(),
    );
    frame!();
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.tooltip_layout_builds()
        ),
        counts
    );
    assert_eq!(gpu.transfers().readback_bytes, readback);
    let pixels = capture(&gpu, target.clone());
    assert!(
        pixels
            .chunks_exact(4)
            .zip(before.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count()
            > 500,
        "tooltip changed actual GPU pixels"
    );
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-tooltip.png").unwrap(),
        600,
        360,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
    ));
    frame!();
    let button = |position, pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        position,
        pressed,
    };
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 66.], pressed),
        );
    }
    frame!();
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 130.], pressed),
        );
    }
    assert!(!screen.ui_preferences().tooltips_enabled);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
    ));
    route(
        &session,
        &mut screen,
        &mut fonts,
        &views,
        InputEvent::PointerMoved {
            position: [90., 200.],
        },
    );
    frame!();
    screen.advance_presentation(Duration::from_secs(2));
    frame!();
    assert!(screen.tooltip_bounds().is_none());
    assert_eq!(screen.presentation_wake_after(), None);
    // Re-enable through the real popup and select Instant. Its first visible
    // source should render immediately and then remain idle at the deadline.
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
    ));
    frame!();
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 66.], pressed),
        );
    }
    frame!();
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 98.], pressed),
        );
    }
    frame!();
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 102.], pressed),
        );
    }
    frame!();
    for pressed in [true, false] {
        route(
            &session,
            &mut screen,
            &mut fonts,
            &views,
            button([230., 88.], pressed),
        );
    }
    assert_eq!(screen.ui_preferences().tooltip_delay.milliseconds(), 0);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
    ));
    route(
        &session,
        &mut screen,
        &mut fonts,
        &views,
        InputEvent::PointerMoved {
            position: [90., 200.],
        },
    );
    frame!();
    assert!(screen.tooltip_bounds().is_some());
    assert!(!screen.advance_presentation(Duration::ZERO));
    assert!(session.project.time == time && session.server.state == rusterix::ServerState::Off);
}
