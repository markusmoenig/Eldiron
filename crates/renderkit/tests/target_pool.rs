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

#[test]
#[ignore = "native GPU; source array pool extent and accounting contract"]
fn source_array_layers_and_mips_are_separate_owned_storage() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_target_cache_limits(0, 0);
    let descriptor = TargetDescriptor {
        array_layers: 3,
        mip_levels: 2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        ..TargetDescriptor::color(8, 4, wgpu::TextureFormat::Rgba8Unorm)
    };
    let array = context.acquire_target(descriptor).unwrap();
    assert_eq!(array.texture().depth_or_array_layers(), 3);
    assert_eq!(context.target_pool_usage().live_bytes, 480);
    assert!(
        GpuImage::from_target(
            &context,
            array.clone(),
            ImageDescriptor {
                width: 8,
                height: 4,
                format: descriptor.format,
                ..ImageDescriptor::rgba(8, 4)
            },
        )
        .is_err(),
        "ordinary image bindings reject array storage"
    );
    let before = context.target_pool_usage();
    for invalid in [
        TargetDescriptor {
            array_layers: 0,
            ..descriptor
        },
        TargetDescriptor {
            array_layers: context.device.limits().max_texture_array_layers + 1,
            ..descriptor
        },
        TargetDescriptor {
            samples: 4,
            mip_levels: 1,
            ..descriptor
        },
    ] {
        assert!(context.acquire_target(invalid).is_err());
        assert_eq!(context.target_pool_usage(), before);
    }
    drop(array);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 0);
}

#[test]
#[ignore = "native GPU; compatible storage/sRGB array views"]
fn source_color_views_preserve_bytes_and_sampling_without_duplicate_storage() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let descriptor = TargetDescriptor {
        array_layers: 2,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        ..TargetDescriptor::color(2, 2, wgpu::TextureFormat::Rgba8Unorm)
    };
    let source = context.acquire_target(descriptor).unwrap();
    context.queue.write_texture(
        source.texture().as_image_copy(),
        &[128, 64, 32, 255].repeat(8),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        source.texture().size(),
    );
    let view = source.texture().create_view(&wgpu::TextureViewDescriptor {
        format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
        ..Default::default()
    });
    assert_eq!(context.target_pool_usage().live_bytes, 32);
    assert_eq!(context.target_pool_usage().allocations, 1);
    let mut target = TargetDescriptor::color(1, 1, wgpu::TextureFormat::Rgba8Unorm);
    target.usage |= wgpu::TextureUsages::STORAGE_BINDING;
    let output = GpuImage::from_target(
        &context,
        context.acquire_target(target).unwrap(),
        ImageDescriptor {
            format: target.format,
            color_space: ColorSpace::Linear,
            ..ImageDescriptor::rgba(1, 1)
        },
    )
    .unwrap();
    let shader = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pooled sRGB sampling contract"),
        source: wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var source:texture_2d_array<f32>; @group(0) @binding(1) var destination:texture_storage_2d<rgba8unorm,write>; @compute @workgroup_size(1) fn main(){textureStore(destination,vec2<i32>(0),textureLoad(source,vec2<i32>(0),1,0));}".into()),
    });
    let pipeline = context
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bindings = context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output.view),
                },
            ],
        });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    context.queue.submit([encoder.finish()]);
    assert_eq!(pixel(&context, output), [55, 13, 4, 255]);
    drop(source);
    let allocations = context.target_pool_usage().allocations;
    let reused = context.acquire_target(descriptor).unwrap();
    assert_eq!(context.target_pool_usage().allocations, allocations);
    assert_eq!(context.target_pool_usage().hits, 1);
    drop(reused);
    context.set_target_cache_limits(0, 0);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
}
