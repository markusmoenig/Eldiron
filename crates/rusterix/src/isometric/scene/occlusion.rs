use super::{Sprite, SpriteImage, SpriteOwner};
use crate::{isometric::Projection, party::PlayerView};
use instant::Instant;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;
use vek::Vec2;

const OCCLUDER_OPACITY: f32 = 0.23;
const FADE_IN_SECONDS: f32 = 0.12;
const RESTORE_SECONDS: f32 = 0.25;
const HOLD_AFTER_OVERLAP: Duration = Duration::from_millis(140);

#[derive(Default)]
pub(super) struct ForegroundFade {
    view: Option<(Uuid, PlayerView)>,
    last_frame: Option<Instant>,
    fades: HashMap<SpriteOwner, Fade>,
}

struct Fade {
    opacity: f32,
    last_overlap: Instant,
}

impl ForegroundFade {
    pub(super) fn update(
        &mut self,
        view: (Uuid, PlayerView),
        sprites: &mut [Sprite<'_>],
        player: Option<usize>,
    ) {
        self.update_at(view, sprites, player, Instant::now());
    }

    fn update_at(
        &mut self,
        view: (Uuid, PlayerView),
        sprites: &mut [Sprite<'_>],
        player: Option<usize>,
        now: Instant,
    ) {
        if self.view != Some(view) {
            self.fades.clear();
            self.last_frame = None;
            self.view = Some(view);
        }
        let dt = self.last_frame.replace(now).map_or(0., |previous| {
            now.saturating_duration_since(previous)
                .as_secs_f32()
                .min(0.05)
        });
        let player = player
            .and_then(|index| {
                sprites.iter().enumerate().find(|(_, sprite)| {
                    matches!(sprite.owner, Some(SpriteOwner::Entity { index: i, .. }) if i == index)
                })
            })
            .and_then(|(order, sprite)| {
                Silhouette::new(sprite).map(|shape| (order, sprite.height, shape))
            });
        if player.is_none() && self.fades.is_empty() {
            return;
        }
        for (order, sprite) in sprites.iter_mut().enumerate() {
            let Some(owner) = sprite.owner else {
                continue;
            };
            if matches!(owner, SpriteOwner::Entity { .. }) {
                continue;
            }
            let blocked = player
                .as_ref()
                .is_some_and(|(actor_order, actor_height, actor)| {
                    let can_fade = sprite.support.or(sprite.floor).is_none_or(|sector| {
                        sector.properties.get_bool_default("iso_overpass", false)
                            && sprite.height > actor_height + 0.5
                    });
                    order > *actor_order && can_fade && actor.overlaps(sprite)
                });
            if blocked {
                self.fades
                    .entry(owner)
                    .or_insert(Fade {
                        opacity: 1.,
                        last_overlap: now,
                    })
                    .last_overlap = now;
            }
            if let Some(fade) = self.fades.get_mut(&owner) {
                if now.saturating_duration_since(fade.last_overlap) < HOLD_AFTER_OVERLAP {
                    fade.opacity = (fade.opacity - dt * (1. - OCCLUDER_OPACITY) / FADE_IN_SECONDS)
                        .max(OCCLUDER_OPACITY);
                } else {
                    fade.opacity =
                        (fade.opacity + dt * (1. - OCCLUDER_OPACITY) / RESTORE_SECONDS).min(1.);
                }
                sprite.opacity = sprite.opacity.min(fade.opacity);
            }
        }
        // Discard fades for removed sprites once the entire transition could have elapsed.
        self.fades.retain(|_, fade| {
            fade.last_overlap == now
                || (fade.opacity < 1.
                    && now.saturating_duration_since(fade.last_overlap)
                        < HOLD_AFTER_OVERLAP + Duration::from_secs(2))
        });
    }
}

struct Silhouette<'a> {
    image: SpriteImage<'a>,
    start: Vec2<f32>,
    scale: f32,
    end: Vec2<f32>,
}

// Use the same placement as both renderers, independent of pan and zoom.
const PROJECTION: Projection = Projection {
    origin: Vec2::new(0., 0.),
    unit: 1.,
};

impl<'a> Silhouette<'a> {
    fn new(actor: &Sprite<'a>) -> Option<Self> {
        let image = actor.image()?;
        let (start, scale) = actor.image_placement(&image, PROJECTION);
        let end = start + Vec2::new(image.width as f32, image.height as f32) * scale;
        Some(Self {
            image,
            start,
            scale,
            end,
        })
    }

