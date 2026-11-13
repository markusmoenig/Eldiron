#![cfg(feature = "render-wgpu")]
use renderkit::*;
use theframework::{paint::*, render_wgpu::PaintRenderer, text::TextSystem};
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn ordered_paint_clips_groups_and_swash_blits() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(ctx.clone());
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(480, 300)).unwrap();
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut list = PaintList::default();
    list.rect(Rect::new(0., 0., 320., 200.), 0., [0.05, 0.05, 0.05, 1.]);
    list.ops
        .push(PaintOp::PushClip(Rect::new(8., 8., 40., 40.)));
    list.rect(Rect::new(0., 0., 64., 64.), 0., [1., 0., 0., 1.]);
    list.ops.push(PaintOp::PopClip);
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.rect(Rect::new(80., 8., 40., 40.), 0., [0., 0., 1., 1.]);
    list.rect(Rect::new(88., 8., 40., 40.), 0., [0., 1., 0., 1.]);
    list.ops.push(PaintOp::PopOpacity);
    let mut text = TextSystem::default();
    text.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let layout = text.layout("Eldiron WGPU – Swash", "Roboto", 18., None);
    for run in &layout.runs {
        list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin: [12., 68.],
            color: [1.; 4],
        });
    }
    list.ops.push(PaintOp::Stroke {
        path: std::sync::Arc::new(Path {
            elements: vec![
                PathElement::Move([12., 130.]),
                PathElement::Cubic([80., 100.], [130., 170.], [240., 125.]),
            ],
        }),
        stroke: Stroke {
            width: 2.,
            cap: LineCap::Round,
            join: LineJoin::Round,
            brush: Brush::Solid([0.4, 0.7, 1., 1.]),
        },
    });
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (480, 300),
            1.5,
            [0.; 4],
        )
        .unwrap();
    let uploads = ctx.transfers().uploaded_bytes;
    let count = renderer.glyphs.rasterizations;
    assert!(count > 0);
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (480, 300),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(renderer.glyphs.rasterizations, count);
    assert_eq!(ctx.transfers().uploaded_bytes, uploads);
    assert_eq!(ctx.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let result = capture.try_finish(&ctx).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &result.pixels()[(y * 480 + x) * 4..(y * 480 + x + 1) * 4];
    assert_eq!(pixel(30, 30), &[255, 0, 0, 255]);
    assert!(pixel(1, 1)[0] < 20);
    let overlap = pixel(150, 30);
    assert!(
        overlap[1] > 170 && overlap[2] < 20,
        "isolated group must not leak blue through overlapping opaque green: {overlap:?}"
    );
    let file = std::fs::File::create("/private/tmp/eldiron-wgpu-paint.png").unwrap();
    let mut encoder = png::Encoder::new(file, 480, 300);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(result.pixels())
        .unwrap();
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn dense_text_batches_atlas_pages_without_reuploading() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(ctx.clone());
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(1024, 256)).unwrap();
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut text = TextSystem::default();
    text.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let layout = text.layout(
        &"Eldiron scene text 0123456789 ".repeat(20),
        "Roboto",
        18.,
        Some(650.),
    );
    let mut list = PaintList::default();
    for run in &layout.runs {
        list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin: [12.25, 8.5],
            color: [0.8, 0.9, 1., 0.75],
        });
    }
    for frame in 0..2 {
        let before = ctx.transfers().uploaded_bytes;
        let rasterizations = renderer.glyphs.rasterizations;
        renderer
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (1024, 256),
                1.,
                [0.; 4],
            )
            .unwrap();
        let stats = renderer.frame_stats();
        assert!(stats.glyph_quads > 400, "{stats:?}");
        assert!(stats.draw_calls <= layout.runs.len() + 1, "{stats:?}");
        assert!(stats.draw_calls * 10 < stats.glyph_quads, "{stats:?}");
        if frame == 1 {
            assert_eq!(ctx.transfers().uploaded_bytes, before);
            assert_eq!(renderer.glyphs.rasterizations, rasterizations);
        }
        assert_eq!(ctx.transfers().readback_bytes, 0);
    }
    let mut ticket = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = ticket.try_finish(&ctx).unwrap().unwrap();
    assert!(
        capture
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[3] > 100)
            .count()
            > 1000
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn source_normalization_filters_transparent_edges_in_linear_premultiplied_color() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(ctx.clone());
    let edge = ImageAsset::rgba(2, 1, vec![255, 0, 0, 255, 0, 0, 0, 0]).unwrap();
    let edge = images.upload_linear(&edge).unwrap();
    let descriptor = images.get(edge).unwrap().descriptor;
    assert_eq!(descriptor.format, wgpu::TextureFormat::Rgba16Float);
    assert_eq!(
        (descriptor.color_space, descriptor.alpha_mode),
        (ColorSpace::Linear, AlphaMode::Premultiplied)
    );
    let mut list = PaintList::default();
    list.image(edge, Rect::new(0., 0., 8., 8.), Sampling::Linear);
    list.rect(Rect::new(0., 8., 64., 8.), 0., [0., 0., 0., 1.]);
    for (i, (space, alpha, pixel)) in [
        (ColorSpace::Srgb, AlphaMode::Straight, [255, 0, 0, 128]),
        (ColorSpace::Srgb, AlphaMode::Premultiplied, [128, 0, 0, 128]),
        (ColorSpace::Linear, AlphaMode::Straight, [255, 0, 0, 128]),
        (
            ColorSpace::Linear,
            AlphaMode::Premultiplied,
            [128, 0, 0, 128],
        ),
        (ColorSpace::Srgb, AlphaMode::Opaque, [255, 0, 0, 0]),
        (
            ColorSpace::Srgb,
            AlphaMode::Premultiplied,
            [200, 128, 64, 0],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut asset = ImageAsset::rgba(1, 1, pixel.to_vec()).unwrap();
        asset.color_space = space;
        asset.alpha_mode = alpha;
        let image = images.upload_linear(&asset).unwrap();
        list.image(
            image,
            Rect::new(i as f32 * 8., 8., 8., 8.),
            Sampling::Nearest,
        );
    }
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(64, 16)).unwrap();
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (64, 16),
            1.,
            [1.; 4],
        )
        .unwrap();
    assert_eq!(ctx.transfers().readback_bytes, 0);
    let mut ticket = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = ticket.try_finish(&ctx).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
    let edge = pixel(3, 3);
    assert_eq!(
        edge[0], 255,
        "transparent texels must not darken the red edge: {edge:?}"
    );
    assert!(
        (160..=170).contains(&edge[1]) && edge[1] == edge[2],
        "linear source-over on white: {edge:?}"
    );
    for x in [3, 11, 19, 27] {
        let p = pixel(x, 11);
        assert!(
            (187..=189).contains(&p[0]) && p[1] == 0 && p[2] == 0 && p[3] == 255,
            "equivalent source representations at {x}: {p:?}"
        );
    }
    assert_eq!(
        pixel(35, 11),
        [255, 0, 0, 255],
        "opaque metadata overrides source alpha"
    );
    assert_eq!(
        pixel(43, 11),
        [0, 0, 0, 255],
        "zero-alpha premultiplied color must normalize to zero"
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires native GPU and system color emoji font; run with --ignored"]
fn swash_color_glyphs_use_normalized_atlas_images() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(ctx.clone());
    let mut text = TextSystem::default();
    let layout = text.layout("😀", "Apple Color Emoji", 48., None);
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut found_color = false;
    let mut list = PaintList::default();
    for run in &layout.runs {
        for glyph in run.glyphs.iter() {
            if let Some((_, bitmap)) = renderer.glyphs.glyph(run, glyph.id, 1., glyph.position) {
                found_color |= bitmap.color;
            }
        }
        list.ops.push(PaintOp::GlyphRun {
            run: run.clone(),
            origin: [8., 4.],
            color: [1.; 4],
        });
    }
    assert!(
        found_color,
        "native fixture requires a Swash-supported system color emoji font"
    );
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(80, 80)).unwrap();
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (80, 80),
            1.,
            [0.; 4],
        )
        .unwrap();
    let uploads = ctx.transfers().uploaded_bytes;
    let rasterizations = renderer.glyphs.rasterizations;
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (80, 80),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(ctx.transfers().uploaded_bytes, uploads);
    assert_eq!(renderer.glyphs.rasterizations, rasterizations);
    let mut ticket = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = ticket.try_finish(&ctx).unwrap().unwrap();
    assert!(
        capture
            .pixels()
            .chunks_exact(4)
            .any(|p| p[3] > 100 && p[0] > p[2].saturating_add(60)),
        "emoji must retain its authored color"
    );
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-swash-color-glyph.png").unwrap(),
        80,
        80,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(capture.pixels())
        .unwrap();
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_editor_viewports_blit_selection_preedit_and_retained_glyphs() {
    use theframework::{
        platform::InputEvent,
        text::TextOptions,
        text_edit::{EditorColors, EditorView},
    };
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(ctx.clone());
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(720, 360)).unwrap();
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut fonts = TextSystem::default();
    fonts.register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
    let mut field = EditorView::new(false);
    field
        .editor
        .set_text(&mut fonts, "Eldiron — العربية 東京 e\u{301} 👩‍💻");
    field.prepare(
        &mut fonts,
        TextOptions {
            family: "Roboto",
            size: 20.,
            width: None,
            line_height: None,
        },
        Rect::new(20., 16., 440., 40.),
        true,
    );
    field.set_focused(&mut fonts, true);
    field.editor.select_bytes(&mut fonts, 0, "Eldiron".len());
    field.reveal_caret(&mut fonts);
    let mut ime = EditorView::new(false);
    ime.editor.set_text(&mut fonts, "Player 東京");
    ime.prepare(
        &mut fonts,
        TextOptions {
            family: "Roboto",
            size: 20.,
            width: None,
            line_height: None,
        },
        Rect::new(20., 72., 440., 40.),
        true,
    );
    ime.set_focused(&mut fonts, true);
    ime.editor.select_bytes(&mut fonts, 7, "Player 東京".len());
    ime.input(
        &mut fonts,
        &InputEvent::Composition {
            text: "にほん".into(),
            cursor: Some((3, 6)),
        },
    );
    assert_eq!(ime.editor.text(), "Player 東京");
    let mut code = EditorView::new(true);
    code.editor.set_text(
        &mut fonts,
        "First line\nSecond line\nThird line\nFourth line\nLast 東京 e\u{301}",
    );
    code.prepare(
        &mut fonts,
        TextOptions {
            family: "Roboto",
            size: 18.,
            width: Some(440.),
            line_height: Some(24.),
        },
        Rect::new(20., 128., 440., 88.),
        false,
    );
    code.set_focused(&mut fonts, true);
    assert!(code.scroll()[1] > 0.);
    let mut list = PaintList::default();
    list.rect(Rect::new(0., 0., 480., 240.), 0., [0.04, 0.06, 0.09, 1.]);
    for view in [&mut field, &mut ime, &mut code] {
        list.rect(view.bounds(), 2., [0.10, 0.14, 0.20, 1.]);
        view.paint(&mut fonts, &mut list, EditorColors::default(), true);
    }
    list.validate().unwrap();
    for frame in 0..2 {
        let before = ctx.transfers().uploaded_bytes;
        let count = renderer.glyphs.rasterizations;
        renderer
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (720, 360),
                1.5,
                [0.; 4],
            )
            .unwrap();
        assert!(renderer.frame_stats().glyph_quads > 20);
        if frame > 0 {
            assert_eq!(ctx.transfers().uploaded_bytes, before);
            assert_eq!(renderer.glyphs.rasterizations, count);
        }
        assert_eq!(ctx.transfers().readback_bytes, 0);
    }
    let mut capture = ReadbackTicket::capture(&ctx, output.clone()).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let result = capture.try_finish(&ctx).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &result.pixels()[(y * 720 + x) * 4..(y * 720 + x + 1) * 4];
    assert_eq!(
        pixel(10, 30),
        pixel(700, 30),
        "text must not escape viewport"
    );
    assert!(
        pixel(32, 36)[2] > pixel(700, 36)[2],
        "selection/chrome must be visible"
    );
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-editors.png").unwrap(),
        720,
        360,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(result.pixels())
        .unwrap();
    // Native input may temporarily delete all preedit while retaining its
    // original undo owner. Painting this state must use the same Swash atlas,
    // allocate no empty underline span and never trigger an implicit capture.
    let source = ime.editor.native_text_snapshot(&mut fonts).unwrap();
    ime.input(
        &mut fonts,
        &InputEvent::NativeText {
            target: source.target,
            revision: source.revision,
            action: theframework::platform::NativeTextAction::Compose {
                range: None,
                text: String::new(),
                cursor: Some((0, 0)),
            },
        },
    );
    let source = ime.editor.native_text_snapshot(&mut fonts).unwrap();
    assert_eq!(source.composition, Some(7..7));
    assert_eq!(ime.editor.text(), "Player 東京");
    let mut empty = PaintList::default();
    ime.paint(&mut fonts, &mut empty, EditorColors::default(), true);
    empty.validate().unwrap();
    let before = ctx.transfers();
    let glyphs = renderer.glyphs.rasterizations;
    renderer
        .render(
            &empty,
            &images,
            &output.view,
            output.descriptor.format,
            (720, 360),
            1.5,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(ctx.transfers().readback_bytes, before.readback_bytes);
    assert_eq!(renderer.glyphs.rasterizations, glyphs);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn decorated_editor_reuses_swash_glyphs_and_paints_native_syntax_and_diagnostics() {
    use std::sync::Arc;
    use theframework::{
        text::{TextOptions, TextStyleSpan},
        text_edit::{EditorColors, EditorDecorations, EditorView},
    };
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(800, 280)).unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let mut fonts = TextSystem::default();
    fonts.register_font(include_bytes!("../embedded/fonts/SourceCodePro-Bold.ttf").to_vec());
    let mut editor = EditorView::new(true);
    let source = "let title = \"Eldiron\";\n// العربية 東京 e\u{301}\n";
    editor.editor.set_text(&mut fonts, source);
    editor.prepare(
        &mut fonts,
        TextOptions {
            family: "Source Code Pro",
            size: 18.,
            width: None,
            line_height: Some(28.),
        },
        Rect::new(16., 16., 368., 108.),
        false,
    );
    let decorations = Arc::new(EditorDecorations {
        foreground: vec![
            TextStyleSpan {
                range: 0..3,
                color: [0.2, 0.6, 1., 1.],
            },
            TextStyleSpan {
                range: 12..21,
                color: [0.3, 0.9, 0.5, 1.],
            },
        ],
        backgrounds: vec![TextStyleSpan {
            range: 4..9,
            color: [0.3, 0.5, 0.7, 0.4],
        }],
        underlines: vec![TextStyleSpan {
            range: 4..9,
            color: [1., 0.2, 0.2, 1.],
        }],
        line_backgrounds: vec![TextStyleSpan {
            range: 22..source.len(),
            color: [0.15, 0.1, 0.1, 1.],
        }],
        whitespace: Some([0.4, 0.4, 0.4, 1.]),
    });
    let mut steady = None;
    let builds = editor.editor.snapshots_built();
    for decorated in [false, true, true] {
        let mut list = PaintList::default();
        list.rect(Rect::new(0., 0., 400., 140.), 0., [0.05, 0.05, 0.05, 1.]);
        if decorated {
            editor.paint_decorated(
                &mut fonts,
                &mut list,
                EditorColors::default(),
                false,
                &decorations,
            );
        } else {
            editor.paint(&mut fonts, &mut list, EditorColors::default(), false);
        }
        list.validate().unwrap();
        renderer
            .render(
                &list,
                &images,
                &target.view,
                target.descriptor.format,
                (800, 280),
                2.,
                [0.; 4],
            )
            .unwrap();
        let now = (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
        );
        if let Some(before) = steady {
            assert_eq!(
                now, before,
                "syntax and diagnostic colors reuse the original glyph atlas"
            );
        }
        steady = Some(now);
        assert_eq!(gpu.transfers().readback_bytes, 0);
    }
    assert_eq!(editor.editor.snapshots_built(), builds);
    let mut capture = ReadbackTicket::capture(&gpu, target).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let pixels = capture.try_finish(&gpu).unwrap().unwrap();
    assert!(
        pixels
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[2] > 180 && p[0] < 100)
            .count()
            > 10,
        "syntax blue must reach the GPU target"
    );
    assert!(
        pixels
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[0] > 200 && p[1] < 100 && p[2] < 100)
            .count()
            > 10,
        "diagnostic underline must reach the GPU target"
    );
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-decorated-editor.png").unwrap(),
        800,
        280,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(pixels.pixels())
        .unwrap();
}

