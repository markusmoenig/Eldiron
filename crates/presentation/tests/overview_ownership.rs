use eldiron_presentation::overview2d::Overview2DRenderer;
use renderkit::{GpuContext, GpuImage, ReadbackTicket, wgpu};
use rusterix::{Assets, Batch2D, PixelSource, RepeatMode, Scene, Texture, Tile, TileUvPages};
use std::sync::Arc;
use theframework_core::Uuid;
use vek::Mat3;

fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .unwrap();
    ticket
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}

#[test]
#[ignore = "native GPU; overview batch/source/output lease ownership"]
fn ordered_overview_batches_reuse_storage_and_pin_retired_outputs() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let assets = Assets::default();
    let namespace = Uuid::new_v4();
    let size = (17, 9);
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let green = [0, 255, 0, 255];
    let yellow = [255, 255, 0, 255];
    let mut paged = Tile::from_texture(Texture::new(red.to_vec(), 1, 1));
    // Unequal layer dimensions exercise sampled-texel initialization when an
    // array is reused: no coordinates may address another layer's padding.
    paged.textures.push(Texture::new(blue.repeat(2), 2, 1));
    paged.uv_pages = Some(TileUvPages {
        animation_frames: 1,
        repeat_period: [0.; 2],
        origin: [0, 0],
        size: [2, 1],
        axes: [[0.; 3]; 2],
        units_per_page: 1,
    });
    let mut scene = Scene::empty();
    scene.dynamic_textures = vec![
        paged,
        Tile::from_texture(Texture::new(green.to_vec(), 1, 1)),
        Tile::from_texture(Texture::new(yellow.to_vec(), 1, 1)),
    ];
    scene.animation_frame = 0;
    for (index, (x, width)) in [(0., 8.), (8., 4.), (12., 5.)].into_iter().enumerate() {
        let mut batch = Batch2D::from_rectangle(x, 0., width, 9.);
        batch.source = PixelSource::DynamicTileIndex(index as u16);
        if index == 0 {
            batch.repeat_mode = RepeatMode::RepeatXY;
            for uv in &mut batch.uvs {
                uv[0] *= 2.;
            }
        }
        scene.d2_static.push(batch);
    }
    let mut renderer = Overview2DRenderer::new(context.clone()).unwrap();
    let image = renderer
        .render(&scene, &assets, size, Mat3::identity(), namespace, 1, &[])
        .unwrap();
    assert_eq!(context.transfers().readback_bytes, 0);
    assert_eq!(context.capture_queue_usage().bytes, 0);
    let expected: Vec<u8> = (0..size.1)
        .flat_map(|_| {
            (0..size.0).flat_map(|x| match x {
                0..=3 => red,
                4..=7 => blue,
                8..=11 => green,
                _ => yellow,
            })
        })
        .collect();
    assert_eq!(capture(&context, image.clone()), expected);
    let allocations = (
        context.target_pool_usage().allocations,
        context.working_buffer_pool_usage().allocations,
    );
    let warm = renderer
        .render(&scene, &assets, size, Mat3::identity(), namespace, 1, &[])
        .unwrap();
    assert_eq!(warm.texture, image.texture);
    assert_eq!(
        (
            context.target_pool_usage().allocations,
            context.working_buffer_pool_usage().allocations,
        ),
        allocations,
        "warm batches reuse metadata without new allocation"
    );
    assert_eq!(capture(&context, warm.clone()), expected);
    let larger = renderer
        .render(
            &scene,
            &assets,
            (19, 11),
            Mat3::identity(),
            namespace,
            1,
            &[],
        )
        .unwrap();
    assert_ne!(larger.texture, image.texture);
    drop((renderer, warm, larger));
    let mut unrelated = Overview2DRenderer::new(context.clone()).unwrap();
    let other = unrelated
        .render_with_background(
            &Scene::empty(),
            &assets,
            size,
            Mat3::identity(),
            namespace,
            2,
            &[],
            [3, 4, 5],
        )
        .unwrap();
    assert_ne!(other.texture, image.texture);
    assert!(
        capture(&context, other.clone())
            .chunks_exact(4)
            .all(|p| p == [3, 4, 5, 255])
    );
    assert_eq!(capture(&context, image.clone()), expected);
    drop((unrelated, other));
    context.set_working_buffer_cache_limits(0, 0);
    context.set_target_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert_eq!(
        context.target_pool_usage().live_bytes,
        u64::from(size.0 * size.1) * 4
    );
    assert_eq!(context.target_pool_usage().live_allocations, 1);
    drop(image);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert!(!scenevm::SceneVM::compatibility_gpu_initialized());
}
