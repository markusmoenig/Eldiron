#![cfg(all(feature = "ui", feature = "render-wgpu"))]
use renderkit::*;
use theframework::{
    paint::PaintOp, prelude::*, render_wgpu::PaintRenderer, theui::paint::UiImages,
};
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn real_ui_tree_has_no_canvas_framebuffer() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut ctx = TheContext::new(480, 160, 1.5);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    let mut layout = TheHLayout::new(TheId::named("toolbar"));
    let mut label = TheText::new(TheId::named("label"));
    label.set_text("Native Eldiron UI".into());
    label.set_text_size(18.);
    let mut button = TheTraybarButton::new(TheId::named("run"));
    button.set_text("Run".into());
    layout.widgets().push(Box::new(label));
    layout.widgets().push(Box::new(button));
    ui.canvas.set_layout(layout);
    let first = ui.paint_gpu(&mut ctx, &mut images).unwrap();
    assert!(ui.canvas.buffer.pixels().is_empty());
    assert!(
        first
            .ops
            .iter()
            .any(|p| matches!(p, PaintOp::GlyphRun { .. }))
    );
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(720, 240)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &first,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (720, 240),
            1.5,
            [0.; 4],
        )
        .unwrap();
    let uploads = gpu.transfers().uploaded_bytes;
    let rasterizations = renderer.glyphs.rasterizations;
    let second = ui.paint_gpu(&mut ctx, &mut images).unwrap();
    renderer
        .render(
            &second,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (720, 240),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    assert_eq!(renderer.glyphs.rasterizations, rasterizations);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    ctx.width = 320;
    ctx.height = 120;
    ctx.ui.relayout = true;
    ui.paint_gpu(&mut ctx, &mut images).unwrap();
    assert!(ui.canvas.buffer.pixels().is_empty());
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = capture.try_finish(&gpu).unwrap().unwrap();
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-ui.png").unwrap();
    let mut encoder = png::Encoder::new(file, 720, 240);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels.pixels())
        .unwrap();
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn gpu_editors_keep_text_ime_undo_and_grapheme_input_separate() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((640, 360), 1.5);
    let id = TheId::named("name");
    let mut edit = TheTextLineEdit::new(id.clone());
    edit.set_text("".into());
    edit.limiter_mut().set_max_width(600);
    let mut layout = TheVLayout::new(TheId::named("editor-layout"));
    layout.widgets().push(Box::new(edit));
    let mut area = TheTextAreaEdit::new(TheId::named("code"));
    area.set_text("let title = \"Eldiron\";\n// GPU text: العربية café\n".into());
    area.as_code_editor("Rust", TheCodeEditorSettings::default());
    area.limiter_mut().set_max_width(600);
    area.limiter_mut().set_max_height(220);
    layout.widgets().push(Box::new(area));
    session.ui.canvas.set_layout(layout);
    session.paint(&mut images).unwrap();
    session.context.ui.set_focus(&id);
    session.ui.update(&mut session.context);
    let key = InputEvent::Key {
        physical: PhysicalKey::Letter(b'A'),
        logical: LogicalKey::Character("a".into()),
        pressed: true,
        repeat: false,
    };
    assert!(session.input(&key).consumed);
    assert_eq!(session.ui.get_text_line_edit("name").unwrap().text(), "");
    session.input(&InputEvent::Composition {
        text: "日本".into(),
        cursor: Some((0, 6)),
    });
    assert_eq!(session.ui.get_text_line_edit("name").unwrap().text(), "");
    session.paint(&mut images).unwrap();
    assert!(
        session
            .input(&InputEvent::Text("e\u{301}日本".into()))
            .consumed
    );
    assert_eq!(
        session.ui.get_text_line_edit("name").unwrap().text(),
        "e\u{301}日本"
    );
    session.paint(&mut images).unwrap();
    session.input(&InputEvent::Key {
        physical: PhysicalKey::Backspace,
        logical: LogicalKey::Named(TheKeyCode::Delete),
        pressed: true,
        repeat: false,
    });
    assert_eq!(
        session.ui.get_text_line_edit("name").unwrap().text(),
        "e\u{301}日"
    );
    session.input(&InputEvent::Modifiers(Modifiers {
        control: true,
        ..Default::default()
    }));
    session.input(&InputEvent::Key {
        physical: PhysicalKey::Letter(b'Z'),
        logical: LogicalKey::Character("z".into()),
        pressed: true,
        repeat: false,
    });
    assert_eq!(
        session.ui.get_text_line_edit("name").unwrap().text(),
        "e\u{301}日本"
    );
    session.input(&InputEvent::Key {
        physical: PhysicalKey::Letter(b'Z'),
        logical: LogicalKey::Character("z".into()),
        pressed: true,
        repeat: false,
    });
    assert_eq!(session.ui.get_text_line_edit("name").unwrap().text(), "");
    session.input(&InputEvent::Modifiers(Modifiers::default()));
    session.input(&InputEvent::Text("Native · e\u{301} · 日本".into()));
    let list = session.paint(&mut images).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 540)).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (960, 540),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert!(session.ui.canvas.buffer.pixels().is_empty());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = capture.try_finish(&gpu).unwrap().unwrap();
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-editors.png").unwrap();
    let mut encoder = png::Encoder::new(file, 960, 540);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels.pixels())
        .unwrap();
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn scrolled_form_input_uses_inverse_paint_transform() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu);
    let mut session = UiSession::new((360, 100), 1.);
    let mut form = TheTextLayout::new(TheId::named("form"));
    for i in 0..10 {
        let mut edit = TheTextLineEdit::new(TheId::named(&format!("field-{i}")));
        edit.set_text("abcd".into());
        form.add_pair(format!("Field {i}"), Box::new(edit));
    }
    session.ui.canvas.set_layout(form);
    session.paint(&mut images).unwrap();
    session
        .ui
        .get_text_layout("form")
        .unwrap()
        .set_scroll_offset(60);
    session.paint(&mut images).unwrap();
    let id = session.ui.get_widget("field-2").unwrap().id().clone();
    let dim = *session.ui.get_widget("field-2").unwrap().dim();
    let pointer = [dim.x as f32 + 8., dim.y as f32 - 60. + 10.];
    assert!(pointer[1] >= 0. && pointer[1] < 100.);
    let local = session.context.ui.local_position(
        &id,
        &dim,
        Vec2::new(pointer[0] as i32, pointer[1] as i32),
    );
    assert_eq!(local, Vec2::new(8, 10));
    let response = session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: pointer,
    });
    assert!(response.consumed);
    assert_eq!(
        session.context.ui.focus.as_ref().map(|i| i.uuid),
        Some(id.uuid)
    );
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: pointer,
    });
    session.input(&InputEvent::Text("X".into()));
    let text = session.ui.get_text_line_edit("field-2").unwrap().text();
    assert!(
        text.starts_with('X'),
        "hit at the left of the visible editor must place its caret at the start: {text}"
    );
    assert!(session.ui.canvas.buffer.pixels().is_empty());
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn tree_embedded_editor_and_dropdown_overlay_share_geometry() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((420, 200), 1.);
    let mut tree = TheTreeLayout::new(TheId::named("tree"));
    tree.set_headerless(true);
    tree.get_root().set_open(true);
    let mut row = TheTreeItem::new(TheId::named("row"));
    row.set_text("Name".into());
    let mut edit = TheTextLineEdit::new(TheId::named("column"));
    edit.set_text("abcd".into());
    row.add_widget_column(180, Box::new(edit));
    tree.get_root().add_widget(Box::new(row));
    let mut choice = TheTreeItem::new(TheId::named("choice"));
    choice.set_text("Choice".into());
    let mut dropdown = TheDropdownMenu::new(TheId::named("dropdown"));
    dropdown.add_option("First".into());
    dropdown.add_option("Second".into());
    choice.add_widget_column(180, Box::new(dropdown));
    tree.get_root().add_widget(Box::new(choice));
    session.ui.canvas.set_layout(tree);
    session.paint(&mut images).unwrap();
    let dim = *session.ui.get_widget("row").unwrap().dim();
    let position = [dim.x as f32 + dim.width as f32 - 168., dim.y as f32 + 10.];
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position,
    });
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position,
    });
    session.input(&InputEvent::Text("X".into()));
    let value = session
        .ui
        .get_widget("row")
        .unwrap()
        .as_tree_item()
        .unwrap()
        .embedded_widget_mut()
        .unwrap()
        .value();
    assert_eq!(value.to_string().unwrap(), "Xabcd");
    session.input(&InputEvent::Modifiers(Modifiers {
        control: true,
        ..Default::default()
    }));
    session.input(&InputEvent::Key {
        physical: PhysicalKey::Letter(b'Z'),
        logical: LogicalKey::Character("z".into()),
        pressed: true,
        repeat: false,
    });
    session.input(&InputEvent::Modifiers(Modifiers::default()));
    assert_eq!(
        session
            .ui
            .get_widget("row")
            .unwrap()
            .as_tree_item()
            .unwrap()
            .embedded_widget_mut()
            .unwrap()
            .value()
            .to_string()
            .unwrap(),
        "abcd"
    );
    let dim = *session.ui.get_widget("choice").unwrap().dim();
    let position = [dim.x as f32 + dim.width as f32 - 100., dim.y as f32 + 10.];
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position,
    });
    let list = session.paint(&mut images).unwrap();
    assert!(session.context.ui.overlay.is_some());
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(420, 200)).unwrap();
    let mut renderer = PaintRenderer::new(gpu).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (420, 200),
            1.,
            [0.; 4],
        )
        .unwrap();
    session.input(&InputEvent::PointerMoved {
        position: [position[0], dim.y as f32 + 45.],
    });
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: [position[0], dim.y as f32 + 45.],
    });
    assert!(session.context.ui.overlay.is_none());
    assert_eq!(
        session
            .ui
            .get_widget("choice")
            .unwrap()
            .as_tree_item()
            .unwrap()
            .embedded_widget_mut()
            .unwrap()
            .value()
            .to_i32(),
        Some(1)
    );
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires native Metal; run with --ignored"]
fn shared_ui_presenter_draws_into_a_foreign_metal_layer() {
    use theframework::platform::*;
    let layer = objc2_quartz_core::CAMetalLayer::new();
    let pointer = (&*layer as *const objc2_quartz_core::CAMetalLayer)
        .cast_mut()
        .cast();
    let surface =
        futures::executor::block_on(unsafe { SurfaceState::from_metal_layer(pointer, (320, 120)) })
            .unwrap();
    let mut presenter = GpuPresenter::new(surface).unwrap();
    let mut session = UiSession::new((320, 120), 1.);
    let mut label = TheText::new(TheId::named("apple-label"));
    label.set_text("Rust WGPU · CAMetalLayer".into());
    label.set_text_size(18.);
    session.ui.canvas.set_widget(label);
    let list = session.paint(&mut presenter.images).unwrap();
    assert_eq!(
        presenter.present(&list, 1., [0., 0., 0., 1.]).unwrap(),
        PresentResult::Presented
    );
    assert!(session.ui.canvas.buffer.pixels().is_empty());
    assert_eq!(presenter.context().transfers().readback_bytes, 0);
    presenter.surface.resize(0, 0);
    assert_eq!(
        presenter.present(&list, 1., [0.; 4]).unwrap(),
        PresentResult::Deferred
    );
    drop(presenter);
    drop(layer);
}
