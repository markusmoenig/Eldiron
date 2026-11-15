use renderkit::*;
use std::sync::Arc;

fn clear(context: &GpuContext, image: &GpuImage, color: wgpu::Color) {
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &image.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    context.queue.submit([encoder.finish()]);
}
fn pixel(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    ticket.try_finish(context).unwrap().unwrap().pixels()[..4].to_vec()
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn leased_targets_do_not_alias_and_eviction_preserves_queued_work() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_target_cache_limits(1024, 2);
    let descriptor = ImageDescriptor {
        width: 8,
        height: 8,
        format: wgpu::TextureFormat::Rgba8Unorm,
        color_space: ColorSpace::Linear,
        alpha_mode: AlphaMode::Premultiplied,
    };
    let first = GpuImage::render_target(&context, descriptor).unwrap();
    let second = GpuImage::render_target(&context, descriptor).unwrap();
    assert_eq!(context.target_pool_usage().allocations, 2);
    assert_eq!(
        context.target_pool_usage().hits,
        0,
        "both live leases need distinct storage"
    );
    clear(&context, &first, wgpu::Color::RED);
    clear(&context, &second, wgpu::Color::GREEN);
    drop(first); // Its clear was submitted; future reuse is ordered on this queue.
    let reused = GpuImage::render_target(&context, descriptor).unwrap();
    assert_eq!(context.target_pool_usage().allocations, 2);
    assert_eq!(context.target_pool_usage().hits, 1);
    clear(&context, &reused, wgpu::Color::BLUE);
    // Evict both while their images and queue references remain live.
    context.set_target_cache_limits(0, 0);
    assert_eq!(context.target_pool_usage().cached_textures, 0);
    assert_eq!(context.target_pool_usage().cached_bytes, 0);
    assert_eq!(context.target_pool_usage().live_bytes, 512);
    assert_eq!(context.target_pool_usage().live_allocations, 2);
    assert_eq!(context.target_pool_usage().peak_live_bytes, 512);
    assert_eq!(pixel(&context, second), [0, 255, 0, 255]);
    assert_eq!(pixel(&context, reused), [0, 0, 255, 255]);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 0);
    let uncached = GpuImage::render_target(&context, descriptor).unwrap();
    clear(&context, &uncached, wgpu::Color::WHITE);
    assert_eq!(context.target_pool_usage().uncached_allocations, 1);
    assert_eq!(pixel(&context, uncached), [255; 4]);
    let before = context.target_pool_usage();
    assert!(
        context
            .acquire_target(TargetDescriptor::color(0, 8, descriptor.format))
            .is_err()
    );
    assert_eq!(
        context.target_pool_usage(),
        before,
        "invalid targets allocate nothing"
    );
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    let rebuilt = GpuImage::render_target(&replacement, descriptor).unwrap();
    assert_ne!(rebuilt.device_generation(), context.generation());
    assert_eq!(replacement.target_pool_usage().allocations, 1);
    assert_eq!(replacement.target_pool_usage().hits, 0);
    clear(&replacement, &rebuilt, wgpu::Color::RED);
    assert_eq!(pixel(&replacement, rebuilt), [255, 0, 0, 255]);
}
