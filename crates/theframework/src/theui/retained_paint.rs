//! Retain explicitly opted-in controls and static layout contributions.
//! Layout/input preparation still runs; dynamic views and receipt producers do not opt in.
use super::paint::UiImageResources;
use crate::{paint::PaintList, prelude::*};
use std::cell::RefCell;

const MAX_ENTRIES: usize = 512;
const MAX_OPS_PER_ENTRY: usize = 256;
const MAX_OPS: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FrameKey {
    pub theme: Uuid,
    pub fonts: (u64, u64),
    pub images: u64,
    pub density: [u32; 2],
    pub icons: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ControlKey {
    pub content: Uuid,
    pub dim: TheDim,
    pub state: TheWidgetState,
    pub interaction: [bool; 5],
}

struct Entry {
    key: Key,
    commands: PaintList,
    used: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Key {
    Control(ControlKey),
    Fragment {
        content: Uuid,
        dim: TheDim,
        variant: u64,
    },
}

/// Identity belongs to a layout contribution, independently of its children.
pub(crate) struct PaintFragment {
    pub id: Uuid,
    pub revision: Uuid,
}
impl Default for PaintFragment {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            revision: Uuid::new_v4(),
        }
    }
}
impl PaintFragment {
    pub fn invalidate(&mut self) {
        self.revision = Uuid::new_v4();
    }
}

/// Per-frame CPU command retention counters. These describe command reuse,
/// not GPU residency, successful presentation or measured speedup.
#[derive(Clone, Copy, Debug, Default)]
pub struct RetainedPaintStats {
    pub reused_controls: usize,
    pub recorded_controls: usize,
    pub resident_controls: usize,
    pub resident_ops: usize,
    pub reused_fragments: usize,
    pub recorded_fragments: usize,
    pub resident_fragments: usize,
}

#[derive(Default)]
pub(super) struct RetainedPaint {
    reused: usize,
    recorded: usize,
    reused_fragments: usize,
    recorded_fragments: usize,
    frame: Option<FrameKey>,
    active: bool,
    entries: FxHashMap<Uuid, Entry>,
    ops: usize,
}

impl RetainedPaint {
    pub fn begin(
        cache: &RefCell<Self>,
        frame: Option<FrameKey>,
        invalidate: bool,
    ) -> RetainedFrame<'_> {
        let mut state = cache.borrow_mut();
        if invalidate || state.frame != frame {
            state.entries.clear();
            state.ops = 0;
        }
        state.reused = 0;
        state.recorded = 0;
        state.reused_fragments = 0;
        state.recorded_fragments = 0;
        state.frame = frame;
        state.active = frame.is_some();
        for entry in state.entries.values_mut() {
            entry.used = false;
        }
        RetainedFrame(cache)
    }

    pub fn stats(&self) -> RetainedPaintStats {
        RetainedPaintStats {
            reused_controls: self.reused,
            recorded_controls: self.recorded,
            resident_controls: self
                .entries
                .values()
                .filter(|entry| matches!(entry.key, Key::Control(_)))
                .count(),
            resident_ops: self.ops,
            reused_fragments: self.reused_fragments,
            recorded_fragments: self.recorded_fragments,
            resident_fragments: self
                .entries
                .values()
                .filter(|entry| matches!(entry.key, Key::Fragment { .. }))
                .count(),
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn reuse(
        &mut self,
        id: Uuid,
        key: &Key,
        images: &dyn UiImageResources,
        output: &mut PaintList,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&id) else {
            return false;
        };
        if &entry.key != key || !images.retained_images_valid(&entry.commands) {
            return false;
        }
        output.ops.extend_from_slice(&entry.commands.ops);
        entry.used = true;
        match key {
            Key::Control(_) => self.reused += 1,
            Key::Fragment { .. } => self.reused_fragments += 1,
        }
        true
    }

    pub fn store(&mut self, id: Uuid, key: Key, commands: PaintList) {
        // Commands are independently balanced. Presentation receipts always require
        // the actual producer to run; never replay them from this static cache.
        if commands.ops.len() > MAX_OPS_PER_ENTRY
            || commands.receipts().next().is_some()
            || commands.validate().is_err()
        {
            return;
        }
        if let Some(old) = self.entries.remove(&id) {
            self.ops -= old.commands.ops.len();
        }
        if self.entries.len() >= MAX_ENTRIES || self.ops + commands.ops.len() > MAX_OPS {
            return;
        }
        match key {
            Key::Control(_) => self.recorded += 1,
            Key::Fragment { .. } => self.recorded_fragments += 1,
        }
        self.ops += commands.ops.len();
        self.entries.insert(
            id,
            Entry {
                key,
                commands,
                used: true,
            },
        );
    }
}

/// Close the cache frame on success and on early paint errors. Custom direct
/// WidgetPainter users therefore cannot accidentally reuse a previous UI frame.
pub(super) struct RetainedFrame<'a>(&'a RefCell<RetainedPaint>);
impl Drop for RetainedFrame<'_> {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = false;
        state.entries.retain(|_, entry| entry.used);
        state.ops = state
            .entries
            .values()
            .map(|entry| entry.commands.ops.len())
            .sum();
    }
}
