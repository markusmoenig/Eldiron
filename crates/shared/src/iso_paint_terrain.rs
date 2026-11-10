//! Bridge the persistent paint layer to the CPU isometric ground renderer.
use crate::iso_paint::{IsoPaintLayer, IsoPaintOwner};
use rusterix::{
    Assets,
    isometric::paint::{TERRAIN_PAINT_GEO, TerrainPaint, TerrainPaintChunk},
};
use std::hash::{Hash, Hasher};
use theframework::prelude::Uuid;

pub fn sync(assets: &mut Assets, map_id: Uuid, layer: &mut IsoPaintLayer) {
    layer.refresh_sources(assets);
    layer.ensure_baked_paint_current();
    if !layer.visible {
        assets.terrain_paint.remove(&map_id);
        return;
    }
    let chunks: Vec<_> = layer
        .baked_chunks
        .values()
        .filter(|c| {
            matches!(c.owner, IsoPaintOwner::Terrain { .. }) && c.paint_geo == TERRAIN_PAINT_GEO
        })
        .collect();
    let stamps: Vec<_> = layer
        .chunks
        .values()
        .flat_map(|c| &c.stamps)
        .filter(|s| {
            matches!(s.owner, Some(IsoPaintOwner::Terrain { .. }))
                && s.paint_geo == Some(TERRAIN_PAINT_GEO)
                && s.surface_uv.is_some()
        })
        .collect();
    if chunks.is_empty() && stamps.is_empty() {
        assets.terrain_paint.remove(&map_id);
        return;
    }
    // Chunk revisions track dabs; unique stroke IDs distinguish edits made after
    // undo, even when the new branch has the same number of points/pixels.
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for chunk in &chunks {
        chunk.origin.hash(&mut hash);
        chunk.revision.hash(&mut hash);
    }
    for chunk in layer.chunks.values() {
        chunk.revision.hash(&mut hash);
        for stroke in &chunk.strokes {
            stroke.id.hash(&mut hash);
        }
    }
    if !layer.source_images.is_empty() { layer.source_revision.hash(&mut hash); }
    let ground_key = hash.finish();
    for stamp in &stamps {
        stamp.id.hash(&mut hash);
        stamp.order.hash(&mut hash);
        stamp
            .surface_uv
            .map(|uv| uv.map(f32::to_bits))
            .hash(&mut hash);
        stamp_raster_key(stamp).hash(&mut hash);
    }
    let key = hash.finish();
    if assets
        .terrain_paint
        .get(&map_id)
        .is_some_and(|p| p.key == key)
    {
        return;
    }
    #[cfg(feature = "graphics")]
    let previous: std::collections::HashMap<_, _> = assets
        .terrain_paint
        .get(&map_id)
        .into_iter()
        .flat_map(|p| &p.stamps)
        .map(|s| (s.id, s))
        .collect();
    let paint = TerrainPaint {
        key,
        ground_key,
        stamps: stamps
            .into_iter()
            .filter_map(|stamp| {
                #[cfg(feature = "graphics")]
                {
                    let raster_key = stamp_raster_key(stamp);
                    let sprite = previous
                        .get(&stamp.id)
                        .filter(|s| s.raster_key == raster_key)
                        .map(|s| s.sprite.clone())
                        .unwrap_or_else(|| {
                            std::sync::Arc::new(
                                crate::iso_paint_render::IsoPaintRenderer::terrain_stamp_sprite(
                                    stamp,
                                ),
                            )
                        });
                    Some(rusterix::isometric::paint::TerrainStamp {
                        id: stamp.id,
                        order: stamp.order,
                        position: stamp.surface_uv?,
                        raster_key,
                        sprite,
                    })
                }
                #[cfg(not(feature = "graphics"))]
                {
                    let _ = stamp;
                    None
                }
            })
            .collect(),
        chunks: chunks
            .into_iter()
            .map(|c| {
                (
                    c.origin,
                    TerrainPaintChunk {
                        color: c.color_rgba.clone(),
                        material: c.material_rgba.clone(),
                    },
                )
            })
            .collect(),
    };
    assets
        .terrain_paint
        .insert(map_id, std::sync::Arc::new(paint));
}