#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn threshold_images_reuse_sources_preserve_coverage_and_match_encoded_luminance_boundaries() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(ctx.clone());
    let pixels = [
        [99, 99, 99, 255],
        [100, 100, 100, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [100, 100, 100, 128],
        [255, 255, 255, 0],
    ];
    let asset = ImageAsset::rgba(6, 1, pixels.into_iter().flatten().collect()).unwrap();
    let image = images.upload_linear(&asset).unwrap();
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(120, 40)).unwrap();
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut list = PaintList::default();
    list.ops
        .push(PaintOp::PushClip(Rect::new(0., 0., 120., 32.)));
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.ops.push(PaintOp::ThresholdImage {
        image,
        rect: Rect::new(0., 0., 120., 40.),
        uv: Rect::new(0., 0., 1., 1.),
        sampling: Sampling::Nearest,
        threshold: 100,
        dark: [35. / 255., 35. / 255., 35. / 255., 1.],
        light: [230. / 255., 230. / 255., 230. / 255., 1.],
    });
    list.ops.extend([PaintOp::PopOpacity, PaintOp::PopClip]);
    list.validate().unwrap();
    assert_eq!(list.image_handles().collect::<Vec<_>>(), vec![image]);
    let uploaded = ctx.transfers().uploaded_bytes;
    for _ in 0..2 {
        renderer
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (120, 40),
                1.,
                [0., 0., 0., 1.],
            )
            .unwrap();
    }
    assert_eq!(
        ctx.transfers().uploaded_bytes,
        uploaded,
        "recoloring cannot upload altered source pixels"
    );
    assert_eq!(ctx.transfers().readback_bytes, 0);
    let mut capture = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let capture = capture.try_finish(&ctx).unwrap().unwrap();
    let pixel = |x: usize, y: usize| &capture.pixels()[(y * 120 + x) * 4..(y * 120 + x + 1) * 4];
    let linear = |value: f32| {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let encoded = |value: f32| {
        if value <= 0.0031308 {
            value * 12.92
        } else {
            1.055 * value.powf(1. / 2.4) - 0.055
        }
    };
    for (i, source) in pixels.iter().enumerate() {
        let luma = (source[0] as u32 * 54 + source[1] as u32 * 183 + source[2] as u32 * 19) / 256;
        let tone = if luma >= 100 { 230. } else { 35. };
        let expected =
            (encoded(linear(tone / 255.) * source[3] as f32 / 255. * 0.5) * 255.).round() as u8;
        let actual = pixel(i * 20 + 10, 10);
        assert!(
            actual[0].abs_diff(expected) <= 2,
            "source {source:?}: {actual:?}, expected {expected}"
        );
        assert_eq!(actual[0], actual[1]);
        assert_eq!(actual[1], actual[2]);
        assert_eq!(actual[3], 255);
    }
    assert_eq!(
        pixel(10, 36),
        &[0, 0, 0, 255],
        "threshold image obeys its outer clip"
    );
}

