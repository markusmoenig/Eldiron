use eldiron_presentation::{
    isometric::{IsometricSourceVersion, IsometricSources},
    screen::ContentRevision,
};
use renderkit::GpuContext;
use rusterix::{
    Texture, Tile,
    isometric::{
        AvatarSprite, SpriteLayout,
        paint::{StampSprite, TerrainStamp},
        scene::Sprite,
    },
};
use std::sync::Arc;
use theframework::prelude::{Uuid, Vec2};

fn sprite<'a>(
    tile: Option<&'a Tile>,
    avatar: Option<&'a AvatarSprite>,
    stamp: Option<&'a TerrainStamp>,
) -> Sprite<'a> {
    Sprite {
        owner: None,
        tile,
        avatar,
        stamp,
        scale: 1.,
        pos: Vec2::zero(),
        height: 0.,
        frame: 0,
        opacity: 1.,
        tie: 0,
        support: None,
        radius: 0.,
        floor: None,
    }
}
fn version() -> IsometricSourceVersion {
    IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: 1,
    }
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn isometric_tile_residency_is_versioned_and_old_leases_survive_strict_eviction() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut cache = IsometricSources::new(gpu.clone(), 1024, 8);
    let mut tile = Tile::from_texture(Texture::new([90, 120, 170, 255].repeat(4), 2, 2));
    tile.isometric = Some(SpriteLayout {
        anchor: [1, 2],
        pixels_per_unit: 2.,
        ..Default::default()
    });
    let mut v = version();
    let first = cache
        .prepare(&sprite(Some(&tile), None, None), v)
        .unwrap()
        .unwrap();
    let uploaded = gpu.transfers().uploaded_bytes;
    let mut moved = sprite(Some(&tile), None, None);
    moved.pos = Vec2::new(9., 7.);
    moved.scale = 2.;
    moved.height = 3.;
    assert!(Arc::ptr_eq(
        &first,
        &cache.prepare(&moved, v).unwrap().unwrap()
    ));
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    assert_eq!(cache.builds(), 1);
    assert_eq!(cache.len(), 1);
    drop(moved);
    tile.textures[0].data[0] = 220;
    let moved = sprite(Some(&tile), None, None);
    v.content.0 += 1;
    let edit = cache.prepare(&moved, v).unwrap().unwrap();
    assert!(!Arc::ptr_eq(&first, &edit));
    v.tiles += 1;
    let baked = cache.prepare(&moved, v).unwrap().unwrap();
    assert!(!Arc::ptr_eq(&edit, &baked));
    v.namespace = Uuid::new_v4();
    let replacement = cache.prepare(&moved, v).unwrap().unwrap();
    assert!(!Arc::ptr_eq(&baked, &replacement));
    assert_eq!(cache.builds(), 4);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.resident_bytes(), 0);
    assert_eq!(first.size(), (2, 2)); // external GPU leases still own their buffers
    let mut zero = IsometricSources::new(gpu.clone(), 0, 0);
    let oversized = zero.prepare(&moved, v).unwrap().unwrap();
    assert!(zero.is_empty());
    assert_eq!(zero.resident_bytes(), 0);
    assert_eq!(oversized.size(), (2, 2));
    let mut one = IsometricSources::new(gpu.clone(), 1024, 1);
    let old = one.prepare(&moved, v).unwrap().unwrap();
    v.content.0 += 1;
    let current = one.prepare(&moved, v).unwrap().unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one.resident_bytes(), current.resident_bytes());
    assert!(!Arc::ptr_eq(&old, &current));
    assert_eq!(gpu.transfers().readback_bytes, 0);
    // GPU leases retain the exact source revision after eviction and editing.
    use scenevm::isometric::{
        IsometricBlitter, IsometricCanvas, IsometricRenderer, SpriteBlit, Uniforms,
    };
    let canvas = IsometricCanvas::new(&gpu, 8, 4).unwrap();
    let mut blitter = IsometricBlitter::new(gpu.clone()).unwrap();
    let place = |source, origin| SpriteBlit {
        source,
        origin,
        pixel_scale: 2.,
        base_height: 0.,
        world_scale: 1.,
        opacity: 1.,
        bitmap: true,
        prelit: true,
        ground: true,
        anchor_y: 2.,
        pixels_per_unit: 2.,
    };
    blitter
        .render(
            &canvas,
            [0; 4],
            &[place(&first, [0., 0.]), place(&edit, [4., 0.])],
        )
        .unwrap();
    let mut renderer = IsometricRenderer::new(gpu.clone()).unwrap();
    let image = renderer
        .render(
            &canvas,
            &Uniforms {
                counts: [8, 4, 0, 0],
                projection: [4., 2., 4., 8.],
                ..Default::default()
            },
            &[],
            &[],
            &[],
            &[],
            None,
            1,
        )
        .unwrap()
        .image;
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let mut capture = renderkit::ReadbackTicket::capture(&gpu, image).unwrap();
    gpu.device
        .poll(renderkit::wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let image = capture.try_finish(&gpu).unwrap().unwrap();
    assert_eq!(&image.pixels()[..4], &[90, 120, 170, 255]);
    assert_eq!(&image.pixels()[16..20], &[220, 120, 170, 255]);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn immutable_avatar_and_stamp_leases_prevent_pointer_reuse_and_share_across_instances() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut cache = IsometricSources::new(gpu.clone(), 1024 * 1024, 8);
    let v = version();
    let mut avatar = AvatarSprite {
        pixels: Arc::new([70, 120, 190, 255].repeat(128 * 128)),
        surface: None,
    };
    let initial = cache
        .prepare(&sprite(None, Some(&avatar), None), v)
        .unwrap()
        .unwrap();
    assert_eq!(
        Arc::strong_count(&avatar.pixels),
        2,
        "cache leases the original immutable source"
    );
    let uploaded = gpu.transfers().uploaded_bytes;
    let other = avatar.clone();
    let mut moved = sprite(None, Some(&other), None);
    moved.pos = Vec2::new(4., 8.);
    assert!(Arc::ptr_eq(
        &initial,
        &cache.prepare(&moved, v).unwrap().unwrap()
    ));
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    Arc::make_mut(&mut avatar.pixels)[0] = 210;
    let edited = cache
        .prepare(&sprite(None, Some(&avatar), None), v)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&initial, &edited));
    let mut stamp = TerrainStamp {
        id: Uuid::new_v4(),
        order: 3,
        position: [0.; 2],
        raster_key: 1,
        sprite: Arc::new(StampSprite {
            texture: Texture::new([60, 160, 30, 255].repeat(4), 2, 2),
            surface: None,
            anchor: [1., 2.],
        }),
    };
    let original = cache
        .prepare(&sprite(None, None, Some(&stamp)), v)
        .unwrap()
        .unwrap();
    assert_eq!(Arc::strong_count(&stamp.sprite), 2);
    stamp.position = [5., 6.];
    stamp.order = 4;
    assert!(Arc::ptr_eq(
        &original,
        &cache
            .prepare(&sprite(None, None, Some(&stamp)), v)
            .unwrap()
            .unwrap()
    ));
    stamp.raster_key += 1;
    assert!(!Arc::ptr_eq(
        &original,
        &cache
            .prepare(&sprite(None, None, Some(&stamp)), v)
            .unwrap()
            .unwrap()
    ));
    assert_eq!(gpu.transfers().readback_bytes, 0);
    cache.clear();
    assert_eq!(Arc::strong_count(&stamp.sprite), 1);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn source_alpha_receipts_reuse_assets_and_survive_revision_edits_and_eviction() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut cache = IsometricSources::new(gpu.clone(), 1024, 1);
    let mut tile = Tile::from_texture(Texture::new(vec![10, 20, 30, 0, 40, 50, 60, 255], 2, 1));
    let mut v = version();
    let s = sprite(Some(&tile), None, None);
    let (first, alpha) = cache.prepare_with_alpha(&s, v).unwrap().unwrap();
    let uploaded = gpu.transfers().uploaded_bytes;
    let (again, reused) = cache.prepare_with_alpha(&s, v).unwrap().unwrap();
    assert!(Arc::ptr_eq(&first, &again) && Arc::ptr_eq(&alpha, &reused));
    assert_eq!(&*alpha, &[0, 255]);
    assert_eq!(uploaded, gpu.transfers().uploaded_bytes);
    drop(s);
    tile.textures[0].data[3] = 190;
    v.content.0 += 1;
    let (_, changed) = cache
        .prepare_with_alpha(&sprite(Some(&tile), None, None), v)
        .unwrap()
        .unwrap();
    assert_eq!(&*changed, &[190, 255]);
    assert_eq!(&*alpha, &[0, 255]);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(&*alpha, &[0, 255]);
    let mut zero = IsometricSources::new(gpu.clone(), 0, 0);
    assert!(
        zero.prepare_with_alpha(&sprite(Some(&tile), None, None), v)
            .unwrap()
            .is_some()
    );
    assert!(zero.is_empty());
}
