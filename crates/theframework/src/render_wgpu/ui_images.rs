//! Device-bound UI image cache. Widgets depend on UiImageResources, not this type.
use crate::{
    paint::ImageHandle,
    prelude::TheRGBABuffer,
    theui::paint::{UiImageResources, UiPaintError},
};
use renderkit::{GpuContext, ImageAsset, ImageRegistry};
use std::{collections::HashMap, sync::Arc};
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
}
/// Application-owned cache. A frame borrows the host registry, allowing UI,
/// scenes and custom application paints to share one handle/ownership domain.
#[derive(Default)]
pub struct UiImageCache {
    cached: HashMap<String, ImageHandle>,
    owner: Option<u64>,
}
pub struct UiFrameImages<'a> {
    pub registry: &'a mut ImageRegistry,
    cache: &'a mut UiImageCache,
}
impl UiImageCache {
    pub fn frame<'a>(&'a mut self, registry: &'a mut ImageRegistry) -> UiFrameImages<'a> {
        if self.owner != Some(registry.owner_id()) {
            self.cached.clear();
            self.owner = Some(registry.owner_id());
        }
        UiFrameImages {
            registry,
            cache: self,
        }
    }
    pub fn clear(&mut self, registry: &mut ImageRegistry) {
        for (_, handle) in self.cached.drain() {
            registry.remove(handle);
        }
    }
}
impl UiImageResources for UiImages {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        self.cache
            .frame(&mut self.registry)
            .image(key, source, changed)
    }
}
impl UiImageResources for UiFrameImages<'_> {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> Result<ImageHandle, UiPaintError> {
        if !changed
            && let Some(handle) = self.cache.cached.get(key)
            && self.registry.get(*handle).is_some()
        {
            return Ok(*handle);
        }
        let image = ImageAsset::rgba(
            source.pixel_width() as u32,
            source.pixel_height() as u32,
            source.pixels().to_vec(),
        )
        .map_err(|e| UiPaintError(e.to_string()))?;
        let handle = self
            .registry
            .upload_linear(&image)
            .map_err(|e| UiPaintError(e.to_string()))?;
        if let Some(old) = self.cache.cached.insert(key.into(), handle) {
            self.registry.remove(old);
        }
        Ok(handle)
    }
}
