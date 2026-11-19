//! Source preparation precedes both input layout and painting. Source leases
//! survive eviction/device replacement; no UI-sized pixels or GPU live here.
use super::ScreenImages;
use rustc_hash::FxHashMap;
use rusterix::{
    Assets,
    client::content::{ContentRecipe, SourceFit},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use theframework_core::{ImageAsset, ImageHandle};

static NEXT_SOURCE_OWNER: AtomicU64 = AtomicU64::new(1);

/// Advance after palette/material/avatar/artwork edits, project replacement or
/// publication of asynchronously prepared appearance variants. A presenter
/// rebuild does not change this CPU source revision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContentRevision(pub u64);

pub struct PreparedContent {
    image: ImageAsset,
    key: String,
    pub fit: SourceFit,
    pub avatar: bool,
    /// Nontransparent bounds: [min_x, min_y, max_x_exclusive, max_y_exclusive].
    pub alpha_bounds: Option<[u32; 4]>,
}
impl PreparedContent {
    pub fn image(&self) -> &ImageAsset {
        &self.image
    }
    pub fn image_key(&self) -> &str {
        &self.key
    }
    pub fn resolve(&self, images: &mut dyn ScreenImages) -> Result<ImageHandle, String> {
        // The source key is unique for this immutable CPU lease. A rebuild
        // receives a new key, so revision zero never aliases changed bytes.
        images.rgba_versioned(
            &self.key,
            0,
            self.image.width(),
            self.image.height(),
            self.image.pixels(),
        )
    }
}
struct Entry {
    content: Option<Arc<PreparedContent>>,
    used: u64,
}
/// Bounded preparation cache, owned by a graphical application/session. Missing
/// recipes are cached too. External Arc leases may outlive cache eviction; the
/// byte count describes only cache-owned source storage.
pub struct ContentSources {
    owner: u64,
    entries: FxHashMap<(ContentRevision, u64), Entry>,
    serial: u64,
    clock: u64,
    budget: usize,
    entry_limit: usize,
    bytes: usize,
    builds: u64,
}
impl Default for ContentSources {
    fn default() -> Self {
        Self::new(32 * 1024 * 1024, 512)
    }
}
impl ContentSources {
    pub fn new(byte_budget: usize, entry_limit: usize) -> Self {
        Self {
            owner: NEXT_SOURCE_OWNER.fetch_add(1, Ordering::Relaxed),
            entries: FxHashMap::default(),
            serial: 0,
            clock: 0,
            budget: byte_budget,
            entry_limit,
            bytes: 0,
            builds: 0,
        }
    }
    pub fn resident_bytes(&self) -> usize {
        self.bytes
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn builds(&self) -> u64 {
        self.builds
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub fn prepare(
        &mut self,
        recipe: ContentRecipe<'_>,
        assets: &Assets,
        revision: ContentRevision,
    ) -> Result<Option<Arc<PreparedContent>>, String> {
        let resolved = recipe.resolve(assets);
        let key = (revision, resolved.fingerprint());
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            return Ok(entry.content.clone());
        }
        self.builds += 1;
        let content = resolved
            .build()
            .map(|art| {
                let image = ImageAsset::rgba(
                    u32::try_from(art.texture.width).map_err(|_| "source width overflow")?,
                    u32::try_from(art.texture.height).map_err(|_| "source height overflow")?,
                    art.texture.data.into(),
                )
                .map_err(|e| e.to_string())?;
                let alpha_bounds = if art.avatar {
                    let mut bounds = [image.width(), image.height(), 0, 0];
                    for (i, pixel) in image.pixels().chunks_exact(4).enumerate() {
                        if pixel[3] > 0 {
                            let (x, y) = (i as u32 % image.width(), i as u32 / image.width());
                            bounds[0] = bounds[0].min(x);
                            bounds[1] = bounds[1].min(y);
                            bounds[2] = bounds[2].max(x + 1);
                            bounds[3] = bounds[3].max(y + 1);
                        }
                    }
                    (bounds[0] < bounds[2] && bounds[1] < bounds[3]).then_some(bounds)
                } else {
                    None
                };
                if image.width() == 0 || image.height() == 0 {
                    return Err("empty content source image".into());
                }
                self.serial += 1;
                Ok::<_, String>(Arc::new(PreparedContent {
                    image,
                    key: format!("content/{}/{}", self.owner, self.serial),
                    fit: art.fit,
                    avatar: art.avatar,
                    alpha_bounds,
                }))
            })
            .transpose()?;
        self.bytes += content.as_ref().map_or(0, |c| c.image.pixels().len());
        self.entries.insert(
            key,
            Entry {
                content: content.clone(),
                used: self.clock,
            },
        );
        // Trim even if the newest source alone exceeds the budget. The returned
        // lease stays valid while cache-owned storage remains strictly bounded.
        while self.bytes > self.budget || self.entries.len() > self.entry_limit {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k);
            let Some(oldest) = oldest else {
                break;
            };
            let removed = self.entries.remove(&oldest).unwrap();
            self.bytes -= removed
                .content
                .as_ref()
                .map_or(0, |c| c.image.pixels().len());
        }
        Ok(content)
    }
}
