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
struct SpriteTarget {
    owner: SpriteOwner,
    position: Vec3<f32>,
    origin: [i32; 2],
    extent: [i32; 2],
    width: u32,
    height: u32,
    scale: f32,
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
    sprites: Vec<SpriteTarget>,
}
impl IsometricWorldTargets {
    pub(super) fn capture<'a>(
        input: &IsometricSceneInput<'_>,
        size: (u32, u32),
        sprites: impl Iterator<Item = &'a Sprite<'a>>,
        sources: &mut IsometricSources,
        elevation: ElevationMap,
        characters: bool,
    ) -> Result<Arc<Self>> {
        let leader = rusterix::party::resolve_party_entity(input.map, Some("leader"))
            .or_else(|| input.map.entities.iter().find(|e| e.is_player()))
            .map(|e| e.get_pos_xz())
            .unwrap_or(Vec2::zero());
        let mut targets = Vec::new();
        for sprite in sprites {
            let Some(owner @ (SpriteOwner::Item { .. } | SpriteOwner::Entity { .. })) =
                sprite.owner
            else {
                continue;
            };
            if matches!(owner, SpriteOwner::Entity { .. })
                && (!characters || sprite.opacity <= 0.125)
            {
                continue;
            }
            let Some(image) = sprite.image() else {
                continue;
            };
            let (at, scale) = sprite.image_placement(&image, input.projection);
            let origin = at.into_array().map(|v| v.round() as i32);
            let extent = [image.width, image.height].map(|v| (v as f32 * scale).round() as i32);
            let viewport = [size.0 as i32, size.1 as i32];
            if (0..2).any(|axis| {
                extent[axis] <= 0
                    || origin[axis] >= viewport[axis]
                    || origin[axis].saturating_add(extent[axis]) <= 0
            }) {
                continue;
            }
            let Some((source, alpha)) = sources.prepare_with_alpha(sprite, input.source_version)?
            else {
                continue;
            };
            let (width, height) = source.size();
            targets.push(SpriteTarget {
                owner,
                position: Vec3::new(sprite.pos.x, sprite.height, sprite.pos.y),
                origin,
                extent,
                width,
                height,
                scale,
                alpha,
            });
        }
        // Keep the established item selection depth/tie behavior. Source texels
        // use the GPU's rounded rectangle and nearest sampling for this frame.
        targets.sort_by(|a, b| {
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
            sprites: targets,
        }))
    }
    fn sprite_at(&self, uv: [f32; 2], include_entities: bool) -> Option<&SpriteTarget> {
        if uv.iter().any(|v| !v.is_finite() || !(0. ..1.).contains(v)) {
            return None;
        }
        let pixel = Vec2::new(uv[0] * self.size.0 as f32, uv[1] * self.size.1 as f32);
        let at = pixel.into_array().map(|v| v.floor() as i32);
        for item in self.sprites.iter().rev() {
            if !include_entities && !matches!(item.owner, SpriteOwner::Item { .. }) {
                continue;
            }
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
            let alpha = item.alpha[(y * item.width + x) as usize];
            if alpha > 32 {
                return Some(item);
            }
        }
        None
    }
    /// Exact source-alpha item selection for the displayed authoring frame.
    pub fn editor_item(&self, uv: [f32; 2]) -> Option<Uuid> {
        match self.sprite_at(uv, false)?.owner {
            SpriteOwner::Item { creator_id, .. } => Some(creator_id),
            _ => None,
        }
    }
    /// Check the drawn sprite silhouette, including portions above its ground cell.
    pub fn editor_character(&self, uv: [f32; 2]) -> Option<u32> {
        match self.editor_subject(uv)? {
            SpriteOwner::Entity { id, .. } => Some(id),
            _ => None,
        }
    }
    /// Frontmost character or item on the presented sprite silhouettes.
    pub fn editor_subject(&self, uv: [f32; 2]) -> Option<SpriteOwner> {
        Some(self.sprite_at(uv, true)?.owner)
    }
    pub fn hit(&self, uv: [f32; 2], prefer_item: bool) -> Option<WorldHit> {
        if uv
            .into_iter()
            .any(|v| !v.is_finite() || !(0. ..1.).contains(&v))
        {
            return None;
        }
        let pixel = Vec2::new(uv[0] * self.size.0 as f32, uv[1] * self.size.1 as f32);
        if let Some(item) = self.sprite_at(uv, false)
            && let SpriteOwner::Item { id, .. } = item.owner
        {
            return Some(WorldHit {
                region: self.region,
                item: Some(id),
                world: Some(item.position),
                distance: self
                    .leader
                    .distance(Vec2::new(item.position.x, item.position.z)),
                ..Default::default()
            });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_hover_uses_sprite_alpha_without_changing_item_picks() {
        let map = rusterix::Map::default();
        let creator = Uuid::new_v4();
        let sprite = |owner, depth, alpha: [u8; 2]| SpriteTarget {
            owner,
            position: Vec3::new(0., 0., depth),
            origin: [2, 2],
            extent: [2, 1],
            width: 2,
            height: 1,
            scale: 1.,
            alpha: Arc::from(alpha),
        };
        let targets = IsometricWorldTargets {
            region: map.id,
            size: (8, 8),
            projection: Projection {
                origin: Vec2::zero(),
                unit: 1.,
            },
            elevation: ElevationMap::from_map(&map),
            cells: CellTargets::capture_isometric(&map),
            leader: Vec2::zero(),
            sprites: vec![
                sprite(
                    SpriteOwner::Item {
                        index: 0,
                        id: 8,
                        creator_id: creator,
                    },
                    0.,
                    [255, 255],
                ),
                sprite(
                    SpriteOwner::Entity {
                        index: 0,
                        id: 9,
                        creator_id: creator,
                    },
                    1.,
                    [255, 0],
                ),
            ],
        };
        let opaque = [2.25 / 8., 2.25 / 8.];
        let transparent = [3.25 / 8., 2.25 / 8.];
        assert_eq!(targets.editor_character(opaque), Some(9));
        assert_eq!(targets.editor_character(transparent), None);
        assert_eq!(targets.editor_item(opaque), Some(creator));
        assert_eq!(targets.editor_character([1., 0.]), None);
    }
}
