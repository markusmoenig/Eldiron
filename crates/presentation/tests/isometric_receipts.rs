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
