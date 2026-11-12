#![cfg(all(feature = "ui", feature = "render-wgpu"))]
use renderkit::*;
use theframework::{
    paint::*,
    prelude::{TheDim, TheRGBABuffer},
    render_wgpu::PaintRenderer,
};
fn capture(gpu: &GpuContext, output: std::sync::Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(gpu, output).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
fn physical(point: Point, transform: Transform, density: f32) -> [i32; 2] {
    transform.point(point).map(|v| (v * density).round() as i32)
}
fn compare(actual: &[u8], expected: &[u8]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (a, b)) in actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
    {
        assert_eq!(
            a[..3].iter().any(|v| *v > 4),
            b[..3].iter().any(|v| *v > 4),
            "coverage at pixel {index}: {a:?}/{b:?}"
        );
        assert!(
            a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1),
            "pixel {index}: {a:?}/{b:?}"
        );
    }
}
#[test]
#[ignore = "native GPU integer line coverage; every octant, ties, endpoints and fractional DPI"]
fn gpu_pixel_lines_preserve_symmetric_bresenham_coverage_at_fractional_densities() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    for density in [1., 1.25, 1.5, 2.] {
        let size = (
            (360f32 * density).round() as u32,
            (360f32 * density).round() as u32,
        );
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
        let mut reference = TheRGBABuffer::new(TheDim::sized(size.0 as i32, size.1 as i32));
        reference.fill([0, 0, 0, 255]);
        let transform = Transform([1., 0., 0., 1., 3.25, 4.5]);
        let mut list = PaintList::default();
        list.ops.push(PaintOp::PushTransform(transform));
        for dy in -8..=8 {
            for dx in -8..=8 {
                let from = [((dx + 8) * 20 + 9) as f32, ((dy + 8) * 20 + 9) as f32];
                let to = [from[0] + dx as f32, from[1] + dy as f32];
                let color = [(31 + dx + 8) as u8, (89 + dy + 8) as u8, 180, 255];
                let a = physical(from, transform, density);
                let b = physical(to, transform, density);
                reference.draw_line(a[0], a[1], b[0], b[1], color);
                list.ops.push(PaintOp::PixelLine {
                    from,
                    to,
                    color: color.map(|v| v as f32 / 255.),
                });
            }
        }
        for (from, to) in [
            ([-30., 8.], [30., 23.]),
            ([8., -30.], [23., 30.]),
            ([330., 350.], [380., 330.]),
            ([350., 330.], [330., 380.]),
        ] {
            let a = physical(from, transform, density);
            let b = physical(to, transform, density);
            reference.draw_line(a[0], a[1], b[0], b[1], [255, 255, 255, 255]);
            list.ops.push(PaintOp::PixelLine {
                from,
                to,
                color: [1.; 4],
            });
        }
        list.ops.push(PaintOp::PopTransform);
        list.validate().unwrap();
        let before = gpu.transfers();
        for _ in 0..2 {
            renderer
                .render(
                    &list,
                    &images,
                    &output.view,
                    output.descriptor.format,
                    size,
                    density,
                    [0., 0., 0., 1.],
                )
                .unwrap();
        }
        assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
        assert_eq!(
            gpu.transfers().uploaded_bytes,
            before.uploaded_bytes,
            "no source image or glyph is uploaded for lines"
        );
        compare(&capture(&gpu, output), reference.pixels());
    }
}
/// Exact closed form is used only as an independent oracle for lines whose
/// offscreen span would make the legacy iterative reference prohibitively long.
fn visible(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> bool {
    let dx = (b[0] as i64 - a[0] as i64).unsigned_abs();
    let dy = (b[1] as i64 - a[1] as i64).unsigned_abs();
    if dx == 0 && dy == 0 {
        return p == a;
    }
    let (major, minor, axis) = if dx >= dy { (dx, dy, 0) } else { (dy, dx, 1) };
    if p[axis] < a[axis].min(b[axis]) || p[axis] > a[axis].max(b[axis]) {
        return false;
    }
    let k = (p[axis] as i64 - a[axis] as i64).unsigned_abs();
    let q = ((k * minor + major / 2) / major) as i64;
    a[1 - axis] as i64 + if b[1 - axis] >= a[1 - axis] { q } else { -q } == p[1 - axis] as i64
}
#[test]
fn closed_form_line_oracle_matches_legacy_iteration_for_all_small_integer_endpoints() {
    for x in -4..=4 {
        for y in -4..=4 {
            for bx in -4..=4 {
                for by in -4..=4 {
                    let a = [x + 6, y + 6];
                    let b = [bx + 6, by + 6];
                    let mut buffer = TheRGBABuffer::new(TheDim::sized(13, 13));
                    buffer.draw_line(a[0], a[1], b[0], b[1], [255; 4]);
                    for py in 0..13 {
                        for px in 0..13 {
                            assert_eq!(
                                visible(a, b, [px, py]),
                                buffer.pixels()[((py * 13 + px) * 4) as usize] != 0,
                                "{a:?}/{b:?} at {px},{py}"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
#[ignore = "native GPU integer multiply/divide and clipped support for huge panned endpoints"]
fn gpu_pixel_lines_keep_original_phase_for_large_signed_endpoints_without_large_geometry() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(64, 64)).unwrap();
    for (a, b) in [
        ([-1_000_000, -999_983], [1_000_000, 999_991]),
        ([-1_000_000, 500_000], [1_000_000, -499_970]),
        (
            [-2_000_000_000, -1_000_000_000],
            [2_000_000_000, 1_000_000_000],
        ),
        ([i32::MIN, i32::MIN], [i32::MAX, i32::MAX]),
        ([1_000_000, 999_991], [-1_000_000, -999_983]),
        ([i32::MIN, 4], [i32::MAX, 4]),
        ([4, i32::MAX], [4, i32::MIN]),
    ] {
        let from = a.map(|v| v as f32);
        let to = b.map(|v| v as f32);
        let a = physical(from, Transform::default(), 1.);
        let b = physical(to, Transform::default(), 1.);
        let mut expected = vec![0; 64 * 64 * 4];
        for y in 0..64 {
            for x in 0..64 {
                let i = ((y * 64 + x) * 4) as usize;
                expected[i + 3] = 255;
                if visible(a, b, [x, y]) {
                    expected[i..i + 4].copy_from_slice(&[255; 4]);
                }
            }
        }
        let mut list = PaintList::default();
        list.ops.push(PaintOp::PixelLine {
            from,
            to,
            color: [1.; 4],
        });
        renderer
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (64, 64),
                1.,
                [0., 0., 0., 1.],
            )
            .unwrap();
        compare(&capture(&gpu, output.clone()), &expected);
    }
}
#[test]
#[ignore = "native GPU clips, scoped opacity and rotated endpoint mapping"]
fn gpu_pixel_lines_share_common_clip_transform_and_opacity_scopes() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(32, 32)).unwrap();
    let mut list = PaintList::default();
    list.ops.push(PaintOp::PushClip(Rect::new(8., 8., 8., 8.)));
    list.ops.push(PaintOp::PushOpacity(0.5));
    list.ops.push(PaintOp::PushTransform(Transform([
        0., 1., -1., 0., 12., 0.,
    ])));
    list.ops.push(PaintOp::PixelLine {
        from: [0., 0.],
        to: [31., 0.],
        color: [1., 0., 0., 1.],
    });
    list.ops
        .extend([PaintOp::PopTransform, PaintOp::PopOpacity, PaintOp::PopClip]);
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (32, 32),
            1.,
            [0., 0., 0., 1.],
        )
        .unwrap();
    let pixels = capture(&gpu, output);
    for y in 0..32 {
        for x in 0..32 {
            let pixel = &pixels[((y * 32 + x) * 4) as usize..((y * 32 + x + 1) * 4) as usize];
            if x == 12 && (8..16).contains(&y) {
                assert!(pixel[0].abs_diff(188) <= 1, "{pixel:?}");
            } else {
                assert_eq!(pixel, &[0, 0, 0, 255]);
            }
        }
    }
}
