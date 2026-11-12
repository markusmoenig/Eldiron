//! The native producer's borrowed CPU scene plan works without a GPU or canvas.
use rusterix::{
    Assets, Entity, Item, Map, PixelSource, Texture, Tile, Value, Vertex,
    isometric::{
        AvatarSprite, Projection, SceneCache, SpriteLayout, scene::SpriteOwner,
        surface::SurfaceFrame,
    },
};
use std::{collections::HashMap, sync::Arc};
use theframework::prelude::{Uuid, Vec2, Vec3};

fn tile() -> Tile {
    let mut tile = Tile::from_texture(Texture::new([180, 40, 20, 255].repeat(16), 4, 4));
    tile.isometric = Some(SpriteLayout {
        anchor: [2, 4],
        pixels_per_unit: 4.,
        directions: 1,
        frames_per_direction: 2,
        frames_per_second: 4.,
        frames: vec![
            Texture::new([180, 40, 20, 255].repeat(16), 4, 4),
            Texture::new([20, 180, 40, 255].repeat(16), 4, 4),
        ],
        ..Default::default()
    });
    tile
}
fn sector(map: &mut Map, tile: Uuid) -> u32 {
    let ids = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].map(|p| {
        let id = map.vertices.len() as u32;
        map.vertices.push(Vertex::new(id, p[0], p[1]));
        id
    });
    for n in 0..4 {
        map.create_linedef_manual(ids[n], ids[(n + 1) % 4]);
    }
    let id = map.close_polygon_manual().unwrap();
    map.find_sector_mut(id)
        .unwrap()
        .properties
        .set("source", Value::Source(PixelSource::TileId(tile)));
    id
}
fn mesh_surface() -> SurfaceFrame {
    let mut s = SurfaceFrame::new(128 * 128);
    let mut mesh = scenevm::isometric::ShadowGeometry::default();
    mesh.triangle([[0., 0., 0.], [0., 2., 0.], [1., 0., 0.]]);
    s.shadow_mesh = Some(mesh);
    s
}

#[test]
fn scene_borrows_selected_frames_and_preserves_distinct_placement_identities() {
    let mut map = Map::default();
    let tile = tile();
    let tile_id = tile.id;
    for x in [0., 3.] {
        let mut item = Item::default();
        item.id = 0;
        item.creator_id = Uuid::new_v4();
        item.position = Vec3::new(x, 1., 2.);
        item.set_attribute("source", Value::Source(PixelSource::TileId(tile_id)));
        map.items.push(item);
    }
    let mut entity = Entity::default();
    entity.id = 0;
    entity.position = Vec3::new(0., 2., 3.);
    entity.set_attribute("source", Value::Source(PixelSource::TileId(tile_id)));
    map.entities.push(entity);
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let before = serde_json::to_value(&map).unwrap();
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let plan = cache.prepare(&map, &assets, 0.25, &avatars);
    assert_eq!(plan.upright.len(), 3);
    assert!(
        plan.upright
            .windows(2)
            .all(|s| !s[0].painter_cmp(&s[1]).is_gt())
    );
    for (index, item) in map.items.iter().enumerate() {
        let draw = plan
            .upright
            .iter()
            .find(|s| {
                s.owner
                    == Some(SpriteOwner::Item {
                        index,
                        id: 0,
                        creator_id: item.creator_id,
                    })
            })
            .unwrap();
        assert_eq!(draw.frame, 1);
        let image = draw.image().unwrap();
        let source = &assets.tile_list[0].isometric.as_ref().unwrap().frames[1];
        assert_eq!(image.data.as_ptr(), source.data.as_ptr());
        let p = Projection {
            origin: Vec2::new(70., 80.),
            unit: 16.,
        };
        let (at, scale) = draw.image_placement(&image, p);
        assert_eq!(scale, 4.);
        assert_eq!(
            at,
            p.screen(item.get_pos_xz(), item.position.y) - Vec2::new(8., 16.)
        );
    }
    assert!(plan.upright.iter().any(|s| s.owner
        == Some(SpriteOwner::Entity {
            index: 0,
            id: 0,
            creator_id: map.entities[0].creator_id
        })));
    assert_eq!(serde_json::to_value(&map).unwrap(), before);
}

#[test]
fn hidden_cutaway_geometry_keeps_shadows_without_a_visible_sprite() {
    let mut map = Map::default();
    let mut tile = tile();
    tile.isometric.as_mut().unwrap().surfaces = vec![mesh_surface(), mesh_surface()];
    let sector_id = sector(&mut map, tile.id);
    let room = map.find_sector_mut(sector_id).unwrap();
    room.properties.set("iso_interior", Value::Bool(true));
    room.properties.set("iso_cutaway", Value::Bool(true));
    let mut player = Entity::default();
    player.position = Vec3::new(0.5, 0., 0.5);
    player.set_attribute("player", Value::Bool(true));
    map.entities.push(player);
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let plan = cache.prepare(&map, &assets, 0., &avatars);
    assert!(
        !plan
            .upright
            .iter()
            .any(|s| s.owner == Some(SpriteOwner::Sector(sector_id)))
    );
    let first = cache.shadow_geometry(&map, &assets, &plan).unwrap();
    let same = cache.shadow_geometry(&map, &assets, &plan).unwrap();
    assert_eq!(first.vertices.len(), 3);
    assert!(Arc::ptr_eq(&first, &same));
    map.entities[0].position.x = 3.;
    let plan = cache.prepare(&map, &assets, 0., &avatars);
    assert!(
        plan.upright
            .iter()
            .any(|s| s.owner == Some(SpriteOwner::Sector(sector_id)))
    );
}

