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
    cached: HashMap<String, ImageHandle>,
}
impl UiImages {
    pub fn new(context: Arc<GpuContext>) -> Self {
        Self {
            registry: ImageRegistry::new(context),
            cached: HashMap::new(),
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
        if !changed
            && let Some(handle) = self.cached.get(key)
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
            .upload(&image)
            .map_err(|e| UiPaintError(e.to_string()))?;
        if let Some(old) = self.cached.insert(key.into(), handle) {
            self.registry.remove(old);
        }
        Ok(handle)
    }
}
