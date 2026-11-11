use super::ScreenImages;
use renderkit::{ImageAsset, ImageHandle, ImageRegistry};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceStamp {
    Pixels(u64),
    Revision {
        revision: u64,
        width: u32,
        height: u32,
    },
}
struct Entry {
    image: ImageHandle,
    stamp: SourceStamp,
    bytes: usize,
    frame: u64,
}
/// Device/registry-bound source asset cache. A frame's handles stay registered
/// through submission, including two revisions of the same source in one frame.
/// Old paint lists must be discarded before `begin_frame` is called again.
pub struct ScreenImageCache {
    entries: FxHashMap<String, Entry>,
    retired: Vec<(ImageHandle, usize)>,
    owner: Option<u64>,
    frame: u64,
    budget: usize,
    bytes: usize,
}
impl Default for ScreenImageCache {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024)
    }
}
impl ScreenImageCache {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: FxHashMap::default(),
            retired: Vec::new(),
            owner: None,
            frame: 0,
            budget,
            bytes: 0,
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
    pub fn begin_frame<'a>(
        &'a mut self,
        images: &'a mut ImageRegistry,
    ) -> Result<ScreenImageFrame<'a>, String> {
        self.check_registry(images)?;
        for (handle, bytes) in self.retired.drain(..) {
            images.remove(handle);
            self.bytes -= bytes;
        }
        self.owner = Some(images.owner_id());
        self.frame = self.frame.wrapping_add(1);
        Ok(ScreenImageFrame {
            cache: self,
            images,
        })
    }
    fn check_registry(&self, images: &ImageRegistry) -> Result<(), String> {
        if self.owner.is_some_and(|o| o != images.owner_id()) {
            return Err(
                "screen image cache belongs to another registry; recreate it with the presenter"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn clear(&mut self, images: &mut ImageRegistry) -> Result<(), String> {
        self.check_registry(images)?;
        for (_, entry) in self.entries.drain() {
            images.remove(entry.image);
        }
        for (image, _) in self.retired.drain(..) {
            images.remove(image);
        }
        self.bytes = 0;
        self.owner = None;
        Ok(())
    }
}
pub struct ScreenImageFrame<'a> {
    cache: &'a mut ScreenImageCache,
    images: &'a mut ImageRegistry,
}
impl ScreenImages for ScreenImageFrame<'_> {
    fn rgba_versioned(
        &mut self,
        key: &str,
        revision: u64,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<ImageHandle, String> {
        self.source(
            key,
            width,
            height,
            pixels,
            SourceStamp::Revision {
                revision,
                width,
                height,
            },
        )
    }
    fn rgba(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<ImageHandle, String> {
        let mut hash = FxHasher::default();
        (width, height, pixels).hash(&mut hash);
        self.source(
            key,
            width,
            height,
            pixels,
            SourceStamp::Pixels(hash.finish()),
        )
    }
}
impl ScreenImageFrame<'_> {
    fn source(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
        stamp: SourceStamp,
    ) -> Result<ImageHandle, String> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(pixels.len()) {
            return Err("invalid screen source image dimensions/bytes".into());
        }
        if let Some(entry) = self.cache.entries.get_mut(key) {
            if entry.stamp == stamp && self.images.get(entry.image).is_some() {
                entry.frame = self.cache.frame;
                return Ok(entry.image);
            }
        }
        let asset = ImageAsset::rgba(width, height, pixels.to_vec()).map_err(|e| e.to_string())?;
        let image = self
            .images
            .upload_linear(&asset)
            .map_err(|e| e.to_string())?;
        if let Some(old) = self.cache.entries.insert(
            key.into(),
            Entry {
                image,
                stamp,
                bytes: pixels.len() * 2,
                frame: self.cache.frame,
            },
        ) {
            // Earlier commands in this frame may still reference this revision.
            self.cache.retired.push((old.image, old.bytes));
        }
        self.cache.bytes += pixels.len() * 2;
        Ok(image)
    }
}
impl Drop for ScreenImageFrame<'_> {
    fn drop(&mut self) {
        // Never evict the current frame's working set. Oversized live sets are
        // observable through resident_bytes; they become eligible next frame.
        if self.cache.bytes <= self.cache.budget {
            return;
        }
        let mut candidates = self
            .cache
            .entries
            .iter()
            .filter(|(_, e)| e.frame != self.cache.frame)
            .map(|(key, e)| (key.clone(), e.frame))
            .collect::<Vec<_>>();
        candidates.sort_unstable_by_key(|(_, frame)| *frame);
        for (key, _) in candidates {
            if self.cache.bytes <= self.cache.budget {
                break;
            }
            if let Some(entry) = self.cache.entries.remove(&key) {
                self.cache.bytes -= entry.bytes;
                self.images.remove(entry.image);
            }
        }
    }
}
