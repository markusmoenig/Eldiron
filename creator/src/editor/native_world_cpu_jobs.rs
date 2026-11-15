//! Bounded native CPU source workers shared by UV and stamp owners.
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(target_arch = "wasm32"))]
pub(super) struct Slot;
#[cfg(not(target_arch = "wasm32"))]
impl Slot {
    pub(super) fn acquire() -> Option<Self> {
        ACTIVE
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 2).then_some(n + 1)
            })
            .ok()
            .map(|_| Self)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
    }
}