#[test]
#[ignore = "requires native GPU"]
fn paint_receipts_survive_scopes_without_changing_pixels_or_gpu_work() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(32, 32)).unwrap();
    let mut painter = PaintRenderer::new(gpu.clone()).unwrap();
    let a = FrameReceipt {
        owner: new_receipt_owner(),
        serial: 1,
    };
    let b = FrameReceipt {
        owner: new_receipt_owner(),
        serial: 1,
    };
    assert_ne!(a.owner, b.owner);
    let mut list = PaintList::default();
    list.ops
        .push(PaintOp::PushClip(Rect::new(4., 4., 24., 24.)));
    list.ops.push(PaintOp::Receipt(a));
    list.rect(Rect::new(0., 0., 32., 32.), 0., [0.7, 0.2, 0.1, 1.]);
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.ops.push(PaintOp::Receipt(b));
    list.rect(Rect::new(8., 8., 20., 20.), 0., [0.1, 0.7, 0.2, 1.]);
    list.ops.push(PaintOp::PopOpacity);
    list.ops.push(PaintOp::PopClip);
    list.validate().unwrap();
    assert_eq!(list.receipts().collect::<Vec<_>>(), [a, b]);
    let mut captures = Vec::new();
    let mut draws = Vec::new();
    for with_receipts in [true, false] {
        if !with_receipts {
            list.ops.retain(|op| !matches!(op, PaintOp::Receipt(_)));
        }
        painter
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (32, 32),
                1.,
                [0.; 4],
            )
            .unwrap();
        draws.push(painter.frame_stats());
        let mut capture = ReadbackTicket::capture(&gpu, output.clone()).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        captures.push(capture.try_finish(&gpu).unwrap().unwrap());
    }
    assert_eq!(captures[0].pixels(), captures[1].pixels());
    assert_eq!(draws[0], draws[1]);
}

