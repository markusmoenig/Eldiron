#![cfg(feature = "render-wgpu")]
use renderkit::*;
use theframework::{paint::*, render_wgpu::PaintRenderer};

#[test]
#[ignore = "requires native GPU; compares fractional capped-DDA samples and repeated alpha blending"]
fn sampled_lines_preserve_cpu_f32_rounding_at_fractional_densities() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let (width, height) = (384usize, 256usize);
    let background = [17, 34, 29, 255];
    let mut logical = background.repeat(width * height);
    let mut list = PaintList::default();
    list.ops.push(PaintOp::DisplayCompositing(
        DisplayCompositing::LegacyOpaque,
    ));
    for (from, to, color) in [
        ([-40.25, -20.5], [600.25, 299.75], [100, 128, 70, 107]),
        ([370.5, -5.25], [20.25, 220.5], [100, 128, 70, 107]),
        ([20.25, 220.5], [370.5, -5.25], [100, 128, 70, 107]),
        ([0.49999997, 6.], [0.5, 200.25], [60, 80, 110, 255]),
        ([-8192.25, 200.5], [8192.25, 100.5], [220, 180, 80, 147]),
        ([300.5, 20.25], [20.25, 180.5], [200, 60, 140, 147]),
    ] {
        list.ops.push(PaintOp::SampledLine {
            from,
            to,
            color: color.map(|v| v as f32 / 255.),
        });
        let delta: [f32; 2] = [to[0] - from[0], to[1] - from[1]];
        let count = delta[0].abs().max(delta[1].abs()).ceil().min(8192.) as usize;
        for i in 0..=count {
            let t = i as f32 / count as f32;
            let [x, y] = [from[0] + delta[0] * t, from[1] + delta[1] * t].map(|v| v.round() as i32);
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let at = (y as usize * width + x as usize) * 4;
            for channel in 0..3 {
                logical[at + channel] = ((color[channel] as u32 * color[3] as u32
                    + logical[at + channel] as u32 * (255 - color[3] as u32)
                    + 127)
                    / 255) as u8;
            }
        }
    }
    for density in [1f32, 1.25, 1.5, 1.75, 2.] {
        let size = (
            (width as f32 * density) as u32,
            (height as f32 * density) as u32,
        );
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
        renderer
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                size,
                density,
                background.map(|v| v as f32 / 255.),
            )
            .unwrap();
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let capture = ticket.try_finish(&gpu).unwrap().unwrap();
        for y in 0..size.1 {
            for x in 0..size.0 {
                let lx = ((x as f32 + 0.5) / density).floor() as usize;
                let ly = ((y as f32 + 0.5) / density).floor() as usize;
                let expected = &logical[(ly * width + lx) * 4..][..4];
                let actual = &capture.pixels()[((y * size.0 + x) * 4) as usize..][..4];
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "density={density} pixel=({x},{y}): {actual:?}/{expected:?}"
                );
            }
        }
    }
}
