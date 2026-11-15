//! Publish only the current immutable stamp artwork/anchors into CPU Assets.
use crate::iso_paint::{IsoPaintChunk, IsoPaintLayer, IsoPaintStamp};
use rusterix::stamp_paint::{StampPaint, StampRecipe};
use std::sync::Arc;
use theframework_core::Uuid;

pub fn sync(assets: &mut rusterix::Assets, map: Uuid, layer: &mut IsoPaintLayer) -> bool {
    if !layer.visible {
        return assets.stamp_paint.remove(&map).is_some();
    }
    layer.refresh_sources(assets);
    let stamps: Vec<_> = layer
        .chunks
        .values()
        .flat_map(|chunk| &chunk.stamps)
        .filter(|stamp| {
            stamp.world.is_some() || stamp.paint_geo.is_some() && stamp.surface_uv.is_some()
        })
        .map(|stamp| StampRecipe {
            id: stamp.id,
            kind: stamp.kind.clone(),
            variant: stamp.variant.clone(),
            world: stamp.world,
            surface_uv: stamp.surface_uv,
            paint_geo: stamp.paint_geo,
            size: stamp.size,
            camera_scale: stamp.camera_scale,
            viewport_size: stamp.viewport_size,
            rotation: stamp.rotation,
            variation: stamp.variation,
            material_id: stamp.material_id,
            color: stamp.color,
            palette_colors: stamp.palette_colors.clone(),
            opacity: stamp.opacity,
            source_image: stamp.source_image.clone(),
            source_material_pass: stamp.source_material_pass,
        })
        .collect();
    if stamps.is_empty() {
        return assets.stamp_paint.remove(&map).is_some();
    }
    let next = StampPaint { stamps };
    if assets
        .stamp_paint
        .get(&map)
        .is_some_and(|old| old.as_ref() == &next)
    {
        return false;
    }
    assets.stamp_paint.insert(map, Arc::new(next));
    true
}

/// Adapt the render recipe fields only. Order, UV/world anchors, authored scale
/// and shared image backing survive; editable strokes/selections never cross.
pub fn presentation_layer(source: &StampPaint) -> IsoPaintLayer {
    let mut layer = IsoPaintLayer::default();
    let mut chunk = IsoPaintChunk::new([0; 2]);
    for (order, stamp) in source.stamps.iter().enumerate() {
        chunk.stamps.push(IsoPaintStamp {
            source: None,
            source_image: stamp.source_image.clone(),
            source_material_pass: stamp.source_material_pass,
            id: stamp.id,
            order: order as u64,
            kind: stamp.kind.clone(),
            screen: [0; 2],
            world: stamp.world,
            surface_uv: stamp.surface_uv,
            paint_geo: stamp.paint_geo,
            surface_normal: None,
            owner: None,
            sort_depth: 0.,
            size: stamp.size,
            camera_scale: stamp.camera_scale,
            viewport_size: stamp.viewport_size,
            variant: stamp.variant.clone(),
            rotation: stamp.rotation,
            variation: stamp.variation,
            material_id: stamp.material_id,
            color: stamp.color,
            palette_indices: Vec::new(),
            palette_colors: stamp.palette_colors.clone(),
            opacity: stamp.opacity,
            screen_bounds: [0; 4],
        });
    }
    layer.chunks.insert("runtime-stamps".into(), chunk);
    layer
}