#[test]
#[ignore = "requires native GPU"]
fn raw_gpu_images_filter_in_linear_premultiplied_color_before_composition() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(ctx.clone());
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let pixels = vec![
        250, 30, 50, 255, 0, 255, 255, 0, 36, 195, 84, 77, 180, 65, 240, 191,
    ];
    let capture = |image: std::sync::Arc<GpuImage>| {
        let mut read = ReadbackTicket::capture(&ctx, image).unwrap();
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        read.try_finish(&ctx).unwrap().unwrap().pixels().to_vec()
    };
    let decode = |c: f64| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let encode = |c: f64| {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1. / 2.4) - 0.055
        }
    };
    // Independent f64 source/color reference. The legacy normalized texture
    // additionally rounds to f16 and uses hardware interpolation weights.
    let ideal = |space: ColorSpace, sampling: Sampling| {
        let wrap_x = matches!(sampling, Sampling::RepeatLinear | Sampling::RepeatLinearX);
        let wrap_y = matches!(sampling, Sampling::RepeatLinear | Sampling::RepeatLinearY);
        let texel = |x: i32, y: i32| {
            let x = if wrap_x {
                x.rem_euclid(2)
            } else {
                x.clamp(0, 1)
            };
            let y = if wrap_y {
                y.rem_euclid(2)
            } else {
                y.clamp(0, 1)
            };
            let at = (y * 2 + x) as usize * 4;
            let alpha = pixels[at + 3] as f64 / 255.;
            std::array::from_fn::<_, 4, _>(|i| {
                if i == 3 {
                    alpha
                } else {
                    let c = pixels[at + i] as f64 / 255.;
                    (if space == ColorSpace::Srgb {
                        decode(c)
                    } else {
                        c
                    }) * alpha
                }
            })
        };
        let tint = [0.8f32, 0.9, 1.].map(|c| decode(c as f64));
        let background = [0.12f32, 0.18, 0.27].map(|c| decode(c as f64));
        let mut out = Vec::new();
        for y in 0..73 {
            for x in 0..97 {
                let u = (-0.3f32 as f64)
                    + ((-0.3f32 + 1.6f32) as f64 - (-0.3f32 as f64)) * (x as f64 + 0.5) / 97.;
                let v = (-0.2f32 as f64)
                    + ((-0.2f32 + 1.4f32) as f64 - (-0.2f32 as f64)) * (y as f64 + 0.5) / 73.;
                let p = [u * 2. - 0.5, v * 2. - 0.5];
                let [i, j] = p.map(|v| v.floor() as i32);
                let [fx, fy] = p.map(|v| v - v.floor());
                let corners = [
                    texel(i, j),
                    texel(i + 1, j),
                    texel(i, j + 1),
                    texel(i + 1, j + 1),
                ];
                let weights = [
                    (1. - fx) * (1. - fy),
                    fx * (1. - fy),
                    (1. - fx) * fy,
                    fx * fy,
                ];
                let color: [f64; 4] = std::array::from_fn(|c| {
                    corners.iter().zip(weights).map(|(p, w)| p[c] * w).sum()
                });
                let alpha = color[3] * 0.7f32 as f64;
                for c in 0..3 {
                    let linear = color[c] * tint[c] * 0.7f32 as f64 + background[c] * (1. - alpha);
                    out.push((encode(linear).clamp(0., 1.) * 255.).round() as u8);
                }
                out.push(255);
            }
        }
        out
    };
    let mut results = Vec::new();
    for (name, format, space) in [
        (
            "srgb texture",
            wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorSpace::Srgb,
        ),
        (
            "encoded unorm texture",
            wgpu::TextureFormat::Rgba8Unorm,
            ColorSpace::Srgb,
        ),
        (
            "linear straight texture",
            wgpu::TextureFormat::Rgba8Unorm,
            ColorSpace::Linear,
        ),
    ] {
        let mut asset = ImageAsset::rgba(2, 2, pixels.clone()).unwrap();
        asset.color_space = space;
        let reference = images.upload_linear(&asset).unwrap();
        let raw = GpuImage::new(
            &ctx,
            ImageDescriptor {
                width: 2,
                height: 2,
                format,
                color_space: space,
                alpha_mode: AlphaMode::Straight,
            },
        )
        .unwrap();
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &raw.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(2),
            },
            raw.texture.size(),
        );
        let raw = images.insert(raw).unwrap();
        for sampling in [
            Sampling::Linear,
            Sampling::RepeatLinear,
            Sampling::RepeatLinearX,
            Sampling::RepeatLinearY,
            Sampling::Nearest,
        ] {
            let mut outputs = Vec::new();
            for handle in [reference, raw] {
                let mut list = PaintList::default();
                list.ops.push(PaintOp::Image {
                    image: handle,
                    rect: Rect::new(0., 0., 97., 73.),
                    uv: Rect::new(-0.3, -0.2, 1.6, 1.4),
                    sampling,
                    tint: [0.8, 0.9, 1., 0.7],
                });
                let output = GpuImage::new(&ctx, ImageDescriptor::rgba(97, 73)).unwrap();
                let before = ctx.transfers();
                renderer
                    .render(
                        &list,
                        &images,
                        &output.view,
                        output.descriptor.format,
                        (97, 73),
                        1.,
                        [0.12, 0.18, 0.27, 1.],
                    )
                    .unwrap();
                assert_eq!(
                    ctx.transfers(),
                    before,
                    "image composition maps/uploads no viewport colors"
                );
                outputs.push(capture(output));
            }
            let largest = outputs[0]
                .iter()
                .zip(&outputs[1])
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            if sampling != Sampling::Nearest {
                let expected = ideal(space, sampling);
                let error = outputs[1]
                    .iter()
                    .zip(&expected)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(
                    error <= 1,
                    "{name} {sampling:?}: raw GPU result differs from f64 linear-premultiplied reference by {error}"
                );
            }
            results.push((name, sampling, largest));
        }
    }
    // f16 normalization and hardware filter-weight precision can add one more
    // output unit at repeat seams. Raw results independently satisfy <=1 above.
    assert!(
        results.iter().all(|(_, _, error)| *error <= 2),
        "raw GPU sources differ from normalized reference: {results:?}"
    );
}

