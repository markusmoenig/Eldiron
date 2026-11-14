//! Device-bound UI source-image cache. Widgets depend only on UiImageResources.
//! Finalize a complete paint list before rendering: retained commands pin every
//! referenced version, and only unused entries are eligible for LRU retirement.
use crate::{
    paint::{ImageHandle, PaintList},
    prelude::TheRGBABuffer,
    theui::paint::{UiImageResources, UiPaintError, UiPaintResult},
};
use renderkit::{AlphaMode, ColorSpace, GpuContext, ImageAsset, ImageRegistry};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const DEFAULT_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_IMAGES: usize = 512;

pub struct UiImages {
    pub registry: ImageRegistry,
    cache: UiImageCache,
}
impl UiImages {
    pub fn new(context: Arc<GpuContext>) -> Self {
        Self {
            registry: ImageRegistry::new(context),
            cache: UiImageCache::default(),
        }
    }
    /// Finish all UI/application contributions together, before GPU execution.
    pub fn finish_frame(&mut self, list: &PaintList) -> UiPaintResult {
        self.cache.finish_frame(&mut self.registry, list)
    }
    pub fn stats(&self) -> UiImageCacheStats {
        self.cache.stats()
    }
    pub fn clear(&mut self) {
        self.cache.clear(&mut self.registry);
    }
}

