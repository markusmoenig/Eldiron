use eldiron_presentation::{
    isometric::{IsometricSceneInput, IsometricSceneRenderer, IsometricSourceVersion},
    screen::ContentRevision,
};
use renderkit::GpuContext;
use rusterix::{
    Assets, Item, Map, PixelSource, RenderSettings, Texture, Tile, Value,
    isometric::{Projection, SpriteLayout, lighting::Lighting},
};
use scenevm::prelude::{Vec2, Vec3};
use std::collections::HashMap;
use theframework_core::Uuid;
#[allow(dead_code)]
mod isometric_support;

#[test]
#[ignore = "requires native GPU; large source handoff and stale packet revocation"]
fn revoked_view_packets_keep_large_sources_until_replacement_without_reuploading() {
    use eldiron_presentation::{SceneViews, ViewId};
    use renderkit::ImageRegistry;
    use rusterix::TileUvPages;
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&images, scenevm::SharedAtlas::new(64, 64));
    let id = ViewId::new();
    views.create(id, (64, 64)).unwrap();
    let mut tile = Tile::from_textures(
        (0..9)
            .map(|_| Texture::new([45, 90, 120, 255].repeat(512 * 512), 512, 512))
            .collect(),
    );
    tile.uv_pages = Some(TileUvPages {
        origin: [0; 2],
        size: [9, 1],
        units_per_page: 1,
        animation_frames: 1,
        repeat_period: [0.; 2],
        axes: [[0.; 3]; 2],
    });
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    isometric_support::sector(&mut map, tile.id);
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let projection = Projection {
        origin: Vec2::new(32., 48.),
        unit: 16.,
    };
    views
        .prepare_isometric_overview(id, (64, 64), &map, &assets, projection, 1, &mut images)
        .unwrap();
    let mut current = views.render(id, 1, &mut images).unwrap().unwrap();
    assert!(
        gpu.transfers().uploaded_bytes > 32 * 1024 * 1024,
        "source must exceed the production cache budget"
    );
    assert!(views.isometric_targets(current).is_some());
    for _ in 0..8 {
        assert!(views.isometric_targets(current).is_some());
        views.scene_mut(id).unwrap();
        assert!(views.isometric_targets(current).is_none());
        assert!(
            views.render(id, 1, &mut images).is_err(),
            "revocation must not replay the stale packet"
        );
        let before = gpu.transfers().uploaded_bytes;
        views
            .prepare_isometric_overview(id, (64, 64), &map, &assets, projection, 1, &mut images)
            .unwrap();
        assert_eq!(
            gpu.transfers().uploaded_bytes,
            before,
            "outgoing source leases must bridge the invalid packet"
        );
        current = views.render(id, 1, &mut images).unwrap().unwrap();
    }
    let mut changed = assets.tiles.clone();
    changed.values_mut().next().unwrap().textures[0].data[0] = 180;
    assets.set_tiles(changed);
    views.scene_mut(id).unwrap();
    let before = gpu.transfers().uploaded_bytes;
    views
        .prepare_isometric_overview(id, (64, 64), &map, &assets, projection, 1, &mut images)
        .unwrap();
    assert!(
        gpu.transfers().uploaded_bytes - before > 32 * 1024 * 1024,
        "new source revision must not alias the old lease"
    );
    views.render(id, 1, &mut images).unwrap().unwrap();
    assert!(views.remove(id, &mut images).unwrap());
    assert_eq!(gpu.transfers().readback_bytes, 0);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn immutable_isometric_frames_keep_item_source_alpha_and_reject_foreign_device_or_bad_extent() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let other = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut producer = IsometricSceneRenderer::new(gpu.clone()).unwrap();
    let mut foreign = IsometricSceneRenderer::new(other.clone()).unwrap();
    let mut pixels = vec![0; 3 * 3 * 4];
    pixels[4..8].copy_from_slice(&[80, 180, 70, 255]);
    let mut tile = Tile::from_texture(Texture::new(pixels, 3, 3));
    tile.isometric = Some(SpriteLayout {
        anchor: [1, 3],
        pixels_per_unit: 8.,
        ..Default::default()
    });
    let id = tile.id;
    let mut assets = Assets::default();
    assets.set_tiles([(id, tile)].into_iter().collect());
    let mut map = Map::default();
    map.camera = rusterix::MapCamera::TwoDIso;
    let mut item = Item::new();
    item.id = 7;
    item.position = Vec3::new(0., 1., 0.);
    item.set_attribute("source", Value::Source(PixelSource::TileId(id)));
    map.items.push(item);
    let avatars = HashMap::new();
    let settings = RenderSettings::default();
    let lighting = Lighting::new(&settings, std::iter::empty(), 0);
    let projection = Projection {
        origin: Vec2::new(32., 48.),
        unit: 8.,
    };
    let version = IsometricSourceVersion {
        namespace: Uuid::new_v4(),
        content: ContentRevision(1),
        tiles: assets.tiles_revision,
    };
    let input = IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &avatars,
        seconds: 0.,
        projection,
        lighting: &lighting,
        particles: &[],
        source_version: version,
    };
    let uploaded = gpu.transfers().uploaded_bytes;
    for size in [(0, 64), (64, 0), (u32::MAX, 64)] {
        assert!(producer.prepare(size, &input).is_err());
    }
    assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
    let first = producer.prepare((64, 64), &input).unwrap();
    let before = other.transfers();
    assert!(foreign.render_prepared(&first, 1).is_err());
    assert_eq!(other.transfers(), before);
    producer.render_prepared(&first, 1).unwrap();
    let targets = first.targets();
    let hits = (0..64)
        .flat_map(|y| (0..64).map(move |x| [(x as f32 + 0.5) / 64., (y as f32 + 0.5) / 64.]))
        .filter(|uv| {
            targets
                .hit(*uv, false)
                .is_some_and(|h| h.item == Some(7) && h.cell.is_none())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        hits.len(),
        1,
        "only the opaque source texel above the ground cell is selected"
    );
    let point = hits[0];
    let original = targets.hit(point, false).unwrap();
    assert_eq!(original.world, Some(Vec3::new(0., 1., 0.)));
    let builds = producer.source_builds();
    let second = producer.prepare((64, 64), &input).unwrap();
    assert_eq!(producer.source_builds(), builds);
    drop(input);
    map.items[0].position.x = 2.;
    let mut tiles = assets.tiles.clone();
    tiles.get_mut(&id).unwrap().textures[0].data[7] = 0;
    assets.set_tiles(tiles);
    let input = IsometricSceneInput {
        map: &map,
        assets: &assets,
        avatars: &avatars,
        seconds: 0.,
        projection,
        lighting: &lighting,
        particles: &[],
        source_version: IsometricSourceVersion {
            content: ContentRevision(2),
            tiles: assets.tiles_revision,
            ..version
        },
    };
    let changed = producer.prepare((64, 64), &input).unwrap();
    producer.render_prepared(&changed, 2).unwrap();
    assert_ne!(
        changed.targets().hit(point, false).and_then(|h| h.item),
        Some(7)
    );
    assert_eq!(targets.hit(point, false), Some(original));
    assert_eq!(second.targets().hit(point, false), Some(original));
    producer.render_prepared(&first, 3).unwrap();
    assert!(!producer.has_cpu_storage() && !producer.has_readback_storage());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu.transfers().capture_requests, 0);
}