#[test]
#[ignore = "requires native GPU"]
fn opaque_display_policy_preserves_encoded_blending_groups_and_surface_formats() {
    let ctx = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(ctx.clone());
    let mut renderer = PaintRenderer::new(ctx.clone()).unwrap();
    let mut list = PaintList::default();
    list.ops.push(PaintOp::DisplayCompositing(
        DisplayCompositing::LegacyOpaque,
    ));
    list.rect(Rect::new(0., 0., 16., 8.), 0., [0.1, 0.1, 0.1, 1.]);
    list.rect(Rect::new(0., 0., 8., 8.), 0., [1., 0., 0., 0.5]);
    list.ops.push(PaintOp::PushClip(Rect::new(8., 0., 8., 8.)));
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.rect(Rect::new(8., 0., 8., 8.), 0., [1., 0., 0., 1.]);
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.rect(Rect::new(8., 0., 8., 8.), 0., [0., 0., 1., 1.]);
    list.ops.push(PaintOp::PopOpacity);
    list.ops.push(PaintOp::PopOpacity);
    list.ops.push(PaintOp::PopClip);
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let output = GpuImage::new(
            &ctx,
            ImageDescriptor {
                width: 16,
                height: 8,
                format,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Opaque,
            },
        )
        .unwrap();
        let before = ctx.transfers();
        renderer
            .render(&list, &images, &output.view, format, (16, 8), 1., [0.; 4])
            .unwrap();
        assert_eq!(
            ctx.transfers(),
            before,
            "compatibility composition remains GPU-only"
        );
        let mut read = ReadbackTicket::capture(&ctx, output).unwrap();
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let image = read.try_finish(&ctx).unwrap().unwrap();
        let pixel = |x: usize| &image.pixels()[(3 * 16 + x) * 4..(3 * 16 + x + 1) * 4];
        for (x, expected) in [(3, [140u8, 13, 13, 255]), (12, [77u8, 13, 77, 255])] {
            assert!(
                pixel(x)
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "{format:?}: {x}: {:?} vs {expected:?}",
                pixel(x)
            );
        }
    }
    let mut invalid = list.clone();
    invalid
        .ops
        .insert(0, PaintOp::PushClip(Rect::new(0., 0., 16., 8.)));
    invalid.ops.push(PaintOp::PopClip);
    assert!(
        invalid.validate().is_err(),
        "display encoding cannot be installed by a nested widget"
    );
    let output = GpuImage::new(&ctx, ImageDescriptor::rgba(16, 8)).unwrap();
    // A renderer can return to its standard linear policy on the next frame.
    list.ops.remove(0);
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (16, 8),
            1.,
            [0.; 4],
        )
        .unwrap();
    let mut read = ReadbackTicket::capture(&ctx, output).unwrap();
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let image = read.try_finish(&ctx).unwrap().unwrap();
    assert!(
        image.pixels()[(3 * 16 + 3) * 4] > 180,
        "standard linear blending remains available and distinct"
    );
    // Browser canvases expose non-sRGB base textures. A declared sRGB attachment
    // view must store the same encoded pixels as the native sRGB attachment,
    // including translucent/group composition, without an application readback.
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let render_format = format.add_srgb_suffix();
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas-compatible-base"),
            size: wgpu::Extent3d {
                width: 16,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[render_format],
        });
        let output = std::sync::Arc::new(
            GpuImage::from_texture(
                &ctx,
                texture,
                ImageDescriptor {
                    width: 16,
                    height: 8,
                    format,
                    color_space: ColorSpace::Srgb,
                    alpha_mode: AlphaMode::Opaque,
                },
            )
            .unwrap(),
        );
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(render_format),
            ..Default::default()
        });
        let before = ctx.transfers();
        renderer
            .render(&list, &images, &view, render_format, (16, 8), 1., [0.; 4])
            .unwrap();
        assert_eq!(ctx.transfers(), before);
        let mut read = ReadbackTicket::capture(&ctx, output).unwrap();
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let actual = read.try_finish(&ctx).unwrap().unwrap();
        assert!(
            actual
                .pixels()
                .iter()
                .zip(image.pixels())
                .all(|(a, b)| a.abs_diff(*b) <= 1),
            "{format:?} sRGB view must preserve native linear composition"
        );
    }
}