fn stamp_raster_key(stamp: &crate::iso_paint::IsoPaintStamp) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    stamp.source.hash(&mut hash);
    stamp.source_image.as_ref().map(|t| std::sync::Arc::as_ptr(t) as usize).hash(&mut hash);
    stamp.kind.hash(&mut hash);
    stamp.variant.hash(&mut hash);
    stamp.size.to_bits().hash(&mut hash);
    stamp.rotation.to_bits().hash(&mut hash);
    stamp.variation.hash(&mut hash);
    stamp.color.hash(&mut hash);
    stamp.palette_colors.hash(&mut hash);
    stamp.opacity.to_bits().hash(&mut hash);
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso_paint::IsoPaintPoint;
    use rusterix::chunkbuilder::{
        terrain_sculpt::{SculptMode, TerrainSculpt},
        terrain_surface::TerrainSurface,
    };
    use rusterix::{
        Map, PixelSource, Texture, Tile, Value, Vertex,
        isometric::{self, GroundCache, Projection},
    };
    use theframework::prelude::*;

    #[test]
    #[cfg(feature = "graphics")]
    fn organic_terrain_stamps_export_without_paint_and_reuse_artwork() {
        let mut layer = IsoPaintLayer::default();
        layer.active_material_mode = "stamp".into();
        layer.active_size = 1.5;
        layer.active_color = [100, 160, 80, 255];
        layer.active_palette_colors = vec![
            [100, 160, 80, 255],
            [230, 100, 160, 255],
            [220, 210, 100, 255],
        ];
        for (i, kind) in ["grass", "flowers", "rubble"].iter().enumerate() {
            layer.active_brush = kind.to_string();
            layer.add_stamp(
                IsoPaintPoint::new(
                    [i as i32 * 100, 100],
                    Some(Vec3::new(i as f32 - 1., 0., 0.)),
                    Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
                )
                .with_surface_uv(Some(Vec2::new(i as f32 - 1., 0.)))
                .with_paint_geo(Some(TERRAIN_PAINT_GEO)),
            );
        }
        let mut assets = Assets::default();
        let map_id = Uuid::new_v4();
        sync(&mut assets, map_id, &mut layer);
        assert_eq!(assets.terrain_paint[&map_id].stamps.len(), 3);
        assert!(assets.terrain_paint[&map_id].chunks.is_empty());
        let original = assets.terrain_paint[&map_id].clone();
        for stamp in &original.stamps {
            assert!(stamp.sprite.texture.data.chunks_exact(4).any(|p| p[3] > 0));
        }
        let mut reopened: IsoPaintLayer =
            serde_json::from_slice(&serde_json::to_vec(&layer).unwrap()).unwrap();
        sync(&mut assets, map_id, &mut reopened);
        assert!(std::sync::Arc::ptr_eq(
            &original,
            &assets.terrain_paint[&map_id]
        ));
        let ground_key = original.ground_key;
        // Erasure ignores the original screen location and preserves other kinds.
        assert!(reopened.erase_terrain_stamps_near([0., 0.], 0.25, "flowers"));
        sync(&mut assets, map_id, &mut reopened);
        let remaining = &assets.terrain_paint[&map_id];
        assert_eq!(remaining.stamps.len(), 2);
        assert_eq!(remaining.ground_key, ground_key);
        for stamp in &remaining.stamps {
            let before = original.stamps.iter().find(|s| s.id == stamp.id).unwrap();
            assert!(std::sync::Arc::ptr_eq(&before.sprite, &stamp.sprite));
            assert_eq!(before.position, stamp.position);
        }
        reopened.visible = false;
        sync(&mut assets, map_id, &mut reopened);
        assert!(!assets.terrain_paint.contains_key(&map_id));
    }

    #[test]
    fn terrain_paint_follows_sculpt_save_reopen_and_cache_changes() {
        let mut map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        map.vertices = vec![Vertex::new(0, -4., -4.), Vertex::new(1, 4., 4.)];
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        let mut ground = Tile::empty();
        ground.textures = vec![Texture::new(vec![30, 70, 40, 255], 1, 1)];
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(ground.id)),
        );
        let mut assets = Assets::default();
        assets.set_tiles([(ground.id, ground)].into_iter().collect());
        let center = Vec2::new(-0.5, 0.5); // Cross the negative paint-chunk seam too.
        let point = |uv: Vec2<f32>| {
            IsoPaintPoint::new(
                [100, 100],
                Some(Vec3::new(uv.x, 0., uv.y)),
                Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
            )
            .with_surface_uv(Some(uv))
            .with_paint_geo(Some(TERRAIN_PAINT_GEO))
        };
        let mut layer = IsoPaintLayer::default();
        layer.active_size = 8.;
        layer.active_color = [220, 40, 70, 255];
        layer.active_palette_colors = vec![layer.active_color];
        layer.begin_stroke(point(center));
        sync(&mut assets, map.id, &mut layer);
        let key = assets.terrain_paint[&map.id].key;
        let source = layer.baked_chunks.clone();
        let mut cache = GroundCache::default();
        let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 260));
        let projection = Projection {
            origin: Vec2::new(160., 170.),
            unit: 40.,
        };
        let mut rendered = Vec::new();
        for mode in [
            None,
            Some(SculptMode::Raise),
            Some(SculptMode::Smooth),
            Some(SculptMode::Lower),
        ] {
            if let Some(mode) = mode {
                TerrainSculpt::dab(&mut map, center, 2., 0.8, mode);
            }
            let surface = TerrainSurface::from_map(&map).unwrap();
            let h = surface.height_at(center).unwrap();
            let pixel = projection.screen(center, h);
            let picked = isometric::terrain_point(&map, projection, pixel).unwrap();
            assert!((Vec2::new(picked.x, picked.z) - center).magnitude() < 0.001);
            isometric::draw_scene_cached(
                &map,
                &assets,
                &mut buffer,
                projection,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            let at = ((pixel.y.round() as usize) * 320 + pixel.x.round() as usize) * 4;
            assert!(
                buffer.pixels()[at] > 150 && buffer.pixels()[at + 1] < 70,
                "paint must follow the visible surface"
            );
            rendered.push(pixel.y);
            assert_eq!(
                source, layer.baked_chunks,
                "sculpting must not rewrite paint"
            );
        }
        assert!(rendered[1] < rendered[0] - 30.);
        // Paint on flat geometry must survive the first terrain brush dab too.
        let mut flat = map.clone();
        flat.properties.set("terrain_enabled", Value::Bool(false));
        flat.vertices.clear();
        flat.sectors.clear();
        flat.linedefs.clear();
        let vertices =
            [(-3., -3.), (3., -3.), (3., 3.), (-3., 3.)].map(|(x, z)| flat.add_vertex_at(x, z));
        for i in 0..4 {
            flat.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
        }
        let sector = flat.close_polygon_manual().unwrap();
        let source = flat
            .properties
            .get_source("default_terrain_tile")
            .unwrap()
            .clone();
        let floor = flat.find_sector_mut(sector).unwrap();
        floor.layer = Some(0);
        floor.properties.set("source", Value::Source(source));
        flat.changed = flat.changed.wrapping_add(1);
        let pixel = projection.screen(center, 0.);
        assert!(isometric::terrain_point(&flat, projection, pixel).is_some());
        isometric::draw_scene_cached(
            &flat,
            &assets,
            &mut buffer,
            projection,
            0.,
            None,
            &Default::default(),
            Some(&mut cache),
        );
        let at = (pixel.y.round() as usize * 320 + pixel.x.round() as usize) * 4;
        assert!(buffer.pixels()[at] > 150);
        TerrainSculpt::dab(&mut flat, center, 2., 0.8, SculptMode::Raise);
        // An upright sprite stands above the painted ground rather than being
        // covered by a post-render screen overlay.
        let mut sprite = Tile::empty();
        sprite.textures = vec![Texture::new([20, 40, 230, 255].repeat(8 * 8), 8, 8)];
        sprite.isometric = Some(isometric::SpriteLayout {
            anchor: [4, 8],
            pixels_per_unit: 16.,
            ..Default::default()
        });
        let id = sprite.id;
        let mut tiles = assets.tiles.clone();
        tiles.insert(id, sprite);
        assets.set_tiles(tiles);
        let h = TerrainSurface::from_map(&flat)
            .unwrap()
            .height_at(center)
            .unwrap();
        let mut actor = rusterix::Entity::new();
        actor.position = Vec3::new(center.x, h, center.y);
        actor
            .attributes
            .set("source", Value::Source(PixelSource::TileId(id)));
        flat.entities.push(actor);
        isometric::draw_scene_cached(
            &flat,
            &assets,
            &mut buffer,
            projection,
            0.,
            None,
            &Default::default(),
            Some(&mut cache),
        );
        let pixel = projection.screen(center, h);
        let at = ((pixel.y.round() as usize - 5) * 320 + pixel.x.round() as usize) * 4;
        assert_eq!(
            &buffer.pixels()[at..at + 3],
            &[20, 40, 230],
            "paint must remain behind the character"
        );
        // Visibility and a different edit after undo must invalidate cached ground.
        let before = layer.clone();
        layer.visible = false;
        sync(&mut assets, map.id, &mut layer);
        assert!(!assets.terrain_paint.contains_key(&map.id));
        layer = before.clone();
        layer.active_color = [40, 40, 230, 255];
        layer.active_palette_colors = vec![layer.active_color];
        layer.begin_stroke(point(center));
        sync(&mut assets, map.id, &mut layer);
        assert_ne!(assets.terrain_paint[&map.id].key, key);
        layer = before;
        sync(&mut assets, map.id, &mut layer);
        assert_eq!(assets.terrain_paint[&map.id].key, key);
        let bytes = serde_json::to_vec(&layer).unwrap();
        let mut reopened: IsoPaintLayer = serde_json::from_slice(&bytes).unwrap();
        sync(&mut assets, map.id, &mut reopened);
        assert_eq!(
            assets.terrain_paint[&map.id]
                .sample(center.into_array())
                .unwrap()
                .0,
            [220, 40, 70, 255]
        );
        reopened.active_operation = "erase".into();
        reopened.begin_stroke(point(center));
        sync(&mut assets, map.id, &mut reopened);
        assert_eq!(
            assets.terrain_paint[&map.id]
                .sample(center.into_array())
                .unwrap()
                .0[3],
            0
        );
    }
}
