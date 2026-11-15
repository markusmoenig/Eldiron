//! Serialize placement metadata in bounded batches from the owner's frozen pose.
//! Pixel backing and the shader packet layout are independent of this cursor.
use super::{packing::error, *};
use crate::source_upload::SourceUploadBudget;

pub(super) struct Layout {
    pub chunks: Vec<(Uuid, u32)>,
    pub chunk_instances: usize,
    pub paint_sprite_base: u32,
    pub sprite_count: u32,
}

pub(super) struct Pack {
    layout: Arc<Layout>,
    words: Vec<u32>,
    group: usize,
    cursor: usize,
    previous: Option<Arc<Vec<u32>>>,
    same_previous: bool,
}
impl Pack {
    pub fn new(
        vm: &VM,
        layout: Arc<Layout>,
        previous: Option<Arc<Vec<u32>>>,
        limit: u64,
    ) -> crate::SceneVMResult<Self> {
        let count = vm
            .organic_billboards
            .instances
            .len()
            .checked_add(layout.chunk_instances)
            .and_then(|count| count.checked_add(vm.paint_billboards.instances.len()))
            .and_then(|count| u32::try_from(count).ok())
            .ok_or_else(|| error("organic instance count overflow"))?;
        let meta = count
            .checked_mul(8)
            .and_then(|n| n.checked_add(8))
            .ok_or_else(|| error("organic instance metadata overflow"))?;
        let pixels = layout
            .sprite_count
            .checked_mul(4)
            .and_then(|n| n.checked_add(meta))
            .ok_or_else(|| error("organic pixel offset overflow"))?;
        if u64::from(meta) * 4 > limit {
            return Err(error("organic instances exceed their buffer budget"));
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(meta as usize)
            .map_err(|_| error("cannot reserve organic instance words"))?;
        words.extend_from_slice(&[
            0x4F42_494C,
            layout.sprite_count,
            count,
            8,
            meta,
            pixels,
            0,
            0,
        ]);
        let same_previous = previous
            .as_ref()
            .is_some_and(|old| old.len() == meta as usize && old[..8] == words[..]);
        Ok(Self {
            layout,
            words,
            group: 0,
            cursor: 0,
            previous,
            same_previous,
        })
    }
    pub fn advance(
        &mut self,
        vm: &VM,
        budget: &mut SourceUploadBudget,
    ) -> crate::SceneVMResult<bool> {
        while self.group < self.layout.chunks.len() + 2 {
            let (instances, base) = if self.group == 0 {
                (&vm.organic_billboards.instances, 0)
            } else if self.group == self.layout.chunks.len() + 1 {
                (
                    &vm.paint_billboards.instances,
                    self.layout.paint_sprite_base,
                )
            } else {
                let (id, base) = self.layout.chunks[self.group - 1];
                let chunk = vm
                    .chunks_map
                    .get(&id)
                    .ok_or_else(|| error("organic chunk changed during preparation"))?;
                (&chunk.organic_billboard_instances, base)
            };
            if self.cursor == instances.len() {
                if budget.available_cpu_words() == 0 {
                    return Ok(false);
                }
                self.group += 1;
                self.cursor = 0;
                budget.record_cpu_step();
                continue;
            }
            let count = (budget.available_cpu_words() / 8)
                .min(256)
                .min(instances.len().saturating_sub(self.cursor));
            if count == 0 {
                return Ok(false);
            }
            let start = self.words.len();
            for instance in &instances[self.cursor..self.cursor + count] {
                self.words.extend_from_slice(&[
                    instance.center[0].to_bits(),
                    instance.center[1].to_bits(),
                    instance.center[2].to_bits(),
                    instance.width.to_bits(),
                    instance.height.to_bits(),
                    base.saturating_add(instance.sprite_index),
                    instance.flags,
                    0,
                ]);
            }
            if self.same_previous {
                self.same_previous =
                    self.previous.as_ref().unwrap()[start..self.words.len()] == self.words[start..];
            }
            self.cursor += count;
            budget.record_cpu_words(count * 8);
        }
        if self.words.len() != self.words[4] as usize {
            return Err(error("organic instance count changed during preparation"));
        }
        Ok(true)
    }
    pub fn finish(self) -> Arc<Vec<u32>> {
        if self.same_previous {
            self.previous.unwrap()
        } else {
            Arc::new(self.words)
        }
    }
}
