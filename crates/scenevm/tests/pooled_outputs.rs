#![cfg(feature = "gpu")]
use renderkit::{GpuContext, GpuImage, ReadbackTicket, wgpu};
use scenevm::{Atom, RenderMode, SceneVM, prelude::Vec4, render_output::SceneRenderTarget};
use std::sync::{Arc, mpsc};
fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    ticket
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}
fn capture_mip(context: &GpuContext, image: &GpuImage, mip: u32) -> Vec<u8> {
    let width = (image.texture.width() >> mip).max(1);
    let height = (image.texture.height() >> mip).max(1);
    let row = (width * 4).div_ceil(256) * 256;
    let bytes = u64::from(row) * u64::from(height);
    let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("explicit scene mip test capture"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &image.texture,
            mip_level: mip,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    context.record_capture(bytes);
    context.queue.submit([encoder.finish()]);
    let (tx, rx) = mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range();
    let result = data
        .chunks_exact(row as usize)
        .flat_map(|row| row[..width as usize * 4].iter().copied())
        .collect();
    drop(data);
    buffer.unmap();
    result
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn retired_scene_targets_wait_for_outputs_and_reset_all_mips() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_target_cache_limits(1024 * 1024, 8);
    let mut scene = SceneVM::new_with_context(context.clone(), 17, 9);
    scene.execute(Atom::SetRenderMode(RenderMode::Raster2D));
    scene.execute(Atom::SetBackground(Vec4::new(1., 0., 0., 1.)));
    let mut first = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    assert!(!first.has_cpu_storage() && !first.has_readback_storage());
    assert_eq!(first.image().texture.mip_level_count(), 5);
    assert!(
        first
            .image()
            .texture
            .usage()
            .contains(wgpu::TextureUsages::STORAGE_BINDING)
    );
    assert_eq!(
        context.target_pool_usage().cached_bytes,
        784,
        "account all five RGBA mips"
    );
    let red = scene.render_gpu(&mut first, 1).unwrap();
    // Deliberately poison a lower mip, flush the explicit write, then retire its
    // owner. A newly acquired target must restore fresh transparent storage.
    context.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &red.image.texture,
            mip_level: 2,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255; 32],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        },
    );
    context.queue.submit([]);
    drop(first);
    let mut second = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    scene.execute(Atom::SetBackground(Vec4::new(0., 0., 1., 1.)));
    let blue = scene.render_gpu(&mut second, 2).unwrap();
    assert_eq!(
        context.target_pool_usage().allocations,
        2,
        "live red output pins its old storage"
    );
    assert_eq!(context.transfers().readback_bytes, 0);
    assert!(
        capture(&context, red.image)
            .chunks_exact(4)
            .all(|p| p == [255, 0, 0, 255])
    );
    assert!(
        capture(&context, blue.image.clone())
            .chunks_exact(4)
            .all(|p| p == [0, 0, 255, 255])
    );
    let before = context.target_pool_usage();
    let mut reused = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    assert_eq!(context.target_pool_usage().allocations, before.allocations);
    assert_eq!(context.target_pool_usage().hits, before.hits + 1);
    for mip in 0..5 {
        assert!(
            capture_mip(&context, &reused.image(), mip)
                .iter()
                .all(|v| *v == 0)
        );
    }
    context.set_target_cache_limits(0, 0); // Eviction cannot invalidate either live view.
    scene.execute(Atom::SetBackground(Vec4::new(0., 1., 0., 1.)));
    let green = scene.render_gpu(&mut reused, 3).unwrap();
    assert!(
        capture(&context, green.image)
            .chunks_exact(4)
            .all(|p| p == [0, 255, 0, 255])
    );
    assert!(
        capture(&context, blue.image)
            .chunks_exact(4)
            .all(|p| p == [0, 0, 255, 255])
    );
    assert_eq!(context.target_pool_usage().cached_bytes, 0);
}

#[test]
#[ignore = "requires native GPU; explicit reference readback comparison"]
fn pooled_output_matches_reference_capture_in_each_renderer_mode() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    for mode in [
        RenderMode::Compute2D,
        RenderMode::Raster2D,
        RenderMode::Compute3D,
        RenderMode::Raster3D,
        RenderMode::Sdf,
    ] {
        let setup = || {
            let mut scene = SceneVM::new_with_context(context.clone(), 17, 9);
            scene.execute(Atom::SetRenderMode(mode));
            scene.execute(Atom::SetBackground(Vec4::new(0.2, 0.4, 0.7, 1.)));
            scene
        };
        let mut pooled = setup();
        let mut reference = setup();
        let mut target = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
        let output = pooled.render_gpu(&mut target, 1).unwrap();
        let actual = capture(&context, output.image);
        let mut expected = vec![0; 17 * 9 * 4];
        assert!(matches!(
            reference.render_frame(&mut expected, 17, 9),
            scenevm::RenderResult::Presented
        ));
        assert!(
            expected.iter().any(|v| *v != 0),
            "reference {mode:?} produced no background"
        );
        assert_eq!(actual.len(), expected.len());
        let delta = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs())
            .max()
            .unwrap();
        assert!(
            delta <= 1,
            "pooled {mode:?} versus explicit reference, max delta {delta}"
        );
        eprintln!("Pooled {mode:?} versus explicit reference: max delta {delta}");
        assert!(!target.has_cpu_storage() && !target.has_readback_storage());
    }
}
