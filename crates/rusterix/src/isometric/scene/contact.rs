//! Object/floor support policy, shared by CPU reference and GPU submission.
//! Masks and owners describe instances, never viewport pixels.
use super::{super::Projection, Sprite};
use crate::Map;
use vek::Clamp;

pub struct ContactPolicy {
    /// Zero belongs to supporting surfaces; other IDs index each protection mask.
    pub owner_by_instance: Vec<u32>,
    pub protected: Vec<Vec<bool>>,
}
impl ContactPolicy {
    pub fn new(map: &Map, sprites: &[Sprite<'_>]) -> Self {
        let objects: Vec<_> = sprites.iter().filter(|s| s.support.is_none()).collect();
        let protected = sprites
            .iter()
            .map(|floor| {
                let Some(surface) = floor.support else {
                    return Vec::new();
                };
                let bounds = surface.bounding_box(map);
                let mut mask = Vec::with_capacity(objects.len() + 1);
                mask.push(false);
                for object in &objects {
                    let nearest = object.pos.clamped(bounds.min, bounds.max);
                    mask.push(
                        object.painter_cmp(floor).is_lt()
                            && object.height + 0.001 >= floor.height
                            && (nearest - object.pos).magnitude_squared()
                                <= object.radius * object.radius + 0.000001
                            && surface
                                .is_inside(map, nearest + (bounds.center() - nearest) * 0.0001),
                    );
                }
                mask
            })
            .collect();
        let mut owner = 0;
        let owner_by_instance = sprites
            .iter()
            .map(|s| {
                if s.support.is_some() {
                    0
                } else {
                    owner += 1;
                    owner
                }
            })
            .collect();
        Self {
            owner_by_instance,
            protected,
        }
    }
    /// Reference fast-path bound only. Higher terrain and foreground props keep
    /// normal painter occlusion; no rectangular support shortcut changes it.
    pub fn affected_screen_bounds(
        &self,
        sprites: &[Sprite<'_>],
        projection: Projection,
    ) -> Option<[i32; 4]> {
        let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        for (object, owner) in sprites.iter().zip(&self.owner_by_instance) {
            if *owner == 0
                || !self
                    .protected
                    .iter()
                    .any(|m| m.get(*owner as usize) == Some(&true))
            {
                continue;
            }
            let Some(image) = object.image() else {
                continue;
            };
            let (at, scale) = object.image_placement(&image, projection);
            let [x, y] = at.into_array().map(|v| v.round() as i32);
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] =
                bounds[2].max(x.saturating_add((image.width as f32 * scale).round() as i32));
            bounds[3] =
                bounds[3].max(y.saturating_add((image.height as f32 * scale).round() as i32));
        }
        (bounds[0] < bounds[2] && bounds[1] < bounds[3]).then_some(bounds)
    }
}