#[test]
fn dynamic_avatar_geometry_updates_without_rebuilding_or_compositing_its_source() {
    let mut map = Map::default();
    let mut entity = Entity::default();
    entity.id = 94;
    map.entities.push(entity);
    let pixels = Arc::new([90, 120, 170, 255].repeat(128 * 128));
    let surface = Arc::new(mesh_surface());
    let avatars = [(
        0,
        AvatarSprite {
            pixels: pixels.clone(),
            surface: Some(surface.clone()),
        },
    )]
    .into_iter()
    .collect();
    let assets = Assets::default();
    let mut cache = SceneCache::default();
    let plan = cache.prepare(&map, &assets, 0., &avatars);
    let image = plan.upright[0].image().unwrap();
    assert_eq!(image.data.as_ptr(), pixels.as_ptr());
    assert!(std::ptr::eq(image.surface.unwrap(), surface.as_ref()));
    let first = cache.shadow_geometry(&map, &assets, &plan).unwrap();
    let plan = cache.prepare(&map, &assets, 1., &avatars);
    assert!(Arc::ptr_eq(
        &first,
        &cache.shadow_geometry(&map, &assets, &plan).unwrap()
    ));
    let unchanged_revision = map.changed;
    map.entities[0].position = Vec3::new(4., 1., 2.);
    map.entities[0].set_attribute("size_2d", Value::Float(2.));
    let plan = cache.prepare(&map, &assets, 1., &avatars);
    let moved = cache.shadow_geometry(&map, &assets, &plan).unwrap();
    assert_ne!(first.revision, moved.revision);
    assert_ne!(first.vertices, moved.vertices);
    assert_eq!(map.changed, unchanged_revision);
    assert_eq!(
        plan.upright[0].image().unwrap().data.as_ptr(),
        pixels.as_ptr()
    );
}

#[test]
fn terrain_ownership_is_camera_independent_and_invalidated_by_source_revision() {
    let mut map = Map::default();
    sector(&mut map, Uuid::new_v4());
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(2));
    let mut cache = SceneCache::default();
    let assets = Assets::default();
    let avatars = HashMap::new();
    let first = cache.prepare(&map, &assets, 0., &avatars).terrain.unwrap();
    let same = cache.prepare(&map, &assets, 1., &avatars).terrain.unwrap();
    assert!(Arc::ptr_eq(&first, &same));
    map.changed = map.changed.wrapping_add(1);
    let edited = cache.prepare(&map, &assets, 1., &avatars).terrain.unwrap();
    assert!(!Arc::ptr_eq(&first, &edited));
    map.id = Uuid::new_v4();
    let other = cache.prepare(&map, &assets, 1., &avatars).terrain.unwrap();
    assert!(!Arc::ptr_eq(&edited, &other));
    map.properties.set("terrain_enabled", Value::Bool(false));
    map.changed = map.changed.wrapping_add(1);
    assert!(cache.prepare(&map, &assets, 2., &avatars).terrain.is_none());
    assert!(
        Arc::strong_count(&first) >= 2,
        "old plans retain their geometry lease"
    );
}

#[test]
fn stamp_presentation_identity_is_independent_of_its_painter_order() {
    use rusterix::isometric::paint::{StampSprite, TerrainPaint, TerrainStamp};
    let mut map = Map::default();
    sector(&mut map, Uuid::new_v4());
    let sprite = Arc::new(StampSprite {
        texture: Texture::new([80, 120, 40, 255].repeat(4), 2, 2),
        surface: None,
        anchor: [1., 2.],
    });
    let stamps: [TerrainStamp; 2] = std::array::from_fn(|_| TerrainStamp {
        id: Uuid::new_v4(),
        order: 7,
        position: [0.5, 0.5],
        raster_key: 1,
        sprite: sprite.clone(),
    });
    let ids = stamps.each_ref().map(|s| s.id);
    let mut assets = Assets::default();
    assets.terrain_paint.insert(
        map.id,
        Arc::new(TerrainPaint {
            stamps: stamps.into(),
            ..Default::default()
        }),
    );
    let mut cache = SceneCache::default();
    let avatars = HashMap::new();
    let plan = cache.prepare(&map, &assets, 0., &avatars);
    assert_eq!(plan.upright.len(), 2);
    assert_eq!(plan.upright[0].tie, plan.upright[1].tie);
    assert_eq!(
        plan.upright[0].owner,
        Some(SpriteOwner::TerrainStamp(ids[0]))
    );
    assert_eq!(
        plan.upright[1].owner,
        Some(SpriteOwner::TerrainStamp(ids[1]))
    );
    assert_ne!(plan.upright[0].owner, plan.upright[1].owner);
}
