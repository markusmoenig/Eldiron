//! CPU source packing retains immutable image handles, never a device borrow.
//! The reference and live paths consume the same ordered metadata/pixel kernel.
use super::*;
use crate::source_upload::SourceUploadBudget;

enum Pixels {
    Rgba(Arc<[u8]>),
    Mask(Arc<[u32]>),
}
impl Pixels {
    fn len(&self) -> usize {
        match self {
            Self::Rgba(bytes) => bytes.len() / 4,
            Self::Mask(words) => words.len(),
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rgba(a), Self::Rgba(b)) => Arc::ptr_eq(a, b),
            (Self::Mask(a), Self::Mask(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
struct Source {
    header: [u32; 4],
    pixels: Pixels,
}
struct Snapshot {
    sources: Vec<Source>,
    words: usize,
}
impl Snapshot {
    fn capture(vm: &VM, limit: u64) -> crate::SceneVMResult<Self> {
        let mut sources = Vec::new();
        let mut pixel_words = 0usize;
        let mut push = |width, height, source_id, pixels: Pixels| -> crate::SceneVMResult<()> {
            let offset = u32::try_from(pixel_words)
                .ok()
                .filter(|offset| *offset < 0x8000_0000)
                .ok_or_else(|| error("organic pixel offset exceeds its packed range"))?;
            let header = if let Some(id) = source_id {
                [id, width, height, offset | 0x8000_0000]
            } else {
                [offset, width, height, 0]
            };
            pixel_words = pixel_words
                .checked_add(pixels.len())
                .ok_or_else(|| error("organic pixel size overflow"))?;
            sources.push(Source { header, pixels });
            let words = sources
                .len()
                .checked_mul(4)
                .and_then(|count| count.checked_add(pixel_words))
                .ok_or_else(|| error("organic source size overflow"))?;
            if words as u64 > limit / 4 {
                return Err(error("organic source exceeds its buffer budget"));
            }
            Ok(())
        };
        for sprite in &vm.organic_billboards.sprites {
            push(
                sprite.width,
                sprite.height,
                None,
                Pixels::Rgba(sprite.rgba.clone()),
            )?;
        }
        for chunk in vm.chunks_map.values() {
            for sprite in &chunk.organic_billboard_sprites {
                push(
                    sprite.width,
                    sprite.height,
                    None,
                    Pixels::Rgba(sprite.rgba.clone()),
                )?;
            }
        }
        for sprite in &vm.paint_billboards.sprites {
            push(
                sprite.width,
                sprite.height,
                None,
                Pixels::Rgba(sprite.rgba.clone()),
            )?;
        }
        for sprite in &vm.paint_billboards.paged_sprites {
            push(
                sprite.width,
                sprite.height,
                Some(sprite.source_id),
                Pixels::Mask(sprite.alpha_mask.clone()),
            )?;
        }
        let words = sources
            .len()
            .checked_mul(4)
            .and_then(|count| count.checked_add(pixel_words))
            .ok_or_else(|| error("organic source size overflow"))?;
        Ok(Self { sources, words })
    }
    fn same(&self, other: &Self) -> bool {
        self.words == other.words
            && self.sources.len() == other.sources.len()
            && self
                .sources
                .iter()
                .zip(&other.sources)
                .all(|(a, b)| a.header == b.header && a.pixels.same(&b.pixels))
    }
}
fn error(message: &str) -> crate::SceneVMError {
    crate::SceneVMError::BufferAllocationFailed(message.into())
}
struct Pack {
    snapshot: Arc<Snapshot>,
    words: Vec<u32>,
    headers: usize,
    source: usize,
    pixel: usize,
    previous: Option<Arc<Vec<u32>>>,
    same_previous: bool,
}
impl Pack {
    fn new(snapshot: Arc<Snapshot>, previous: Option<Arc<Vec<u32>>>) -> crate::SceneVMResult<Self> {
        let mut words = Vec::new();
        words
            .try_reserve_exact(snapshot.words)
            .map_err(|_| error("cannot reserve organic source words"))?;
        let same_previous = previous
            .as_ref()
            .is_some_and(|old| old.len() == snapshot.words);
        Ok(Self {
            snapshot,
            words,
            headers: 0,
            source: 0,
            pixel: 0,
            previous,
            same_previous,
        })
    }
    fn compare(&mut self, start: usize) {
        if self.same_previous {
            self.same_previous =
                self.previous.as_ref().unwrap()[start..self.words.len()] == self.words[start..];
        }
    }
    fn advance(&mut self, budget: &mut SourceUploadBudget) -> bool {
        while self.headers < self.snapshot.sources.len() {
            let count = (budget.available_cpu_words() / 4)
                .min(256)
                .min(self.snapshot.sources.len() - self.headers);
            if count == 0 {
                return false;
            }
            let start = self.words.len();
            for source in &self.snapshot.sources[self.headers..self.headers + count] {
                self.words.extend_from_slice(&source.header);
            }
            self.headers += count;
            self.compare(start);
            budget.record_cpu_words(count * 4);
        }
        while let Some(source) = self.snapshot.sources.get(self.source) {
            let count = budget
                .available_cpu_words()
                .min(16 * 1024)
                .min(source.pixels.len() - self.pixel);
            if source.pixels.len() == self.pixel {
                if budget.available_cpu_words() == 0 {
                    return false;
                }
                self.source += 1;
                self.pixel = 0;
                budget.record_cpu_step();
                continue;
            }
            if count == 0 {
                return false;
            }
            let start = self.words.len();
            let end = self.pixel + count;
            match &source.pixels {
                Pixels::Rgba(bytes) => self.words.extend(
                    bytes[self.pixel * 4..end * 4]
                        .chunks_exact(4)
                        .map(|px| u32::from_le_bytes([px[0], px[1], px[2], px[3]])),
                ),
                Pixels::Mask(words) => self.words.extend_from_slice(&words[self.pixel..end]),
            }
            self.pixel = end;
            self.compare(start);
            budget.record_cpu_words(count);
        }
        true
    }
    fn finish(self) -> Arc<Vec<u32>> {
        // Vec ownership avoids a second full-size copy into an Arc slice.
        if self.same_previous {
            self.previous.unwrap()
        } else {
            Arc::new(self.words)
        }
    }
}
#[derive(Default)]
pub(in crate::vm) struct SourcePacking {
    requested: Option<Arc<Snapshot>>,
    ready: Option<(Arc<Snapshot>, Arc<Vec<u32>>)>,
    pending: Option<Pack>,
}
impl SourcePacking {
    fn request(&mut self, snapshot: Snapshot) {
        if self
            .requested
            .as_ref()
            .is_some_and(|old| old.same(&snapshot))
        {
            return;
        }
        self.requested = Some(Arc::new(snapshot));
        self.pending = None;
    }
    fn poll(
        &mut self,
        budget: &mut SourceUploadBudget,
    ) -> crate::SceneVMResult<Option<Arc<Vec<u32>>>> {
        let snapshot = self.requested.as_ref().unwrap();
        if let Some((old, ready)) = &self.ready
            && Arc::ptr_eq(old, snapshot)
        {
            return Ok(Some(ready.clone()));
        }
        if self.pending.is_none() {
            self.pending = Some(Pack::new(
                snapshot.clone(),
                self.ready.as_ref().map(|(_, words)| words.clone()),
            )?);
        }
        if !self.pending.as_mut().unwrap().advance(budget) {
            return Ok(None);
        }
        let words = self.pending.take().unwrap().finish();
        self.ready = Some((snapshot.clone(), words.clone()));
        Ok(Some(words))
    }
}
impl VM {
    pub(crate) fn cancel_organic_preparation(&mut self) {
        self.organic_packing.pending = None;
    }
    pub(super) fn poll_organic_words(
        &mut self,
        budget: &mut SourceUploadBudget,
        limit: u64,
    ) -> crate::SceneVMResult<Option<Words>> {
        if self.organic_sources_dirty || self.organic_packing.requested.is_none() {
            let snapshot = Snapshot::capture(self, limit)?;
            self.organic_packing.request(snapshot);
            self.organic_sources_dirty = false;
        }
        let Some(sources) = self.organic_packing.poll(budget)? else {
            return Ok(None);
        };
        let instances = if let Some(old) = &self.cached_organic_words
            && !self.organic_instances_dirty
        {
            old.instances.clone()
        } else {
            let next = self.build_organic_instance_words();
            match &self.cached_organic_words {
                Some(old) if old.instances.as_slice() == next.as_slice() => old.instances.clone(),
                _ => Arc::new(next),
            }
        };
        self.organic_instances_dirty = false;
        let words = Words { instances, sources };
        self.cached_organic_words = Some(words.clone());
        Ok(Some(words))
    }
}
