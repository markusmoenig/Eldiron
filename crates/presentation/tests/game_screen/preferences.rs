use super::item_pointer::render;
use super::*;
use eldiron_presentation::ScreenEffect;
use rusterix::client::command::ClientCommandBinding;
use theframework::platform::{InputEvent, PointerButton};
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn preferences_compose_on_a_worldless_menu_with_retained_swash_and_acknowledged_popup_selection() {
    let mut session = fixture(false);
    session.current_map = "No selected world".into();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("preferences".into()))
    ));
    let mut fonts = GameFonts::default();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    let button = |position, pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        position,
        pressed,
    };
    for pressed in [true, false] {
        assert!(
            screen
                .input(&session.view(), &mut fonts, &button([230., 66.], pressed))
                .unwrap()
                .effects
                .is_empty()
        );
    }
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.source_builds(),
    );
    assert!(counts.1 > 0);
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = capture(&gpu, target.clone());
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-preferences-hud.png").unwrap(),
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
    for pressed in [true, false] {
        assert!(
            screen
                .input(&session.view(), &mut fonts, &button([230., 130.], pressed))
                .unwrap()
                .effects
                .is_empty()
        );
    }
    assert!(!screen.ui_preferences().tooltips_enabled);
    assert_eq!(screen.elements.ui_state["preferences.tooltips"], "off");
    assert!(session.view().map.is_none());
    assert!(session.server.state == rusterix::ServerState::Off);
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.source_builds(),
    );
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts
    );
}
