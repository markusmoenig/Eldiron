//! Frozen shipped-map geometry isolates guide arithmetic from animated avatars,
//! world shading, HUD text and host pixel resampling.
use super::*;

#[test]
#[ignore = "native GPU; shipped terrain guides against actual CPU DDA"]
fn shipped_isometric_guides_match_actual_cpu_lines_with_and_without_batching() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_projects/UnendingNight.eldiron");
    let project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    let mut map = project.regions.into_iter().next().unwrap().map;
    map.grid_size = 25.;
    map.offset = Vec2::new(-725., -12.5);
    map.properties.set("iso_scale", Value::Float(4.));
    let (width, height) = (1169u32, 639u32);
    let background = [17u8, 34, 29, 255];
    let mut cpu = TheRGBABuffer::from(background.repeat((width * height) as usize), width, height);
    let mut list = PaintList {
        ops: vec![PaintOp::DisplayCompositing(
            theframework::paint::DisplayCompositing::LegacyOpaque,
        )],
    };
    let mut builder = rusterix::D2PreviewBuilder::new();
    builder.set_map_tool_type(MapToolType::Sector);
    builder.draw_grid = true;
    rusterix::isometric::editor::guide_segments(
        &map,
        Vec2::new(width as f32, height as f32),
        rusterix::isometric::Projection::editor(&map, Vec2::new(width as f32, height as f32)),
        builder.isometric_guides(),
        |from, to, color| {
            rusterix::isometric::line(&mut cpu, from, to, color);
            list.ops.push(PaintOp::SampledLine {
                from: from.into_array(),
                to: to.into_array(),
                color: color.map(|v| v as f32 / 255.),
            });
        },
    );
    assert!(
        list.ops.len() > 10_000,
        "fixture must exercise dense terrain guide segments"
    );
    let mut individual = PaintList::default();
    for op in &list.ops {
        individual.ops.push(op.clone());
        if matches!(op, PaintOp::SampledLine { .. }) {
            individual.ops.push(PaintOp::PushTransform(
                theframework::paint::Transform::default(),
            ));
            individual.ops.push(PaintOp::PopTransform);
        }
    }
    let context = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(context.clone());
    let mut renderer = PaintRenderer::new(context.clone()).unwrap();
    let mut results = Vec::new();
    for (name, paint) in [("batched", list), ("individual", individual)] {
        let image = GpuImage::render_target(
            &context,
            ImageDescriptor {
                width,
                height,
                format: wgpu::TextureFormat::Rgba8Unorm,
                ..ImageDescriptor::rgba(width, height)
            },
        )
        .unwrap();
        renderer
            .render(
                &paint,
                &images,
                &image.view,
                image.descriptor.format,
                (width, height),
                1.,
                background.map(|v| v as f32 / 255.),
            )
            .unwrap();
        let mut read = renderkit::ReadbackTicket::capture(&context, image).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let image = read.try_finish(&context).unwrap().unwrap();
        let max = image
            .pixels()
            .iter()
            .zip(cpu.pixels())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        let wrong = image
            .pixels()
            .chunks_exact(4)
            .zip(cpu.pixels().chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 1))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        eprintln!(
            "shipped guide {name}: ops={} draws={} max={max} pixels>1={} first={:?}",
            paint.ops.len(),
            renderer.frame_stats().draw_calls,
            wrong.len(),
            wrong
                .first()
                .map(|i| (*i % width as usize, *i / width as usize))
        );
        results.push((name, max, image));
    }
    assert_eq!(
        results[0].2.pixels(),
        results[1].2.pixels(),
        "batching must preserve exact guide pixels"
    );
    for (name, max, _) in results {
        assert!(max <= 1, "shipped guide {name} max CPU delta={max}");
    }
}

