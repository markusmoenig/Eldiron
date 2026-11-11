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
