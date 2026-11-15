//! Pool accounting follows allocation owners even after cache eviction.
//! Raw GPU handles retained by the driver/queued commands are outside this count.
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub(crate) struct AllocationUsage {
    bytes: AtomicU64,
    count: AtomicU64,
    peak: AtomicU64,
}
impl AllocationUsage {
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.bytes.load(Ordering::Relaxed),
            self.count.load(Ordering::Relaxed),
            self.peak.load(Ordering::Relaxed),
        )
    }
    pub fn charge(self: &Arc<Self>, bytes: u64) -> AllocationCharge {
        let live = self.bytes.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.peak.fetch_max(live, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        AllocationCharge {
            usage: self.clone(),
            bytes,
        }
    }
}
pub(crate) struct AllocationCharge {
    usage: Arc<AllocationUsage>,
    bytes: u64,
}
impl Drop for AllocationCharge {
    fn drop(&mut self) {
        self.usage.bytes.fetch_sub(self.bytes, Ordering::Relaxed);
        self.usage.count.fetch_sub(1, Ordering::Relaxed);
    }
}
