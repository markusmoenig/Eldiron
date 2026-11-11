#![cfg(all(feature = "ui", feature = "render-wgpu"))]
use renderkit::*;
use theframework::{paint::PaintOp, prelude::*, render_wgpu::PaintRenderer, render_wgpu::UiImages};
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
    let first = ui.paint_commands(&mut ctx, &mut images).unwrap();
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
    let second = ui.paint_commands(&mut ctx, &mut images).unwrap();
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
    ui.paint_commands(&mut ctx, &mut images).unwrap();
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
    let mut cache = theframework::render_wgpu::UiImageCache::default();
    let list = session
        .paint(&mut cache.frame(&mut presenter.images))
        .unwrap();
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
#[test]
fn installing_layout_after_initial_resize_measures_children() {
    use theframework::platform::UiSession;
    let mut session = UiSession::new((640, 400), 1.);
    // Browser/native hosts can receive size information before application data.
    session.ui.prepare_paint_layout(&mut session.context);
    let mut layout = TheVLayout::new(TheId::named("late-layout"));
    let mut label = TheText::new(TheId::named("late-label"));
    label.set_text("Installed after resize".into());
    layout.widgets().push(Box::new(label));
    session.ui.canvas.set_layout(layout);
    session.ui.prepare_paint_layout(&mut session.context);
    let dim = *session.ui.get_widget("late-label").unwrap().dim();
    assert!(dim.width > 0 && dim.height > 0);
    assert!(session.ui.canvas.buffer.pixels().is_empty());
    let mut replacement = TheVLayout::new(TheId::named("replacement-layout"));
    replacement
        .widgets()
        .push(Box::new(TheTraybarButton::new(TheId::named("replacement"))));
    session.ui.canvas.set_layout(replacement);
    assert!(session.ui.needs_paint(&session.context));
    session.ui.prepare_paint_layout(&mut session.context);
    let dim = *session.ui.get_widget("replacement").unwrap().dim();
    assert!(dim.width > 0 && dim.height > 0);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn rgba_tile_view_blits_assets_and_keeps_display_changes_gpu_only() {
    use theframework::platform::UiSession;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((256, 128), 1.);
    let mut view = TheRGBAView::new(TheId::named("tiles"));
    let mut source = TheRGBABuffer::new(TheDim::sized(16, 8));
    source.fill([255, 0, 0, 255]);
    view.set_buffer(source);
    view.set_mode(TheRGBAViewMode::TileEditor);
    view.set_grid_dimensions(Vec2::new(8, 4));
    view.set_zoom(8.);
    view.set_grid_color([0, 255, 0, 255]);
    view.set_selection([(0, 0), (1, 0)].into_iter().collect());
    let mut preview = TheRGBABuffer::new(TheDim::sized(4, 2));
    preview.fill([0, 0, 255, 255]);
    view.set_paste_preview(Some((preview, Vec2::new(8, 4))));
    session.ui.canvas.set_widget(view);
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(256, 128)).unwrap();
    let list = session.paint(&mut images).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &target.view,
            target.descriptor.format,
            (256, 128),
            1.,
            [0.; 4],
        )
        .unwrap();
    let uploads = gpu.transfers().uploaded_bytes;
    session
        .ui
        .get_widget("tiles")
        .unwrap()
        .as_rgba_view()
        .unwrap()
        .set_selection([(0, 0), (1, 0), (0, 1)].into_iter().collect());
    let list = session.paint(&mut images).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &target.view,
            target.descriptor.format,
            (256, 128),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(
        gpu.transfers().uploaded_bytes,
        uploads,
        "selection changes must not reupload editable source pixels"
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, target).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &pixels.pixels()[(y * 256 + x) * 4..(y * 256 + x + 1) * 4];
    assert_eq!(pixel(68, 36), &[255, 0, 0, 255]);
    assert_eq!(
        pixel(128, 48),
        &[0, 255, 0, 255],
        "shared selection edges must leave the grid visible"
    );
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-tiles.png").unwrap();
    let mut encoder = png::Encoder::new(file, 256, 128);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels.pixels())
        .unwrap();
    let paste = pixel(132, 68);
    assert!(
        paste[2] > paste[0],
        "paste preview is a translucent GPU image: {paste:?}"
    );
    session
        .ui
        .get_widget("tiles")
        .unwrap()
        .as_rgba_view()
        .unwrap()
        .buffer_mut()
        .pixels_mut()[..4]
        .copy_from_slice(&[0, 255, 255, 255]);
    session.paint(&mut images).unwrap();
    assert_eq!(
        gpu.transfers().uploaded_bytes,
        uploads + 16 * 8 * 8,
        "an asset edit uploads the source image, not a window-sized framebuffer"
    );
    assert!(session.ui.canvas.buffer.pixels().is_empty());
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn palette_grid_and_popup_pick_from_their_painted_cells() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((320, 320), 1.);
    let palette = ThePalette::new(vec![
        Some(TheColor::from([255, 0, 0, 255])),
        Some(TheColor::from([0, 255, 0, 255])),
        Some(TheColor::from([0, 0, 255, 255])),
        Some(TheColor::from([255, 255, 255, 255])),
    ]);
    let mut grid = ThePalettePicker::new(TheId::named("palette"));
    grid.set_palette(palette.clone());
    grid.set_rows_columns(2, 2);
    grid.limiter_mut().set_max_height(90);
    let mut picker = ThePaletteIndexPicker::new(TheId::named("palette-popup"));
    picker.set_palette(palette.clone());
    let mut row = ThePaletteIndexRowPicker::new(TheId::named("palette-row"));
    row.set_palette(palette);
    row.set_selected_indices(vec![0, 1]);
    let mut layout = TheVLayout::new(TheId::named("palette-controls"));
    layout.widgets().push(Box::new(grid));
    layout.widgets().push(Box::new(picker));
    layout.widgets().push(Box::new(row));
    session.ui.canvas.set_layout(layout);
    session.paint(&mut images).unwrap();
    let dim = *session.ui.get_widget("palette").unwrap().dim();
    let position = [dim.x as f32 + 10. + 19. + 9., dim.y as f32 + 8. + 9.];
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
    assert_eq!(
        session
            .ui
            .get_widget("palette")
            .unwrap()
            .as_palette_picker()
            .unwrap()
            .index(),
        1
    );
    for (name, index) in [("palette-popup", 2), ("palette-row", 3)] {
        let dim = *session.ui.get_widget(name).unwrap().dim();
        let position = [dim.x as f32 + 10., dim.y as f32 + 10.];
        session.input(&InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        });
        let list = session.paint(&mut images).unwrap();
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(320, 320)).unwrap();
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (320, 320),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert!(session.context.ui.overlay.is_some());
        let position = [
            dim.x as f32 + 10. + index as f32 * 20. + 9.,
            dim.y as f32 + 20. + 10. + 9.,
        ];
        if name == "palette-popup" {
            assert_eq!(gpu.transfers().readback_bytes, 0);
            let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
            let pixels = capture.try_finish(&gpu).unwrap().unwrap();
            let offset = (position[1] as usize * 320 + position[0] as usize) * 4;
            assert_eq!(
                &pixels.pixels()[offset..offset + 4],
                &[0, 0, 255, 255],
                "the blue palette cell must be painted where it receives input"
            );
            let file = std::fs::File::create("/private/tmp/eldiron-wgpu-palette.png").unwrap();
            let mut encoder = png::Encoder::new(file, 320, 320);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(pixels.pixels())
                .unwrap();
        }
        session.input(&InputEvent::PointerMoved { position });
        session.input(&InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position,
        });
        assert!(session.context.ui.overlay.is_none());
        let value = session.ui.get_widget(name).unwrap().value();
        match value {
            TheValue::Int(v) => assert_eq!(v, index),
            TheValue::List(v) => assert_eq!(v[0].to_i32(), Some(index)),
            _ => panic!("unexpected palette value"),
        };
    }
    assert!(session.ui.canvas.buffer.pixels().is_empty());
    assert_eq!(gpu.transfers().capture_requests, 1);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn rich_text_paints_styles_and_routes_a_grapheme_link_once() {
    use theframework::{paint::Brush, platform::*, theui::thewidget::thetextedit::TheTextStyle};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((420, 140), 1.5);
    let events = session.ui.add_state_listener("rich-events".into());
    let link = TheId::named("rich-link");
    let mut view = TheTextView::new(TheId::named("rich"));
    view.set_font_size(18.);
    view.set_word_wrap(false);
    view.set_padding((8, 8, 8, 8));
    view.set_blocks(vec![TheTextViewBlock {
        spans: vec![
            TheTextViewSpan {
                text: "Read ".into(),
                ..Default::default()
            },
            TheTextViewSpan {
                text: "cafe\u{301} link".into(),
                style: TheTextStyle {
                    foreground: Some(TheColor::from([0u8, 255, 255, 255])),
                    background: Some(TheColor::from([30u8, 40, 60, 255])),
                    underline: Some(TheColor::from([0u8, 100, 255, 255])),
                },
                interaction: Some(TheTextViewInteraction {
                    id: link.clone(),
                    value: TheValue::Int(7),
                    hover_background: Some(TheColor::from([50u8, 60, 90, 255])),
                }),
            },
        ],
        ..Default::default()
    }]);
    session.ui.canvas.set_widget(view);
    session.paint(&mut images).unwrap();
    let layout = session
        .context
        .text
        .layout("Read cafe\u{301} link", "Roboto", 18., None);
    let caret = layout.caret(5);
    let position = [8. + caret.x + 3., 8. + caret.y + 10.];
    session.input(&InputEvent::PointerMoved { position });
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
    let count=events.try_iter().filter(|e|matches!(e,TheEvent::Custom(id,value) if id.uuid==link.uuid&&value.to_i32()==Some(7))).count();
    assert_eq!(count, 1, "link activation must occur exactly once");
    let list = session.paint(&mut images).unwrap();
    assert!(
        list.ops.iter().any(
            |v| matches!(v,PaintOp::GlyphRun{color,..} if color[0]==0.&&color[1]==1.&&color[2]==1.)
        ),
        "span foreground must survive hover background changes"
    );
    assert!(list.ops.iter().any(|v|matches!(v,PaintOp::Rect{rect,brush:Brush::Solid(color),..} if rect.height==1.&&color[2]==1.)),"underlines are paint geometry");
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(630, 210)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (630, 210),
            1.5,
            [0.05, 0.05, 0.05, 1.],
        )
        .unwrap();
    let uploaded = gpu.transfers().uploaded_bytes;
    let rasterized = renderer.glyphs.rasterizations;
    let repeated = session.paint(&mut images).unwrap();
    renderer
        .render(
            &repeated,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (output.descriptor.width, output.descriptor.height),
            1.5,
            [0.05, 0.05, 0.05, 1.],
        )
        .unwrap();
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    assert_eq!(renderer.glyphs.rasterizations, rasterized);
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
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-richtext.png").unwrap();
    let mut encoder = png::Encoder::new(file, 630, 210);
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
fn markdown_uses_projected_grapheme_ranges_for_gpu_links() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut session = UiSession::new((420, 220), 1.5);
    let events = session.ui.add_state_listener("markdown-events".into());
    let mut view = TheMarkdownView::new(TheId::named("markdown"));
    view.set_font_size(18.);
    view.set_padding((8, 8, 8, 8));
    view.set_text(
        "é **[cafe\u{301}](https://example.invalid)**\n\n# Heading\n\nUse `value`.".into(),
    );
    view.set_markdown_styles(TheMarkdownStyles {
        heading: Some(TheColor::from([255u8, 200, 60, 255])),
        link_hovered: Some(TheColor::from([0u8, 255, 255, 255])),
        ..Default::default()
    });
    let projected = view.text();
    assert_eq!(projected, "é cafe\u{301}\nHeading\nUse value.");
    session.ui.canvas.set_widget(view);
    session.paint(&mut images).unwrap();
    let layout = session
        .context
        .text
        .layout(&projected, "Roboto", 18., Some(391.));
    let caret = layout.caret("é ".len());
    let position = [8. + caret.x + 3., 8. + caret.y + 10.];
    session.input(&InputEvent::PointerMoved { position });
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
    assert_eq!(events.try_iter().filter(|e|matches!(e,TheEvent::ExternalUrlRequested(url) if url=="https://example.invalid")).count(),1);
    let list = session.paint(&mut images).unwrap();
    assert!(list.ops.iter().any(
        |v| matches!(v,PaintOp::GlyphRun{color,..} if color[0]==0.&&color[1]==1.&&color[2]==1.)
    ));
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(630, 330)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (630, 330),
            1.5,
            [0.05, 0.05, 0.05, 1.],
        )
        .unwrap();
    let uploaded = gpu.transfers().uploaded_bytes;
    let rasterized = renderer.glyphs.rasterizations;
    let repeated = session.paint(&mut images).unwrap();
    renderer
        .render(
            &repeated,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (output.descriptor.width, output.descriptor.height),
            1.5,
            [0.05, 0.05, 0.05, 1.],
        )
        .unwrap();
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    assert_eq!(renderer.glyphs.rasterizations, rasterized);
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
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-markdown.png").unwrap();
    let mut encoder = png::Encoder::new(file, 630, 330);
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
fn procedural_controls_use_mesh_and_analytic_distance_paint() {
    use theframework::{
        paint::{PaintList, Rect},
        theui::paint::WidgetPainter,
    };
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut ctx = TheContext::new(640, 230, 1.5);
    let mut style = TheClassicStyle::new();
    let mut color = TheColorPicker::new(TheId::named("color"));
    color.set_dim(TheDim::new(10, 10, 200, 200), &mut ctx);
    color.set_color(Vec3::new(0., 1., 0.));
    let mut direction = TheDirectionPicker::new(TheId::named("direction"));
    direction.set_dim(TheDim::new(220, 10, 200, 200), &mut ctx);
    assert!(!direction.on_event(&TheEvent::MouseDown(Vec2::new(100, 100)), &mut ctx));
    direction.on_event(&TheEvent::MouseDown(Vec2::new(190, 100)), &mut ctx);
    assert_eq!(
        direction.value(),
        TheValue::Direction(Vec3::new(1., 0., 0.))
    );
    let mut sdf = TheSDFView::new(TheId::named("sdf"));
    sdf.set_dim(TheDim::new(430, 10, 200, 200), &mut ctx);
    let mut field = TheSDFCanvas::new();
    field.add(
        TheSDF::Circle(TheDim::new(20, 20, 80, 80)),
        ThePattern::Solid(TheColor::from_u8_array([255, 0, 0, 255])),
    );
    field.add(
        TheSDF::Circle(TheDim::new(55, 25, 70, 70)),
        ThePattern::Solid(TheColor::from_u8_array([0, 255, 0, 255])),
    );
    field.add(
        TheSDF::RoundedRect(TheDim::new(20, 130, 55, 45), (0., 0., 0., 0.)),
        ThePattern::SolidWithBorder(
            TheColor::from_u8_array([0, 0, 255, 255]),
            TheColor::white(),
            2.,
        ),
    );
    field.add(
        TheSDF::Rhombus(TheDim::new(100, 130, 50, 50)),
        ThePattern::Solid(TheColor::white()),
    );
    sdf.set_canvas(field);
    let mut list = PaintList::default();
    list.rect(Rect::new(0., 0., 640., 230.), 0., [0.06, 0.06, 0.07, 1.]);
    for widget in [&mut color as &mut dyn TheWidget, &mut direction, &mut sdf] {
        widget.prepare_paint(&mut ctx, style.theme().as_ref());
        let mut p = WidgetPainter {
            list: &mut list,
            text: &mut ctx.text,
            images: &mut images,
            ui: &ctx.ui,
            theme: style.theme().as_mut(),
        };
        p.widget(widget).unwrap();
    }
    assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Mesh { .. })));
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::DistanceField { .. }))
    );
    assert_eq!(
        gpu.transfers().uploaded_bytes,
        0,
        "procedural controls must not upload raster assets"
    );
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 345)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (960, 345),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let result = capture.try_finish(&gpu).unwrap().unwrap();
    let pixel = |x: usize, y: usize| {
        let i = ((y as f32 * 1.5) as usize * 960 + (x as f32 * 1.5) as usize) * 4;
        &result.pixels()[i..i + 4]
    };
    let red = pixel(110, 24);
    assert!(
        red[0] > 245 && red[1] < 15 && red[2] < 15,
        "hue ring: {red:?}"
    );
    assert_eq!(
        pixel(505, 70),
        [255, 0, 0, 255],
        "nearest field wins in overlap, independent of painter order"
    );
    assert_eq!(pixel(530, 70), [0, 255, 0, 255]);
    assert_eq!(pixel(470, 160), [0, 0, 255, 255]);
    assert_eq!(pixel(555, 165), [255, 255, 255, 255]);
    assert!(
        pixel(400, 110)[0] < 110,
        "direction line follows its own widget center: {:?}",
        pixel(400, 110)
    );
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-procedural.png").unwrap();
    let mut encoder = png::Encoder::new(file, 960, 345);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(result.pixels())
        .unwrap();
    color.on_event(&TheEvent::MouseDown(Vec2::new(100, 5)), &mut ctx);
    color.on_event(&TheEvent::MouseUp(Vec2::new(100, 5)), &mut ctx);
    if let TheValue::Float3(v) = color.value() {
        assert!(v.x > 0.99 && v.y < 0.01 && v.z < 0.01);
    } else {
        panic!("color value lost");
    }
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn node_canvas_paints_live_gpu_previews_and_routes_terminal_dragging() {
    use theframework::{paint::PaintList, platform::*};
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let preview = GpuImage::new(&gpu, ImageDescriptor::rgba(64, 48)).unwrap();
    renderer
        .render(
            &PaintList::default(),
            &images.registry,
            &preview.view,
            preview.descriptor.format,
            (64, 48),
            1.,
            [0., 0., 1., 1.],
        )
        .unwrap();
    let handle = images.registry.insert(preview).unwrap();
    let mut session = UiSession::new((600, 300), 1.5);
    let events = session.ui.add_state_listener("nodes".into());
    let mut canvas = TheNodeCanvas::new();
    canvas.node_width = 160;
    let terminal = || TheNodeTerminal {
        name: "Color".into(),
        category_name: "ColorSocket".into(),
    };
    for (name, position, input) in [
        ("Source", Vec2::new(20, 30), false),
        ("Output", Vec2::new(330, 70), true),
    ] {
        canvas.nodes.push(TheNode {
            name: name.into(),
            status_text: None,
            position,
            inputs: if input { vec![terminal()] } else { vec![] },
            outputs: if input { vec![] } else { vec![terminal()] },
            preview: TheRGBABuffer::empty(),
            supports_preview: true,
            preview_is_open: true,
            can_be_deleted: true,
        });
    }
    canvas
        .categories
        .insert("ColorSocket".into(), TheColor::white());
    let mut view = TheNodeCanvasView::new(TheId::named("nodes"));
    view.set_canvas(canvas);
    view.set_node_preview_image(0, Some((handle, Vec2::new(64, 48))));
    let mut overlay = TheRGBABuffer::new(TheDim::sized(1, 1));
    overlay.fill([0, 0, 0, 0]);
    view.set_overlay(Some(overlay));
    session.ui.canvas.set_widget(view);
    let first = session.paint(&mut images).unwrap();
    assert_eq!(
        first
            .ops
            .iter()
            .filter(|op| matches!(op, PaintOp::Image { .. }))
            .count(),
        2,
        "a tiled 1px overlay is one quad; the preview is another"
    );
    assert!(session.ui.canvas.buffer.pixels().is_empty());
    let uploads = gpu.transfers().uploaded_bytes;
    let down = [168., 61.];
    let up = [342., 101.];
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: down,
    });
    session.input(&InputEvent::PointerMoved { position: up });
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: up,
    });
    assert_eq!(
        events
            .try_iter()
            .filter(|e| matches!(e,TheEvent::NodeConnectionAdded(_,v) if v==&vec![(0,0,1,0)]))
            .count(),
        1
    );
    let list = session.paint(&mut images).unwrap();
    assert_eq!(
        gpu.transfers().uploaded_bytes,
        uploads,
        "selection and wires cannot upload previews or overlays again"
    );
    assert!(list.ops.iter().any(|op|matches!(op,PaintOp::Stroke{path,..} if path.elements.iter().any(|e|matches!(e,theframework::paint::PathElement::Cubic(..))))));
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(900, 450)).unwrap();
    renderer
        .render(
            &list,
            &images.registry,
            &output.view,
            output.descriptor.format,
            (900, 450),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let result = capture.try_finish(&gpu).unwrap().unwrap();
    let i = (198 * 900 + 151) * 4;
    assert_eq!(
        &result.pixels()[i..i + 4],
        [0, 0, 255, 255],
        "GPU-only preview appears inside the node"
    );
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-nodes.png").unwrap();
    let mut encoder = png::Encoder::new(file, 900, 450);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(result.pixels())
        .unwrap();
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: [80., 39.],
    });
    session.input(&InputEvent::PointerMoved {
        position: [100., 39.],
    });
    session.input(&InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: [100., 39.],
    });
    assert_eq!(
        events
            .try_iter()
            .filter(|e| matches!(e,TheEvent::NodeDragged(_,0,p) if *p==Vec2::new(40,30)))
            .count(),
        1
    );
    session.paint(&mut images).unwrap();
    assert_eq!(
        session
            .ui
            .get_widget("nodes")
            .unwrap()
            .as_node_canvas_view()
            .unwrap()
            .node_index_at(&Vec2::new(41, 31)),
        Some(0)
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn optional_ui_adapter_uses_host_registry_and_rebuilds_only_device_resources() {
    use theframework::{
        paint::{PaintList, Rect, Sampling},
        platform::*,
        render_wgpu::UiFrameImages,
        theui::paint::UiImageResources,
    };
    struct App {
        initializations: usize,
        source: TheRGBABuffer,
    }
    impl UiApplication for App {
        fn initialize(&mut self, ui: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
            self.initializations += 1;
            let mut label = TheText::new(TheId::named("retained-document"));
            label.set_text("Persistent UI".into());
            ui.ui.canvas.set_widget(label);
            Ok(())
        }
        fn frame(
            &mut self,
            ui: &mut UiSession,
            images: &mut UiFrameImages<'_>,
        ) -> Result<PaintList> {
            let mut list = ui.paint(images).map_err(|e| RenderError(e.to_string()))?;
            let handle = images
                .image("test-source", &self.source, false)
                .map_err(|e| RenderError(e.to_string()))?;
            assert_eq!(handle.owner, images.registry.owner_id());
            list.image(handle, Rect::new(8., 60., 20., 20.), Sampling::Nearest);
            Ok(list)
        }
    }
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let source = TheRGBABuffer::from(vec![255, 0, 0, 255], 1, 1);
    let mut app = UiApplicationAdapter::new(App {
        initializations: 0,
        source,
    });
    let mut host = HostContext::new((240, 100));
    app.initialize(&mut host, &mut images).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(240, 100)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let first = app.frame(&mut host, &mut images).unwrap();
    renderer
        .render(
            &first,
            &images,
            &output.view,
            output.descriptor.format,
            (240, 100),
            1.,
            [0.; 4],
        )
        .unwrap();
    let uploads = gpu.transfers().uploaded_bytes;
    let second = app.frame(&mut host, &mut images).unwrap();
    renderer
        .render(
            &second,
            &images,
            &output.view,
            output.descriptor.format,
            (240, 100),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    let old_owner = images.owner_id();
    let mut replacement = ImageRegistry::new(gpu.clone());
    app.gpu_recreated(&mut host, &mut replacement).unwrap();
    let third = app.frame(&mut host, &mut replacement).unwrap();
    assert!(third.ops.iter().any(|op| matches!(op,PaintOp::Image { image,.. } if image.owner != old_owner && image.owner == replacement.owner_id())));
    renderer
        .render(
            &third,
            &replacement,
            &output.view,
            output.descriptor.format,
            (240, 100),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(app.application.initializations, 1);
    assert!(app.session.ui.get_widget("retained-document").is_some());
    assert!(app.session.ui.canvas.buffer.pixels().is_empty());
    assert_eq!(gpu.transfers().readback_bytes, 0);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_multiline_editor_keeps_annotations_preedit_and_undo_across_gpu_replacement() {
    use theframework::platform::*;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = UiImages::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(1000, 640)).unwrap();
    let mut session = UiSession::new((500, 320), 2.);
    let id = TheId::named("Native Code");
    let mut area = TheTextAreaEdit::new(id.clone());
    area.set_text("name = \"東京\"\nprint(name)\n# العربية e\u{301}\n".into());
    area.as_code_editor("Python", TheCodeEditorSettings::default());
    area.set_font_size(18.);
    area.display_line_number(true);
    area.use_statusbar(true);
    area.set_matches(&[(12, 17)]);
    area.set_errors(&[(0, 4)]);
    area.set_debug_line(Some(2));
    session.ui.canvas.set_widget(area);
    session.ui.prepare_paint_layout(&mut session.context);
    session.context.ui.set_focus(&id);
    session.ui.process_events(&mut session.context);
    session
        .ui
        .get_text_area_edit("Native Code")
        .unwrap()
        .goto_line(1);
    session.input(&InputEvent::Text("e\u{301} ".into()));
    let committed = session.ui.get_text_area_edit("Native Code").unwrap().text();
    let input = session
        .ui
        .get_widget_id(id.uuid)
        .unwrap()
        .editor_input_state(&mut session.context)
        .unwrap();
    session.input(&InputEvent::CompositionFor {
        target: input.target,
        text: "日本".into(),
        cursor: Some((3, 6)),
    });
    assert_eq!(
        session.ui.get_text_area_edit("Native Code").unwrap().text(),
        committed
    );
    let mut steady = None;
    for _ in 0..2 {
        let list = session.paint(&mut images).unwrap();
        list.validate().unwrap();
        assert!(
            !list
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::Image { .. } | PaintOp::FilteredImage { .. })),
            "code text/chrome cannot become a CPU source image"
        );
        renderer
            .render(
                &list,
                &images.registry,
                &target.view,
                target.descriptor.format,
                (1000, 640),
                2.,
                [0.; 4],
            )
            .unwrap();
        let now = (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
        );
        if let Some(previous) = steady {
            assert_eq!(now, previous);
        }
        steady = Some(now);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let state = session
            .ui
            .get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(&mut session.context)
            .unwrap();
        assert_eq!(state.target, input.target);
        assert!(
            state.area.x >= 0.
                && state.area.y >= 0.
                && state.area.x + state.area.width <= 500.01
                && state.area.y + state.area.height <= 320.01
        );
    }
    let capture = |gpu: &GpuContext, target: std::sync::Arc<GpuImage>| {
        let mut ticket = ReadbackTicket::capture(gpu, target).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        ticket.try_finish(gpu).unwrap().unwrap().into_pixels()
    };
    let pixels = capture(&gpu, target);
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-code-editor.png").unwrap(),
        1000,
        640,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    // The retained CPU editor/IME/history and annotations do not belong to the device.
    let gpu2 = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images2 = UiImages::new(gpu2.clone());
    let mut renderer2 = PaintRenderer::new(gpu2.clone()).unwrap();
    let target2 = GpuImage::new(&gpu2, ImageDescriptor::rgba(1000, 640)).unwrap();
    let list = session.paint(&mut images2).unwrap();
    renderer2
        .render(
            &list,
            &images2.registry,
            &target2.view,
            target2.descriptor.format,
            (1000, 640),
            2.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu2.transfers().readback_bytes, 0);
    assert_eq!(
        session
            .ui
            .get_widget_id(id.uuid)
            .unwrap()
            .editor_input_state(&mut session.context)
            .unwrap()
            .target,
        input.target
    );
    assert_eq!(
        capture(&gpu2, target2),
        pixels,
        "recreated GPU resources preserve actual native editor pixels"
    );
    session.input(&InputEvent::TextFor {
        target: input.target,
        text: "日本".into(),
    });
    assert_ne!(
        session.ui.get_text_area_edit("Native Code").unwrap().text(),
        committed
    );
    session.ui.undo(&mut session.context);
    assert_eq!(
        session.ui.get_text_area_edit("Native Code").unwrap().text(),
        committed,
        "IME commits form one undo transaction across GPU replacement"
    );
    assert!(session.ui.canvas.buffer.pixels().is_empty());
}