#[test]
#[ignore = "native GPU; actual CPU guides over hidden RGB and mixed alpha"]
fn source_guides_preserve_hidden_rgb_and_touched_alpha_at_all_densities() {
    let context = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(context.clone());
    let raw: Vec<_> = (0..17 * 11)
        .flat_map(|i| {
            [
                ((i * 37 + 17) % 256) as u8,
                ((i * 73 + 34) % 256) as u8,
                ((i * 97 + 29) % 256) as u8,
                [0, 64, 128, 255][i % 4],
            ]
        })
        .collect();
    let descriptor = ImageDescriptor {
        format: wgpu::TextureFormat::Rgba8Unorm,
        ..ImageDescriptor::rgba(17, 11)
    };
    let source = GpuImage::new(&context, descriptor).unwrap();
    context.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &source.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &raw,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(17 * 4),
            rows_per_image: Some(11),
        },
        source.texture.size(),
    );
    context.record_upload(raw.len() as u64);
    let source = images.insert(source).unwrap();
    let output = ViewImage {
        view: ViewId::new(),
        image: source,
        descriptor,
        revision: 1,
        content_revision: 1,
    };
    let mut cpu = TheRGBABuffer::from(raw, 17, 11);
    let mut source_paint = PaintList::default();
    source_paint.rect(Rect::new(0., 0., 17., 11.), 0., [0., 0., 0., 1.]);
    source_paint.ops.push(PaintOp::SpriteImage {
        image: source,
        origin: [0.; 2],
        scale: [1.; 2],
        tint: [1.; 4],
    });
    for (from, to, color) in [
        ([1., 2.], [14., 8.], [165, 193, 174, 50]),
        ([14., 8.], [1., 2.], [165, 193, 174, 50]),
        ([-2.5, 3.5], [20.25, 3.5], [90, 30, 220, 147]),
        ([4.5, -3.], [4.5, 12.], [20, 210, 60, 255]),
        ([2., 1.], [13., 7.], [240, 30, 90, 0]),
        ([8., 6.], [8., 6.], [255, 255, 255, 255]),
    ] {
        rusterix::isometric::line(&mut cpu, Vec2::from(from), Vec2::from(to), color);
        source_paint.ops.push(PaintOp::SampledLine {
            from,
            to,
            color: color.map(|v| v as f32 / 255.),
        });
    }
    let mut layer = Gpu::default();
    let mut renderer = PaintRenderer::new(context.clone()).unwrap();
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        let before = context.transfers().readback_bytes;
        let paint = layer
            .compose(source_paint.clone(), output, (17, 11), density, &mut images)
            .unwrap();
        assert_eq!(context.transfers().readback_bytes, before);
        let mut local = TheRGBABuffer::new(TheDim::sized(17, 11));
        local.set_render_scale(density);
        cpu.scaled_into_using_dim(&mut local, &TheDim::sized(17, 11));
        let mut reference = TheRGBABuffer::new(TheDim::sized(48, 32));
        reference.set_render_scale(density);
        reference.copy_into(5, 3, &local);
        let size = (
            reference.pixel_width() as u32,
            reference.pixel_height() as u32,
        );
        let reference_image = images
            .upload_linear(
                &renderkit::ImageAsset::rgba(size.0, size.1, reference.pixels().to_vec()).unwrap(),
            )
            .unwrap();
        let mut actual = PaintList {
            ops: vec![PaintOp::PushTransform(
                theframework::paint::Transform::translate(5., 3.),
            )],
        };
        actual.ops.extend(paint.ops);
        actual.ops.push(PaintOp::PopTransform);
        let mut reference = PaintList::default();
        reference.image(
            reference_image,
            Rect::new(0., 0., 48., 32.),
            Sampling::Nearest,
        );
        let mut captures = Vec::new();
        for paint in [&actual, &reference] {
            let target =
                GpuImage::render_target(&context, ImageDescriptor::rgba(size.0, size.1)).unwrap();
            renderer
                .render(
                    paint,
                    &images,
                    &target.view,
                    target.descriptor.format,
                    size,
                    density,
                    [0., 0., 0., 1.],
                )
                .unwrap();
            let mut ticket = renderkit::ReadbackTicket::capture(&context, target).unwrap();
            context
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
            captures.push(ticket.try_finish(&context).unwrap().unwrap());
        }
        let max = captures[0]
            .pixels()
            .iter()
            .zip(captures[1].pixels())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        eprintln!("source guide hidden RGB density={density}: max actual CPU delta={max}");
        assert!(
            max <= 1,
            "source alpha/guide composition density={density}, max delta={max}"
        );
        images.remove(reference_image);
    }
    layer.clear(&mut images);
    assert!(layer.cached.is_none() && layer.renderer.is_none() && layer.alpha.is_none());
}
