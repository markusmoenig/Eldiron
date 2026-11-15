//! Bounded GPU residency of immutable sprite sources. Revisioned tile data is
//! borrowed; dynamic Arc sources are leased to prevent pointer-identity reuse.
use crate::screen::ContentRevision;
use renderkit::{GpuContext, Result};
use rustc_hash::FxHashMap;
use rusterix::isometric::{AvatarSprite, paint::StampSprite, scene::Sprite};
use scenevm::isometric::GpuSpriteSource;
use std::sync::Arc;
use theframework_core::Uuid;

mod publications;
use publications::Publications;

/// The application supplies its session/document incarnation and advances the
/// content revision after tile/source edits. Device replacement creates a new
/// residency owner; it does not change CPU sources or serialized project data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IsometricSourceVersion {
    pub namespace: Uuid,
    pub content: ContentRevision,
    pub tiles: u64,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SourceKey {
    Tile(IsometricSourceVersion, Uuid, usize),
    Avatar(Uuid, usize, usize),
    Stamp(Uuid, Uuid, u64, usize),
}
struct Entry {
    source: Arc<GpuSpriteSource>,
    used: u64,
    alpha: Option<Arc<[u8]>>,
    _avatar: Option<AvatarSprite>,
    _stamp: Option<Arc<StampSprite>>,
}
/// The cache counts GPU source bytes. Avatar/stamp leases share the existing
/// CPU assets; optional derived alpha is bounded to one byte per source texel.
/// No viewport pixel copy is allocated.
pub struct IsometricSources {
    context: Arc<GpuContext>,
    entries: FxHashMap<SourceKey, Entry>,
    publications: Publications,
    bytes: u64,
    byte_budget: u64,
    entry_limit: usize,
    clock: u64,
    builds: u64,
}
impl IsometricSources {
    pub fn new(context: Arc<GpuContext>, byte_budget: u64, entry_limit: usize) -> Self {
        Self {
            context,
            entries: FxHashMap::default(),
            publications: Publications::default(),
            bytes: 0,
            byte_budget,
            entry_limit,
            clock: 0,
            builds: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn resident_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn builds(&self) -> u64 {
        self.builds
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.publications.clear();
        self.bytes = 0;
    }
    /// Source-space alpha for explicit frame picking. It is generated once per
    /// live source, at most one byte per texel. Cache ownership is bounded by the
    /// GPU budget; receipts retain their alpha lease after edits/eviction.
    pub fn prepare_with_alpha(
        &mut self,
        sprite: &Sprite<'_>,
        version: IsometricSourceVersion,
    ) -> Result<Option<(Arc<GpuSpriteSource>, Arc<[u8]>)>> {
        let Some(source) = self.prepare(sprite, version)? else {
            return Ok(None);
        };
        let Some((key, _, _)) = source_identity(sprite, version) else {
            return Ok(None);
        };
        if let Some(alpha) = self
            .entries
            .get(&key)
            .and_then(|entry| entry.alpha.clone())
            .or_else(|| self.publications.alpha(&key, sprite))
        {
            return Ok(Some((source, alpha)));
        }
        let Some(image) = sprite.image() else {
            return Ok(None);
        };
        let alpha: Arc<[u8]> = image.data.chunks_exact(4).map(|p| p[3]).collect();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.alpha = Some(alpha.clone());
        }
        self.publications.publish_alpha(&key, &alpha);
        Ok(Some((source, alpha)))
    }
    /// Sources are shared by instances and camera positions. Returned GPU leases
    /// remain usable when an entry is evicted, including an oversized new source.
    pub fn prepare(
        &mut self,
        sprite: &Sprite<'_>,
        version: IsometricSourceVersion,
    ) -> Result<Option<Arc<GpuSpriteSource>>> {
        if self.context.is_lost() {
            return Err(renderkit::RenderError(
                "isometric source GPU device lost".into(),
            ));
        }
        let Some((key, avatar, stamp)) = source_identity(sprite, version) else {
            return Ok(None);
        };
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            return Ok(Some(entry.source.clone()));
        }
        if let Some(source) = self.publications.source(&key, sprite) {
            return Ok(Some(source));
        }
        let Some(image) = sprite.image() else {
            return Ok(None);
        };
        let width = u32::try_from(image.width)
            .map_err(|_| renderkit::RenderError("isometric source width overflow".into()))?;
        let height = u32::try_from(image.height)
            .map_err(|_| renderkit::RenderError("isometric source height overflow".into()))?;
        let source = GpuSpriteSource::new(
            &self.context,
            width,
            height,
            image.data,
            image.surface.map(|s| s.data.as_slice()),
        )?;
        self.builds += 1;
        let alpha = self.publications.alpha(&key, sprite);
        self.publications.publish(key, sprite, &source);
        self.bytes += source.resident_bytes();
        self.entries.insert(
            key,
            Entry {
                source: source.clone(),
                used: self.clock,
                alpha,
                _avatar: avatar,
                _stamp: stamp,
            },
        );
        while self.bytes > self.byte_budget || self.entries.len() > self.entry_limit {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            self.bytes -= self
                .entries
                .remove(&oldest)
                .unwrap()
                .source
                .resident_bytes();
        }
        Ok(Some(source))
    }
}

fn source_identity(
    sprite: &Sprite<'_>,
    version: IsometricSourceVersion,
) -> Option<(SourceKey, Option<AvatarSprite>, Option<Arc<StampSprite>>)> {
    Some(if let Some(stamp) = sprite.stamp {
        (
            SourceKey::Stamp(
                version.namespace,
                stamp.id,
                stamp.raster_key,
                Arc::as_ptr(&stamp.sprite) as usize,
            ),
            None,
            Some(stamp.sprite.clone()),
        )
    } else if let Some(avatar) = sprite.avatar {
        (
            SourceKey::Avatar(
                version.namespace,
                Arc::as_ptr(&avatar.pixels) as usize,
                avatar
                    .surface
                    .as_ref()
                    .map_or(0, |s| Arc::as_ptr(s) as usize),
            ),
            Some(avatar.clone()),
            None,
        )
    } else if let Some(tile) = sprite.tile {
        (SourceKey::Tile(version, tile.id, sprite.frame), None, None)
    } else {
        return None;
    })
}