    fn overlaps(&self, sprite: &Sprite<'_>) -> bool {
        let Some(info) = sprite.image_info(false).filter(|image| !image.ground) else {
            return false;
        };
        let (at, scale) = sprite.image_placement(&info, PROJECTION);
        let end = at + Vec2::new(info.width as f32, info.height as f32) * scale;
        if end.x <= self.start.x
            || at.x >= self.end.x
            || end.y <= self.start.y
            || at.y >= self.end.y
        {
            return false;
        }
        let Some(image) = sprite.image() else {
            return false;
        };
        // A small grid of opaque avatar pixels ignores transparent windows and
        // the empty corners of a tree sprite.
        let player = &self.image;
        let step_x = (player.width / 16).max(1);
        let step_y = (player.height / 24).max(1);
        let mut covered = 0;
        for y in (step_y / 2..player.height).step_by(step_y) {
            for x in (step_x / 2..player.width).step_by(step_x) {
                if player
                    .data
                    .get((y * player.width + x) * 4 + 3)
                    .copied()
                    .unwrap_or(0)
                    < 128
                {
                    continue;
                }
                let point = self.start + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * self.scale;
                let local = (point - at) / scale;
                if local.x >= 0.
                    && local.y >= 0.
                    && local.x < image.width as f32
                    && local.y < image.height as f32
                    && image
                        .data
                        .get((local.y as usize * image.width + local.x as usize) * 4 + 3)
                        .copied()
                        .unwrap_or(0)
                        >= 128
                {
                    covered += 1;
                    if covered >= 2 {
                        return true;
                    }
                }
            }
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::isometric::AvatarSprite;
    use std::sync::Arc;

    #[test]
    fn only_overlapping_foreground_scenery_fades() {
        let opaque = AvatarSprite {
            pixels: Arc::new(vec![255; 4 * 8 * 4]),
            surface: None,
            size: [4, 8],
            anchor: [2., 8.],
            pixels_per_unit: 8.,
        };
        let transparent = AvatarSprite {
            pixels: Arc::new(vec![0; 4 * 8 * 4]),
            ..opaque.clone()
        };
        let sector = crate::Sector::new(5, Vec::new());
        let sprite = |owner, avatar, x, z| Sprite {
            owner: Some(owner),
            avatar: Some(avatar),
            tile: None,
            stamp: None,
            scale: 1.,
            pos: Vec2::new(x, z),
            height: 0.,
            frame: 0,
            opacity: 1.,
            tie: 0,
            support: None,
            radius: 0.,
            floor: None,
        };
        let entity = |index| SpriteOwner::Entity {
            index,
            id: 0,
            creator_id: Uuid::nil(),
        };
        let mut floor = sprite(SpriteOwner::Sector(5), &opaque, -0.1, 0.1);
        floor.floor = Some(&sector);
        let mut sprites = [
            sprite(SpriteOwner::Sector(1), &opaque, 0.1, -0.1),
            sprite(entity(1), &opaque, 0., 0.),
            sprite(SpriteOwner::Sector(2), &opaque, -0.1, 0.1),
            sprite(SpriteOwner::Sector(3), &transparent, -0.1, 0.1),
            sprite(SpriteOwner::Sector(4), &opaque, 3., 4.),
            sprite(entity(0), &opaque, -0.1, 0.1),
            floor,
        ];
        sprites.sort_by(Sprite::painter_cmp);
        let mut fade = ForegroundFade::default();
        let view = (Uuid::nil(), PlayerView::Controlled(0));
        let now = Instant::now();
        for ms in [0, 40, 80, 120, 160] {
            for sprite in &mut sprites {
                sprite.opacity = 1.;
            }
            fade.update_at(view, &mut sprites, Some(1), now + Duration::from_millis(ms));
            let opacity = sprites
                .iter()
                .find(|s| s.owner == Some(SpriteOwner::Sector(2)))
                .unwrap()
                .opacity;
            if ms == 0 {
                assert_eq!(opacity, 1.);
            }
            if ms == 40 {
                assert!(opacity > OCCLUDER_OPACITY && opacity < 1.);
            }
        }
        for sprite in &sprites {
            let expected = match sprite.owner {
                Some(SpriteOwner::Sector(2)) => OCCLUDER_OPACITY,
                _ => 1.,
            };
            assert_eq!(sprite.opacity, expected, "{:?}", sprite.owner);
        }

        // A few clear frames during a walk cycle must not restore the scenery.
        let opacity_at = |fade: &mut ForegroundFade, sprites: &mut [Sprite<'_>], ms, overlaps| {
            for sprite in sprites.iter_mut() {
                sprite.opacity = 1.;
                if sprite.owner == Some(entity(1)) {
                    sprite.pos = if overlaps {
                        Vec2::zero()
                    } else {
                        Vec2::new(4., 4.)
                    };
                }
            }
            fade.update_at(view, sprites, Some(1), now + Duration::from_millis(ms));
            sprites
                .iter()
                .find(|s| s.owner == Some(SpriteOwner::Sector(2)))
                .unwrap()
                .opacity
        };
        assert_eq!(
            opacity_at(&mut fade, &mut sprites, 200, false),
            OCCLUDER_OPACITY
        );
        assert_eq!(
            opacity_at(&mut fade, &mut sprites, 240, true),
            OCCLUDER_OPACITY
        );
        for ms in [280, 320, 360] {
            assert_eq!(
                opacity_at(&mut fade, &mut sprites, ms, false),
                OCCLUDER_OPACITY
            );
        }
        let restoring = opacity_at(&mut fade, &mut sprites, 400, false);
        assert!(restoring > OCCLUDER_OPACITY && restoring < 1.);
        for ms in [440, 480, 520, 560, 600, 640, 680] {
            opacity_at(&mut fade, &mut sprites, ms, false);
        }
        assert_eq!(opacity_at(&mut fade, &mut sprites, 720, false), 1.);
    }
}
