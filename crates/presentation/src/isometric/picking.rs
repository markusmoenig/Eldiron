//! Immutable input receipts. CPU work queries source alpha and retained world
//! geometry at a requested point; it never rasterizes a destination image.
use super::{IsometricSceneInput, IsometricSources};
use renderkit::Result;
use rusterix::{
    isometric::{
        Projection,
        elevation::ElevationMap,
        scene::{Sprite, SpriteOwner},
    },
    world_interaction::{CellTargets, WorldHit},
};
use scenevm::prelude::{Vec2, Vec3};
use std::sync::Arc;
use theframework_core::Uuid;
struct ItemTarget {
    id: u32,
    position: Vec3<f32>,
    origin: [i32; 2],
    extent: [i32; 2],
    width: u32,
    height: u32,
    scale: f32,
    opacity: f32,
    alpha: Arc<[u8]>,
}
/// Retains the reference isometric policy: source-alpha item selection precedes
/// ground-cell living/item/dead targets. This is a source hit policy, not a
/// newly introduced depth/occlusion policy. Animation/scale use the drawn frame.
pub struct IsometricWorldTargets {
    region: Uuid,
    size: (u32, u32),
    projection: Projection,
    elevation: ElevationMap,
    cells: CellTargets,
    leader: Vec2<f32>,
    items: Vec<ItemTarget>,
}
impl IsometricWorldTargets {
    pub(super) fn capture<'a>(
        input: &IsometricSceneInput<'_>,
        size: (u32, u32),
        sprites: impl Iterator<Item = &'a Sprite<'a>>,
        sources: &mut IsometricSources,
        elevation: ElevationMap,
    ) -> Result<Arc<Self>> {
        let leader = rusterix::party::resolve_party_entity(input.map, Some("leader"))
            .or_else(|| input.map.entities.iter().find(|e| e.is_player()))
            .map(|e| e.get_pos_xz())
            .unwrap_or(Vec2::zero());
        let mut items = Vec::new();
        for sprite in sprites {
            let Some(SpriteOwner::Item { id, .. }) = sprite.owner else {
                continue;
            };
            let Some(image) = sprite.image() else {
                continue;
            };
            let Some((source, alpha)) = sources.prepare_with_alpha(sprite, input.source_version)?
            else {
                continue;
            };
            let (at, scale) = sprite.image_placement(&image, input.projection);
            let (width, height) = source.size();
            items.push(ItemTarget {
                id,
                position: Vec3::new(sprite.pos.x, sprite.height, sprite.pos.y),
                origin: at.into_array().map(|v| v.round() as i32),
                extent: [width, height].map(|v| (v as f32 * scale).round() as i32),
                width,
                height,
                scale,
                opacity: sprite.opacity,
                alpha,
            });
        }
        // Keep the established item selection depth/tie behavior. Source texels
        // use the GPU's rounded rectangle and nearest sampling for this frame.
        items.sort_by(|a, b| {
            (a.position.z - a.position.x + a.position.y * 0.01)
                .total_cmp(&(b.position.z - b.position.x + b.position.y * 0.01))
        });
        Ok(Arc::new(Self {
            region: input.map.id,
            size,
            projection: input.projection,
            elevation,
            cells: CellTargets::capture_isometric(input.map),
            leader,
            items,
        }))
    }
    pub fn hit(&self, uv: [f32; 2], prefer_item: bool) -> Option<WorldHit> {
        if uv
            .into_iter()
            .any(|v| !v.is_finite() || !(0. ..1.).contains(&v))
        {
            return None;
        }
        let pixel = Vec2::new(uv[0] * self.size.0 as f32, uv[1] * self.size.1 as f32);
        let at = pixel.into_array().map(|v| v.floor() as i32);
        for item in self.items.iter().rev() {
            let (Some(x), Some(y)) = (
                at[0].checked_sub(item.origin[0]),
                at[1].checked_sub(item.origin[1]),
            ) else {
                continue;
            };
            let local = [x, y];
            if item.scale <= 0.
                || !item.scale.is_finite()
                || local.into_iter().any(|v| v < 0)
                || local[0] >= item.extent[0]
                || local[1] >= item.extent[1]
            {
                continue;
            }
            let x = ((local[0] as f32 / item.scale) as u32).min(item.width - 1);
            let y = ((local[1] as f32 / item.scale) as u32).min(item.height - 1);
            let alpha = (item.alpha[(y * item.width + x) as usize] as f32 * item.opacity).round();
            if alpha > 32. {
                return Some(WorldHit {
                    region: self.region,
                    item: Some(item.id),
                    world: Some(item.position),
                    distance: self
                        .leader
                        .distance(Vec2::new(item.position.x, item.position.z)),
                    ..Default::default()
                });
            }
        }
        let world = self.elevation.pick(self.projection, pixel);
        self.cells
            .hit(Vec2::new(world.x, world.z), prefer_item)
            .map(|mut hit| {
                hit.world = Some(world);
                hit
            })
    }
}