#[test]
#[ignore = "requires native GPU"]
fn pixel_images_match_left_edge_source_resizing_at_fractional_densities() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    for (sw, sh, dw, dh) in [
        (64, 32, 40, 40),
        (63, 29, 40, 40),
        (7, 5, 52, 32),
        (1, 1, 40, 40),
    ] {
        let mut pixels = vec![0; (sw * sh * 4) as usize];
        for y in 0..sh {
            for x in 0..sw {
                let i = ((y * sw + x) * 4) as usize;
                pixels[i..i + 4].copy_from_slice(&[
                    ((x * 13 + 70) % 256) as u8,
                    ((y * 17 + 40) % 256) as u8,
                    ((x * 5 + y * 7 + 50) % 256) as u8,
                    [0, 64, 200, 255][((x + y) % 4) as usize],
                ]);
            }
        }
        let source = ImageAsset::rgba(sw, sh, pixels.clone()).unwrap();
        let handle = images.upload_linear(&source).unwrap();
        for density in [1., 1.25, 1.5, 2.] {
            let size = (
                ((dw + 8) as f32 * density) as u32,
                ((dh + 8) as f32 * density) as u32,
            );
            let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            let mut list = PaintList::default();
            list.ops.push(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque,
            ));
            list.ops.push(PaintOp::PixelImage {
                image: handle,
                rect: Rect::new(4., 4., dw as f32, dh as f32),
                tint: [1.; 4],
            });
            assert_eq!(list.image_handles().collect::<Vec<_>>(), vec![handle]);
            let before = gpu.transfers();
            renderer
                .render(
                    &list,
                    &images,
                    &output.view,
                    output.descriptor.format,
                    size,
                    density,
                    [12. / 255., 24. / 255., 36. / 255., 1.],
                )
                .unwrap();
            assert_eq!(gpu.transfers(), before);
            let mut read = ReadbackTicket::capture(&gpu, output).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
            let image = read.try_finish(&gpu).unwrap().unwrap();
            let physical = ((dw as f32 * density) as u32, (dh as f32 * density) as u32);
            let origin = (4. * density) as u32;
            let sx = sw as f32 / physical.0 as f32;
            let sy = sh as f32 / physical.1 as f32;
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let mut expected = [12u8, 24, 36, 255];
                    if x >= origin
                        && y >= origin
                        && x < origin + physical.0
                        && y < origin + physical.1
                    {
                        let xx = (((x - origin) as f32 * sx) as u32).min(sw - 1);
                        let yy = (((y - origin) as f32 * sy) as u32).min(sh - 1);
                        let src = &pixels
                            [((yy * sw + xx) * 4) as usize..((yy * sw + xx + 1) * 4) as usize];
                        for c in 0..3 {
                            expected[c] = (src[c] as f32 * src[3] as f32 / 255.
                                + expected[c] as f32 * (1. - src[3] as f32 / 255.))
                                .round() as u8;
                        }
                    }
                    let actual = &image.pixels()
                        [((y * size.0 + x) * 4) as usize..((y * size.0 + x + 1) * 4) as usize];
                    assert!(
                        actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                        "source {sw}x{sh}, target {dw}x{dh}, density {density}, ({x},{y}): {actual:?}, expected {expected:?}"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "requires native GPU"]
fn opaque_glyph_boxes_blit_cached_swash_coverage_in_order_over_supplied_backgrounds() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut ctx = theframework::prelude::TheContext::new(64, 40, 1.);
    let run = ctx.text.layout("O", "Roboto", 20., None).runs.remove(0);
    let first = run.glyphs[0];
    let mut second = first;
    second.position[0] += 5.;
    let run = std::sync::Arc::new(GlyphRun {
        font: run.font.clone(),
        size: run.size,
        variations: run.variations.clone(),
        glyphs: std::sync::Arc::from([first, second]),
    });
    let origin = [5., 3.];
    let background = [20u8, 40, 70, 17];
    let foreground = [210u8, 20, 90, 80];
    let clear = [12u8, 24, 36, 255];
    let mut list = PaintList::default();
    list.ops.push(PaintOp::DisplayCompositing(
        DisplayCompositing::LegacyOpaque,
    ));
    list.ops.push(PaintOp::OpaqueGlyphRun {
        run: run.clone(),
        origin,
        color: foreground.map(|v| v as f32 / 255.),
        background: background.map(|v| v as f32 / 255.),
    });
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(64, 40)).unwrap();
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (64, 40),
            1.,
            clear.map(|v| v as f32 / 255.),
        )
        .unwrap();
    assert_eq!(renderer.frame_stats().glyph_quads, 2);
    assert_eq!(
        renderer.frame_stats().draw_calls,
        2,
        "one atlas batch plus the display blit"
    );
    let warm = gpu.transfers();
    let rasterizations = renderer.glyphs.rasterizations;
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (64, 40),
            1.,
            clear.map(|v| v as f32 / 255.),
        )
        .unwrap();
    assert_eq!(gpu.transfers(), warm);
    assert_eq!(renderer.glyphs.rasterizations, rasterizations);
    let mut expected = vec![clear; 64 * 40];
    let mut previous = vec![0u8; 64 * 40];
    let mut erased = 0;
    for glyph in run.glyphs.iter() {
        let physical = [origin[0] + glyph.position[0], origin[1] + glyph.position[1]];
        let (_, bitmap) = renderer.glyphs.glyph(&run, glyph.id, 1., physical).unwrap();
        assert!(!bitmap.color);
        let left = physical[0].floor() as i32 + bitmap.left;
        let top = physical[1].floor() as i32 - bitmap.top;
        for y in 0..bitmap.height {
            for x in 0..bitmap.width {
                let index = ((top + y as i32) * 64 + left + x as i32) as usize;
                let mask = bitmap.pixels[(y * bitmap.width + x) as usize];
                if previous[index] > 0 && mask == 0 {
                    erased += 1;
                }
                previous[index] = mask;
                expected[index] = [0, 0, 0, 255];
                for c in 0..3 {
                    expected[index][c] = (foreground[c] as f32 * mask as f32 / 255.
                        + background[c] as f32 * (1. - mask as f32 / 255.))
                        .round() as u8;
                }
            }
        }
    }
    assert!(
        erased > 0,
        "overlapping boxes must replace earlier coverage with background"
    );
    let mut read = ReadbackTicket::capture(&gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let image = read.try_finish(&gpu).unwrap().unwrap();
    for (i, (actual, expected)) in image.pixels().chunks_exact(4).zip(expected).enumerate() {
        assert!(
            actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
            "glyph pixel {i}: {actual:?} vs {expected:?}"
        );
    }
}
