#![cfg(feature = "render-wgpu")]
use renderkit::*;
use theframework::{paint::*, render_wgpu::PaintRenderer};
use theframework_core::{TheDim, TheRGBABuffer};

#[test]
#[ignore = "requires native GPU"]
fn pixel_images_match_actual_cpu_buffer_composition_at_odd_origins_and_extents() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    for (sw, sh, x, y, width, height) in [
        (17, 11, 5, 3, 17, 11),
        (11, 7, 3, 5, 11, 7),
        (11, 7, 5, 3, 17, 11),
        (17, 11, 3, 5, 11, 7),
        (63, 29, -3, -5, 43, 23),
    ] {
        let pixels: Vec<_> = (0..sw * sh)
            .flat_map(|i| {
                [
                    (i * 73 % 256) as u8,
                    (i * 97 % 256) as u8,
                    (i * 179 % 256) as u8,
                    255,
                ]
            })
            .collect();
        let source = TheRGBABuffer::from(pixels.clone(), sw, sh);
        let handle = images
            .upload_linear(&ImageAsset::rgba(sw, sh, pixels).unwrap())
            .unwrap();
        for density in [1., 1.25, 1.5, 1.75, 2.] {
            // At unit density this legacy helper's explicit-resize fallback
            // uses rounded sampling, a separate contract from its RenderView
            // copy/composite path. Exercise that path with equal-sized sources
            // at all five densities, and resized/clipped inputs above unit DPI.
            if density == 1. && (sw as i32 != width || sh as i32 != height || x < 0 || y < 0) {
                continue;
            }
            let mut cpu = TheRGBABuffer::new(TheDim::sized(48, 32));
            cpu.set_render_scale(density);
            for pixel in cpu.pixels_mut().chunks_exact_mut(4) {
                pixel.copy_from_slice(&[12, 24, 36, 255]);
            }
            source.scaled_into_using_dim(&mut cpu, &TheDim::rect(x, y, width, height));
            let size = (cpu.pixel_width() as u32, cpu.pixel_height() as u32);
            let output = GpuImage::new(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            let mut list = PaintList::default();
            list.ops.push(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque,
            ));
            list.ops.push(PaintOp::PushTransform(Transform::translate(
                x as f32, y as f32,
            )));
            list.ops.push(PaintOp::PixelImage {
                image: handle,
                rect: Rect::new(0., 0., width as f32, height as f32),
                tint: [1.; 4],
            });
            list.ops.push(PaintOp::PopTransform);
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
            let actual = read.try_finish(&gpu).unwrap().unwrap();
            if let Some((index, (actual, expected))) = actual
                .pixels()
                .iter()
                .zip(cpu.pixels())
                .enumerate()
                .find(|(_, (a, b))| a != b)
            {
                panic!(
                    "source={sw}x{sh} destination=({x},{y},{width},{height}) density={density} byte={index}: actual={actual} expected={expected}"
                );
            }
        }
        images.remove(handle);
    }
}

#[test]
#[ignore = "requires native GPU"]
fn local_pixel_canvas_then_parent_copy_matches_actual_cpu_rounding_at_all_densities() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let pixels: Vec<_> = (0..17 * 11)
        .flat_map(|i| {
            [
                (i * 73 % 256) as u8,
                (i * 97 % 256) as u8,
                (i * 179 % 256) as u8,
                255,
            ]
        })
        .collect();
    let cpu_source = TheRGBABuffer::from(pixels.clone(), 17, 11);
    let source = images
        .upload_linear(&ImageAsset::rgba(17, 11, pixels).unwrap())
        .unwrap();
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        let mut local = TheRGBABuffer::new(TheDim::sized(17, 11));
        local.set_render_scale(density);
        cpu_source.scaled_into_using_dim(&mut local, &TheDim::sized(17, 11));
        let mut cpu = TheRGBABuffer::new(TheDim::sized(48, 32));
        cpu.set_render_scale(density);
        for pixel in cpu.pixels_mut().chunks_exact_mut(4) {
            pixel.copy_from_slice(&[12, 24, 36, 255]);
        }
        cpu.copy_into(5, 3, &local);

        let target = GpuImage::render_target(
            &gpu,
            ImageDescriptor {
                width: local.pixel_width() as u32,
                height: local.pixel_height() as u32,
                format: wgpu::TextureFormat::Rgba8Unorm,
                ..ImageDescriptor::rgba(17, 11)
            },
        )
        .unwrap();
        let mut list = PaintList {
            ops: vec![
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
                PaintOp::PixelImage {
                    image: source,
                    rect: Rect::new(0., 0., 17., 11.),
                    tint: [1.; 4],
                },
            ],
        };
        renderer
            .render(
                &list,
                &images,
                &target.view,
                target.descriptor.format,
                (target.descriptor.width, target.descriptor.height),
                density,
                [0., 0., 0., 1.],
            )
            .unwrap();
        let local_image = images.insert(target).unwrap();
        let size = (cpu.pixel_width() as u32, cpu.pixel_height() as u32);
        let output = GpuImage::render_target(&gpu, ImageDescriptor::rgba(size.0, size.1)).unwrap();
        list.ops[1] = PaintOp::PixelImage {
            image: local_image,
            rect: Rect::new(5., 3., 17., 11.),
            tint: [1.; 4],
        };
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
        let mut read = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let actual = read.try_finish(&gpu).unwrap().unwrap();
        if let Some((index, (actual, expected))) = actual
            .pixels()
            .iter()
            .zip(cpu.pixels())
            .enumerate()
            .find(|(_, (a, b))| a != b)
        {
            panic!(
                "nested pixel canvas density={density} byte={index}: actual={actual} expected={expected}"
            );
        }
        images.remove(local_image);
    }
}
