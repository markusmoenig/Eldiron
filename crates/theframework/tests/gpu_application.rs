#![cfg(all(feature = "render-wgpu", target_os = "macos"))]
//! Foreign-layer application contract deliberately builds without ui or winit.
use renderkit::*;
use theframework::{
    paint::{PaintList, Rect},
    platform::*,
    text::{TextOptions, TextSystem},
    text_edit::{EditorColors, EditorView},
};
struct Application {
    fonts: TextSystem,
    input: EditorView,
    initializations: usize,
    recreations: usize,
}
impl Application {
    fn layout(&mut self, host: &mut HostContext) {
        self.input.prepare(
            &mut self.fonts,
            TextOptions {
                family: "Roboto",
                size: 18.,
                width: None,
                line_height: None,
            },
            Rect::new(12., 12., host.logical_size[0] - 24., 40.),
            true,
        );
        host.text_input = self.input.ime_area(&mut self.fonts);
    }
}
impl GpuApplication for Application {
    fn initialize(&mut self, host: &mut HostContext, _: &mut ImageRegistry) -> Result<()> {
        self.initializations += 1;
        self.fonts
            .register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        self.input
            .editor
            .set_text(&mut self.fonts, "Rust + Xcode 東京");
        self.input.set_focused(&mut self.fonts, true);
        self.layout(host);
        Ok(())
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, _: &mut ImageRegistry) -> Result<()> {
        self.recreations += 1;
        self.layout(host);
        Ok(())
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.layout(host);
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        let response = self.input.input(&mut self.fonts, event);
        host.text_input = self.input.ime_area(&mut self.fonts);
        response.input
    }
    fn frame(&mut self, host: &mut HostContext, _: &mut ImageRegistry) -> Result<PaintList> {
        let mut list = PaintList::default();
        list.rect(
            Rect::new(0., 0., host.logical_size[0], host.logical_size[1]),
            0.,
            [0.04, 0.08, 0.12, 1.],
        );
        self.input
            .paint(&mut self.fonts, &mut list, EditorColors::default(), true);
        Ok(list)
    }
}
fn create_presenter(
    layer: &objc2_quartz_core::CAMetalLayer,
    size: (u32, u32),
) -> GpuPresenter<'static> {
    let pointer = (layer as *const objc2_quartz_core::CAMetalLayer)
        .cast_mut()
        .cast();
    let surface =
        futures::executor::block_on(unsafe { SurfaceState::from_metal_layer(pointer, size) })
            .unwrap();
    GpuPresenter::new(surface).unwrap()
}
#[test]
#[ignore = "requires native Metal; run with --ignored"]
fn foreign_layer_hosts_widget_free_app_and_preserves_state_on_surface_replacement() {
    let layer = objc2_quartz_core::CAMetalLayer::new();
    let mut presenter = create_presenter(&layer, (480, 180));
    let mut app = Application {
        fonts: TextSystem::default(),
        input: EditorView::new(false),
        initializations: 0,
        recreations: 0,
    };
    let mut host = HostContext::new((320, 120));
    host.resize((480, 180), 1.5);
    app.initialize(&mut host, &mut presenter.images).unwrap();
    app.input(&InputEvent::Text("!".into()), &mut host);
    assert_eq!(app.input.editor.text(), "Rust + Xcode 東京!");
    let builds = app.input.editor.snapshots_built();
    for _ in 0..2 {
        let list = app.frame(&mut host, &mut presenter.images).unwrap();
        list.validate().unwrap();
        assert_eq!(
            presenter.present(&list, host.density, [0.; 4]).unwrap(),
            PresentResult::Presented
        );
        assert_eq!(presenter.context().transfers().readback_bytes, 0);
    }
    assert_eq!(app.input.editor.snapshots_built(), builds);
    assert!(host.text_input.is_some());
    presenter.surface.suspend();
    let list = app.frame(&mut host, &mut presenter.images).unwrap();
    assert_eq!(
        presenter.present(&list, host.density, [0.; 4]).unwrap(),
        PresentResult::Deferred
    );
    drop(presenter);
    let mut replacement = create_presenter(&layer, (640, 240));
    host.resize((640, 240), 2.);
    app.gpu_recreated(&mut host, &mut replacement.images)
        .unwrap();
    assert_eq!(app.initializations, 1);
    assert_eq!(app.recreations, 1);
    assert_eq!(app.input.editor.text(), "Rust + Xcode 東京!");
    let list = app.frame(&mut host, &mut replacement.images).unwrap();
    assert_eq!(
        replacement.present(&list, host.density, [0.; 4]).unwrap(),
        PresentResult::Presented
    );
    app.input(&InputEvent::Focus(false), &mut host);
    assert!(host.text_input.is_none());
    drop(replacement);
    drop(layer);
}
