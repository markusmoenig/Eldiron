//! Reuse caller-owned GPU/alpha leases after bounded strong-cache eviction.
//! Pointer keys require CPU allocation guards even though GPU data owns no CPU
//! pixels: weak guards prevent address reuse and detach Arc::make_mut edits.
use super::*;
use rusterix::isometric::surface::SurfaceFrame;
use std::sync::Weak;

enum Guard {
    Tile,
    Avatar(Weak<Vec<u8>>, Option<Weak<SurfaceFrame>>),
    Stamp(Weak<StampSprite>),
}
impl Guard {
    fn new(sprite: &Sprite<'_>) -> Self {
        if let Some(stamp) = sprite.stamp {
            Self::Stamp(Arc::downgrade(&stamp.sprite))
        } else if let Some(avatar) = sprite.avatar {
            Self::Avatar(
                Arc::downgrade(&avatar.pixels),
                avatar.surface.as_ref().map(Arc::downgrade),
            )
        } else {
            Self::Tile
        }
    }
    fn matches(&self, sprite: &Sprite<'_>) -> bool {
        fn same<T>(guard: &Weak<T>, current: &Arc<T>) -> bool {
            guard.ptr_eq(&Arc::downgrade(current)) && guard.strong_count() != 0
        }
        match self {
            Self::Tile => {
                sprite.stamp.is_none() && sprite.avatar.is_none() && sprite.tile.is_some()
            }
            Self::Avatar(pixels, surface) => {
                sprite.stamp.is_none()
                    && sprite.avatar.is_some_and(|avatar| {
                        same(pixels, &avatar.pixels)
                            && match (surface, &avatar.surface) {
                                (Some(guard), Some(current)) => same(guard, current),
                                (None, None) => true,
                                _ => false,
                            }
                    })
            }
            Self::Stamp(guard) => sprite.stamp.is_some_and(|stamp| same(guard, &stamp.sprite)),
        }
    }
}
struct Published {
    source: Weak<GpuSpriteSource>,
    alpha: Option<Weak<[u8]>>,
    guard: Guard,
}
#[derive(Default)]
pub(super) struct Publications(FxHashMap<SourceKey, Published>);
impl Publications {
    pub(super) fn source(
        &self,
        key: &SourceKey,
        sprite: &Sprite<'_>,
    ) -> Option<Arc<GpuSpriteSource>> {
        let entry = self
            .0
            .get(key)
            .filter(|entry| entry.guard.matches(sprite))?;
        entry.source.upgrade()
    }
    pub(super) fn alpha(&self, key: &SourceKey, sprite: &Sprite<'_>) -> Option<Arc<[u8]>> {
        let entry = self
            .0
            .get(key)
            .filter(|entry| entry.guard.matches(sprite))?;
        entry.alpha.as_ref()?.upgrade()
    }
    pub(super) fn publish(
        &mut self,
        key: SourceKey,
        sprite: &Sprite<'_>,
        source: &Arc<GpuSpriteSource>,
    ) {
        let alpha = self.alpha(&key, sprite);
        self.0.retain(|_, entry| {
            entry.source.strong_count() != 0
                || entry
                    .alpha
                    .as_ref()
                    .is_some_and(|alpha| alpha.strong_count() != 0)
        });
        self.0.insert(
            key,
            Published {
                source: Arc::downgrade(source),
                alpha: alpha.as_ref().map(Arc::downgrade),
                guard: Guard::new(sprite),
            },
        );
    }
    pub(super) fn publish_alpha(&mut self, key: &SourceKey, alpha: &Arc<[u8]>) {
        self.0
            .get_mut(key)
            .expect("source published before alpha")
            .alpha = Some(Arc::downgrade(alpha));
    }
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }
}
