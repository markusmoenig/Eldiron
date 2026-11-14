#![cfg(feature = "render-wgpu")]
use renderkit::*;
use std::sync::Arc;
use theframework::{paint::*, render_wgpu::PaintRenderer};
fn list(reverse: bool) -> PaintList {
    let mut list = PaintList::default();
    let colors = if reverse {
        [[0., 0., 1., 1.], [1., 0., 0., 1.], [0., 1., 0., 1.]]
    } else {
        [[1., 0., 0., 1.], [0., 1., 0., 1.], [0., 0., 1., 1.]]
    };
    for (index, color) in colors.into_iter().enumerate() {
        list.ops.push(PaintOp::PushOpacity(0.5));
        if index == 0 {
            list.ops.push(PaintOp::PushOpacity(0.5));
        }
        list.rect(Rect::new(0., 0., 32., 32.), 0., color);
        if index == 0 {
            list.ops.push(PaintOp::PopOpacity);
        }
        list.ops.push(PaintOp::PopOpacity);
    }
    list
}
fn draw(
    context: &Arc<GpuContext>,
    renderer: &mut PaintRenderer,
    images: &ImageRegistry,
    output: &Arc<GpuImage>,
    reverse: bool,
) {
    renderer
        .render(
            &list(reverse),
            images,
            &output.view,
            output.descriptor.format,
            (output.descriptor.width, output.descriptor.height),
            1.,
            [0., 0., 0., 1.],
        )
        .unwrap();
    assert_eq!(context.transfers().readback_bytes, 0);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn nested_opacity_survives_warm_reuse_eviction_and_inflight_resizes() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(context.clone());
    let mut renderer = PaintRenderer::new(context.clone()).unwrap();
    let descriptor = |size| ImageDescriptor {
        width: size,
        height: size,
        format: wgpu::TextureFormat::Rgba8Unorm,
        color_space: ColorSpace::Linear,
        alpha_mode: AlphaMode::Premultiplied,
    };
    let output = GpuImage::new(&context, descriptor(32)).unwrap();
    draw(&context, &mut renderer, &images, &output, false);
    let warm = context.target_pool_usage();
    assert!(warm.allocations > 0);
    for _ in 0..64 {
        draw(&context, &mut renderer, &images, &output, false);
    }
    let reused = context.target_pool_usage();
    eprintln!("Warmed target storage over64frames: {warm:?} -> {reused:?}");
    assert_eq!(
        reused.allocations, warm.allocations,
        "warm frames reuse group/MSAA/stencil textures"
    );
    assert!(reused.hits > warm.hits);
    let mut frames = vec![(output, false)];
    // Force count/byte pressure, and submit several sizes without waiting. Draw
    // leases must protect sibling/nested groups before the parent submits; wgpu
    // must retain evicted storage through already queued passes afterward.
    context.set_target_cache_limits(16 * 1024, 2);
    for (index, size) in [40, 48, 56, 32, 64, 40].into_iter().enumerate() {
        let output = GpuImage::new(&context, descriptor(size)).unwrap();
        let reverse = index % 2 == 1;
        draw(&context, &mut renderer, &images, &output, reverse);
        let usage = context.target_pool_usage();
        assert!(usage.cached_bytes <= usage.byte_limit);
        assert!(usage.cached_textures <= usage.texture_limit);
        frames.push((output, reverse));
    }
    assert!(context.target_pool_usage().evictions > 0);
    assert!(context.target_pool_usage().uncached_allocations > 0);
    eprintln!(
        "Target pressure after queued resizes: {:?}",
        context.target_pool_usage()
    );
    // Explicit captures happen only after every ordinary frame has submitted.
    for (image, reverse) in frames {
        let width = image.descriptor.width as usize;
        let mut capture = ReadbackTicket::capture(&context, image).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(15)),
            })
            .unwrap();
        let result = capture.try_finish(&context).unwrap().unwrap();
        let offset = (12 * width + 12) * 4;
        let actual = &result.pixels()[offset..offset + 4];
        let expected: [u8; 4] = if reverse {
            [64, 128, 16, 255]
        } else {
            [16, 64, 128, 255]
        };
        for (a, b) in actual.iter().zip(expected) {
            assert!(
                (*a as i16 - b as i16).abs() <= 2,
                "{actual:?} versus {expected:?}"
            );
        }
    }
}