struct CachedImage {
    handle: ImageHandle,
    width: usize,
    height: usize,
    bytes: usize,
    used: u64,
}
/// Cache-owned RGBA16F resources only; scene outputs and glyph atlases are
/// accounted by their producers. Pinned frame working sets can exceed the
/// retention budget and report that explicitly instead of invalidating paint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiImageCacheStats {
    pub resident_bytes: usize,
    pub resident_images: usize,
    pub frame_pinned_bytes: usize,
    pub frame_pinned_images: usize,
    pub over_budget: bool,
    pub hits: u64,
    pub uploads: u64,
    pub evictions: u64,
}
/// Application-owned cache sharing the host's image registry. `frame` borrows
/// resource services; `finish_frame` is the single retirement boundary, after
/// all UI, viewport and overlay lists have been assembled. Retained lists must
/// be included in that final list even when no source image was requested anew.
/// A superseded version remains valid until a subsequent finalization omits it.
pub struct UiImageCache {
    cached: HashMap<(String, AlphaMode, ColorSpace), CachedImage>,
    retired: Vec<CachedImage>,
    owner: Option<u64>,
    byte_budget: usize,
    image_budget: usize,
    clock: u64,
    stats: UiImageCacheStats,
}
impl Default for UiImageCache {
    fn default() -> Self {
        Self::with_budget(DEFAULT_BYTES, DEFAULT_IMAGES)
    }
}
pub struct UiFrameImages<'a> {
    pub registry: &'a mut ImageRegistry,
    cache: &'a mut UiImageCache,
}
impl UiImageCache {
    pub fn with_budget(bytes: usize, images: usize) -> Self {
        Self {
            cached: HashMap::new(),
            retired: Vec::new(),
            owner: None,
            byte_budget: bytes,
            image_budget: images,
            clock: 0,
            stats: UiImageCacheStats::default(),
        }
    }
    pub fn stats(&self) -> UiImageCacheStats {
        self.stats
    }
    fn domain(&mut self, registry: &ImageRegistry) {
        if self.owner != Some(registry.owner_id()) {
            // The previous host/device domain owns its registry and teardown.
            // Never try to retire its handles in this different registry.
            self.cached.clear();
            self.retired.clear();
            self.owner = Some(registry.owner_id());
            self.stats = UiImageCacheStats::default();
            self.clock = 0;
        }
    }
    pub fn frame<'a>(&'a mut self, registry: &'a mut ImageRegistry) -> UiFrameImages<'a> {
        self.domain(registry);
        UiFrameImages {
            registry,
            cache: self,
        }
    }
    fn tick(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }
    fn account(&mut self) {
        self.stats.resident_images = self.cached.len() + self.retired.len();
        self.stats.resident_bytes = self
            .cached
            .values()
            .chain(self.retired.iter())
            .map(|entry| entry.bytes)
            .sum();
        self.stats.over_budget = self.stats.resident_bytes > self.byte_budget
            || self.stats.resident_images > self.image_budget;
    }
    /// Validate and pin the *complete* assembled list before retiring anything.
    /// On malformed scopes/stale handles this leaves residency untouched. WGPU
    /// submissions retain their resources after encoding; callers must finish
    /// rendering this list before finalizing a newer frame, or keep its handles
    /// in that newer list. Independent queued frames require explicit leases.
    pub fn finish_frame(
        &mut self,
        registry: &mut ImageRegistry,
        list: &PaintList,
    ) -> UiPaintResult {
        list.validate().map_err(|e| UiPaintError(e.into()))?;
        let pinned: HashSet<_> = list.image_handles().collect();
        if pinned.iter().any(|handle| registry.get(*handle).is_none()) {
            return Err(UiPaintError("paint references a stale source image".into()));
        }
        self.domain(registry);
        let used = self.tick();
        self.stats.frame_pinned_bytes = 0;
        self.stats.frame_pinned_images = 0;
        for entry in self.cached.values_mut().chain(self.retired.iter_mut()) {
            if pinned.contains(&entry.handle) {
                entry.used = used;
                self.stats.frame_pinned_bytes += entry.bytes;
                self.stats.frame_pinned_images += 1;
            }
        }
        // Unused superseded versions never need a cache slot, even under budget.
        let mut keep = Vec::new();
        for entry in self.retired.drain(..) {
            if pinned.contains(&entry.handle) {
                keep.push(entry);
            } else {
                registry.remove(entry.handle);
                self.stats.evictions += 1;
            }
        }
        self.retired = keep;
        // A source may also have been retired explicitly by its owning host.
        self.cached
            .retain(|_, entry| registry.get(entry.handle).is_some());
        self.account();
        let mut candidates: Vec<_> = self
            .cached
            .iter()
            .filter(|(_, entry)| !pinned.contains(&entry.handle))
            .map(|(key, entry)| (entry.used, key.clone()))
            .collect();
        candidates.sort_unstable_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.0.cmp(&b.1.0))
                .then_with(|| (a.1.1 as u8).cmp(&(b.1.1 as u8)))
                .then_with(|| (a.1.2 as u8).cmp(&(b.1.2 as u8)))
        });
        for (_, key) in candidates {
            if !self.stats.over_budget {
                break;
            }
            let entry = self.cached.remove(&key).unwrap();
            registry.remove(entry.handle);
            self.stats.evictions += 1;
            self.account();
        }
        Ok(())
    }
    /// Explicit owner teardown. Previously published lists become invalid;
    /// application/document close must discard them before requesting new paint.
    pub fn clear(&mut self, registry: &mut ImageRegistry) {
        for entry in self
            .cached
            .drain()
            .map(|(_, entry)| entry)
            .chain(self.retired.drain(..))
        {
            registry.remove(entry.handle);
        }
        self.owner = Some(registry.owner_id());
        self.clock = 0;
        self.stats = UiImageCacheStats::default();
    }
    fn image(
        &mut self,
        registry: &mut ImageRegistry,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.image_with_alpha(registry, key, source, changed, AlphaMode::Straight)
    }
    fn image_with_alpha(
        &mut self,
        registry: &mut ImageRegistry,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
        alpha: AlphaMode,
    ) -> Result<ImageHandle, UiPaintError> {
        self.import(
            registry,
            key,
            source.pixel_width(),
            source.pixel_height(),
            alpha,
            ColorSpace::Srgb,
            changed,
            |registry| {
                let mut image = ImageAsset::rgba(
                    u32::try_from(source.pixel_width())
                        .map_err(|_| UiPaintError("source image too wide".into()))?,
                    u32::try_from(source.pixel_height())
                        .map_err(|_| UiPaintError("source image too tall".into()))?,
                    source.pixels().to_vec(),
                )
                .map_err(|e| UiPaintError(e.to_string()))?;
                image.alpha_mode = alpha;
                registry
                    .upload_linear(&image)
                    .map_err(|e| UiPaintError(e.to_string()))
            },
        )
    }
    fn asset(
        &mut self,
        registry: &mut ImageRegistry,
        key: &str,
        source: &ImageAsset,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.import(
            registry,
            key,
            source.width() as usize,
            source.height() as usize,
            source.alpha_mode,
            source.color_space,
            changed,
            |registry| {
                registry
                    .upload_linear(source)
                    .map_err(|e| UiPaintError(e.to_string()))
            },
        )
    }
    fn import(
        &mut self,
        registry: &mut ImageRegistry,
        key: &str,
        width: usize,
        height: usize,
        alpha: AlphaMode,
        color: ColorSpace,
        changed: bool,
        upload: impl FnOnce(&mut ImageRegistry) -> Result<ImageHandle, UiPaintError>,
    ) -> Result<ImageHandle, UiPaintError> {
        self.domain(registry);
        let used = self.tick();
        let cache_key = (key.to_owned(), alpha, color);
        if !changed
            && let Some(entry) = self.cached.get_mut(&cache_key)
            && entry.width == width
            && entry.height == height
            && registry.get(entry.handle).is_some()
        {
            entry.used = used;
            self.stats.hits += 1;
            return Ok(entry.handle);
        }
        let bytes = width
            .checked_mul(height)
            .and_then(|count| count.checked_mul(8))
            .ok_or_else(|| UiPaintError("source image size overflow".into()))?;
        let handle = upload(registry)?;
        let entry = CachedImage {
            handle,
            width,
            height,
            bytes,
            used,
        };
        if let Some(old) = self.cached.insert(cache_key, entry) {
            self.retired.push(old);
        }
        self.stats.uploads += 1;
        self.account();
        Ok(handle)
    }
}
impl UiImageResources for UiImages {
    fn retained_domain(&self) -> Option<u64> {
        Some(self.registry.owner_id())
    }
    fn retained_images_valid(&self, commands: &PaintList) -> bool {
        commands.image_handles().all(|handle| self.registry.get(handle).is_some())
    }

    fn asset(
        &mut self,
        key: &str,
        source: &ImageAsset,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache.asset(&mut self.registry, key, source, changed)
    }

    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache.image(&mut self.registry, key, source, changed)
    }
    fn copied_framebuffer(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache
            .image_with_alpha(&mut self.registry, key, source, changed, AlphaMode::Opaque)
    }
}
impl UiFrameImages<'_> {
    pub fn finish_frame(&mut self, list: &PaintList) -> UiPaintResult {
        self.cache.finish_frame(self.registry, list)
    }
    pub fn stats(&self) -> UiImageCacheStats {
        self.cache.stats()
    }
}
impl UiImageResources for UiFrameImages<'_> {
    fn retained_domain(&self) -> Option<u64> {
        Some(self.registry.owner_id())
    }
    fn retained_images_valid(&self, commands: &PaintList) -> bool {
        commands.image_handles().all(|handle| self.registry.get(handle).is_some())
    }

    fn asset(
        &mut self,
        key: &str,
        source: &ImageAsset,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache.asset(self.registry, key, source, changed)
    }

    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache.image(self.registry, key, source, changed)
    }
    fn copied_framebuffer(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache
            .image_with_alpha(self.registry, key, source, changed, AlphaMode::Opaque)
    }
}
