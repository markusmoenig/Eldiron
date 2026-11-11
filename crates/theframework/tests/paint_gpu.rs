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
