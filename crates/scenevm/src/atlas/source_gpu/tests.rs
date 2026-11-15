use super::*;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ReadbackTicket, TargetDescriptor};

fn pixels(context: &GpuContext, source: &AtlasGpuSnapshot, material: bool) -> Vec<u8> {
    let index = usize::from(material);
    let texture = if material {
        &source.material
    } else {
        &source.color
    };
    let descriptor = ImageDescriptor {
        format: texture.format(),
        ..ImageDescriptor::rgba(texture.width(), texture.height())
    };
    let image = if let Some(lease) = &source._leases[index] {
        GpuImage::from_target(context, lease.clone(), descriptor).unwrap()
    } else {
        Arc::new(GpuImage::from_texture(context, texture.clone(), descriptor).unwrap())
    };
    let mut capture = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    capture
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}

#[test]
#[ignore = "native GPU; immutable atlas source leases, accounting and generations"]
fn atlas_source_leases_preserve_retained_pixels_and_exact_mip_ownership() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let atlas = SharedAtlas::new(16, 16);
    atlas.inner.lock().unwrap().auto_size = false;
    let id = Uuid::new_v4();
    atlas.add_tile(
        id,
        1,
        1,
        vec![vec![240, 20, 10, 255]],
        vec![vec![7, 15, 128, 128]],
    );
    atlas.upload_to_gpu_with_context(&context).unwrap();
    let first = atlas.retained_texture_views().unwrap();
    let rect = atlas.frame_rect(&id, 0).unwrap();
    let offset = (rect.x + rect.y * 16) as usize * 4;
    let expected_bytes = (16 * 16 + 8 * 8 + 4 * 4 + 2 * 2 + 1) * 4 * 2;
    assert_eq!(context.target_pool_usage().live_bytes, expected_bytes);
    assert_eq!(context.transfers().uploaded_bytes, 16 * 16 * 4 * 2);
    let warm = context.target_pool_usage();
    let transfers = context.transfers();
    atlas.upload_to_gpu_with_context(&context).unwrap();
    assert_eq!(context.target_pool_usage().allocations, warm.allocations);
    assert_eq!(context.transfers(), transfers);
    assert!(first.same_storage(&atlas.retained_texture_views().unwrap()));
    atlas
        .with_tile_mut(&id, |tile| {
            tile.frames[0].copy_from_slice(&[10, 220, 30, 255])
        })
        .unwrap();
    atlas.upload_to_gpu_with_context(&context).unwrap();
    let second = atlas.retained_texture_views().unwrap();
    assert!(!first.same_storage(&second));
    assert_eq!(context.target_pool_usage().live_bytes, expected_bytes * 2);
    context.set_target_cache_limits(0, 0);
    let mut descriptor = TargetDescriptor::color(16, 16, wgpu::TextureFormat::Rgba8Unorm);
    descriptor.mip_levels = 5;
    descriptor.usage |= wgpu::TextureUsages::STORAGE_BINDING;
    let unrelated = context.acquire_target(descriptor).unwrap();
    assert_ne!(unrelated.texture(), &first.color);
    assert_ne!(unrelated.texture(), &first.material);
    context.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: unrelated.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &vec![199; 16 * 16 * 4],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(16),
        },
        wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
    );
    assert_eq!(
        &pixels(&context, &first, false)[offset..offset + 4],
        &[240, 20, 10, 255]
    );
    assert_eq!(
        &pixels(&context, &second, false)[offset..offset + 4],
        &[10, 220, 30, 255]
    );
    assert_eq!(
        &pixels(&context, &second, true)[offset..offset + 4],
        &[7, 15, 128, 128]
    );
    // Failure of the second plane cannot replace the first published plane.
    {
        let mut inner = atlas.inner.lock().unwrap();
        inner.atlas_material.width = context.device.limits().max_texture_dimension_2d + 1;
        inner.atlas_dirty = true;
    }
    assert!(atlas.upload_to_gpu_with_context(&context).is_err());
    assert!(second.same_storage(&atlas.retained_texture_views().unwrap()));
    {
        let mut inner = atlas.inner.lock().unwrap();
        inner.atlas_material.width = 16;
        inner.atlas_dirty = false;
    }
    // Explicit raw-device reference updates must not overwrite leased sources.
    atlas
        .with_tile_mut(&id, |tile| {
            tile.frames[0].copy_from_slice(&[20, 30, 230, 255])
        })
        .unwrap();
    atlas.upload_to_gpu_in_domain(&context.device, &context.queue, context.generation());
    assert_eq!(
        &pixels(&context, &second, false)[offset..offset + 4],
        &[10, 220, 30, 255]
    );
    let raw = atlas.retained_texture_views().unwrap();
    assert_eq!(
        &pixels(&context, &raw, false)[offset..offset + 4],
        &[20, 30, 230, 255]
    );
    let replacement = pollster::block_on(GpuContext::headless()).unwrap();
    replacement.set_target_cache_limits(0, 0);
    atlas.upload_to_gpu_with_context(&replacement).unwrap();
    let rebuilt = atlas.retained_texture_views().unwrap();
    assert_eq!(
        &pixels(&replacement, &rebuilt, false)[offset..offset + 4],
        &[20, 30, 230, 255]
    );
    assert_eq!(
        &pixels(&context, &first, false)[offset..offset + 4],
        &[240, 20, 10, 255]
    );
    drop((atlas, first, second, raw, rebuilt, unrelated));
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert_eq!(replacement.target_pool_usage().live_bytes, 0);
}
