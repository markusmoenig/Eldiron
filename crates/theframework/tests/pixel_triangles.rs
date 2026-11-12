#![cfg(all(feature = "ui", feature = "render-wgpu"))]
use renderkit::*;
use theframework::{paint::*, render_wgpu::PaintRenderer};

fn capture(gpu: &GpuContext, image: std::sync::Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(gpu, image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
// Independent original scalar pixel-center/edge oracle, including inclusive
// shared-edge overdraw and the legacy per-triangle encoded RGB truncation.
fn oracle(pixels: &mut [u8], size: (u32, u32), points: [Point; 3], color: [u8; 4], clip: Rect) {
    let edge = |a: Point, b: Point, p: Point| {
        (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
    };
    let [a, b, c] = points;
    let min = [0, 1].map(|axis| {
        points
            .iter()
            .map(|p| p[axis])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as u32
    });
    let max = [0, 1].map(|axis| {
        points
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
    });
    let alpha = color[3] as f32 / 255.;
    for y in min[1]..size.1 {
        for x in min[0]..size.0 {
            if x as f32 > max[0] || y as f32 > max[1] {
                continue;
            }
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            if !clip.contains(p) {
                continue;
            }
            let e = [edge(a, b, p), edge(b, c, p), edge(c, a, p)];
            if !e.iter().all(|v| *v >= 0.) && !e.iter().all(|v| *v <= 0.) {
                continue;
            }
            let offset = ((y * size.0 + x) * 4) as usize;
            for ch in 0..3 {
                pixels[offset + ch] =
                    (pixels[offset + ch] as f32 * (1. - alpha) + color[ch] as f32 * alpha) as u8;
            }
            pixels[offset + 3] = 255;
        }
    }
}
#[test]
fn pixel_coverage_commands_reject_nonfinite_geometry_and_color() {
    let good = [[0., 0.], [1., 0.], [0., 1.]];
    let mut list = PaintList::default();
    list.ops.push(PaintOp::PixelTriangle {
        points: good,
        color: [1.; 4],
    });
    list.validate().unwrap();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut points = good;
        points[0][0] = bad;
        list.ops[0] = PaintOp::PixelTriangle {
            points,
            color: [1.; 4],
        };
        assert!(list.validate().is_err());
        list.ops[0] = PaintOp::PixelTriangle {
            points: good,
            color: [1., bad, 1., 1.],
        };
        assert!(list.validate().is_err());
        list.ops[0] = PaintOp::PixelLine {
            from: [bad, 0.],
            to: [1., 1.],
            color: [1.; 4],
        };
        assert!(list.validate().is_err());
    }
}
#[test]
#[ignore = "actual GPU inclusive pixel triangles, clips, winding, overdraw, transforms and fractional density"]
fn gpu_pixel_triangles_preserve_inclusive_centers_and_shared_edge_order_at_all_densities() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let triangles = [
        [[4., 4.], [28., 4.], [28., 28.]],
        [[4., 4.], [28., 28.], [4., 28.]],
        [[40., 4.], [40., 28.], [64., 28.]],
        [[40., 4.], [64., 28.], [64., 4.]],
        [[72.25, 5.75], [96.75, 9.25], [80.5, 37.5]],
        [[110., 5.], [111., 5.], [110., 38.]],
        [[-20., -20.], [20., 3.], [10., 25.]],
        [[93., 60.], [93., 60.], [93., 60.]],
        [[100., 44.], [110., 54.], [120., 64.]],
        [[-100., 70.], [80., 90.], [60., 140.]],
        [[200., 200.], [250., 220.], [220., 250.]],
    ];
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        let size = (
            (160f32 * density).round() as u32,
            (128f32 * density).round() as u32,
        );
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
        for transform in [
            Transform::translate(1.25, 2.5),
            Transform([0., 1., -1., 0., 125.25, 2.5]),
        ] {
            let clip = Rect::new(2., 3., size.0 as f32 - 4., size.1 as f32 - 6.);
            let mut expected = [17, 29, 43, 255].repeat((size.0 * size.1) as usize);
            let mut list = PaintList::default();
            list.ops.push(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque,
            ));
            list.ops.push(PaintOp::PushClip(Rect::new(
                clip.x / density,
                clip.y / density,
                clip.width / density,
                clip.height / density,
            )));
            list.ops.push(PaintOp::PushTransform(transform));
            for (index, points) in triangles.iter().enumerate() {
                let color = [
                    80 + index as u8 * 10,
                    180 - index as u8 * 7,
                    55 + index as u8 * 11,
                    245,
                ];
                let physical = points.map(|p| transform.point(p).map(|v| v * density));
                oracle(&mut expected, size, physical, color, clip);
                list.ops.push(PaintOp::PixelTriangle {
                    points: *points,
                    color: color.map(|v| v as f32 / 255.),
                });
            }
            list.ops.extend([PaintOp::PopTransform, PaintOp::PopClip]);
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
                        [17. / 255., 29. / 255., 43. / 255., 1.],
                    )
                    .unwrap();
            }
            assert_eq!(gpu.transfers().uploaded_bytes, before.uploaded_bytes);
            assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
            for (index, (actual, expected)) in capture(&gpu, output.clone())
                .chunks_exact(4)
                .zip(expected.chunks_exact(4))
                .enumerate()
            {
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "density {density}, pixel ({},{}): {actual:?}/{expected:?}",
                    index % size.0 as usize,
                    index / size.0 as usize
                );
            }
        }
    }
}
#[test]
#[ignore = "actual GPU huge triangle support, clip and isolated common opacity"]
fn gpu_pixel_triangles_clip_huge_support_and_use_common_opacity_groups() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(96, 96)).unwrap();
    let mut list = PaintList::default();
    list.ops.extend([
        PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
        PaintOp::PushClip(Rect::new(20., 20., 10., 10.)),
        PaintOp::PushOpacity(0.5),
        PaintOp::PixelTriangle {
            points: [
                [-1_000_000_000., -1_000_000_000.],
                [1_000_000_000., -1_000_000_000.],
                [0., 1_000_000_000.],
            ],
            color: [224. / 255., 128. / 255., 64. / 255., 1.],
        },
        PaintOp::PopOpacity,
        PaintOp::PopClip,
    ]);
    renderer
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (96, 96),
            1.,
            [20. / 255., 40. / 255., 60. / 255., 1.],
        )
        .unwrap();
    let pixels = capture(&gpu, output);
    for y in 0..96 {
        for x in 0..96 {
            let expected = if (20..30).contains(&x) && (20..30).contains(&y) {
                [122, 84, 62, 255]
            } else {
                [20, 40, 60, 255]
            };
            let actual = &pixels[(x + y * 96) * 4..(x + y * 96 + 1) * 4];
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "{x}/{y}: {actual:?}/{expected:?}"
            );
        }
    }
}
