//! Admission for optional derived caches, shared by a device domain. Account
//! metadata together with GPU storage; this is a reservation, not an additional
//! physical allocation counter. Required producer buffers have separate owners.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetainedCacheUsage {
    pub bytes: u64,
    pub peak_bytes: u64,
    pub reservations: u64,
    pub byte_limit: u64,
    pub requests: u64,
    pub rejected_requests: u64,
}
pub(crate) struct RetainedCaches(Mutex<RetainedCacheUsage>);
impl Default for RetainedCaches {
    fn default() -> Self {
        Self(Mutex::new(RetainedCacheUsage {
            byte_limit: 64 * 1024 * 1024,
            ..Default::default()
        }))
    }
}
impl RetainedCaches {
    pub fn usage(&self) -> RetainedCacheUsage {
        *self.0.lock().unwrap()
    }
    pub fn set_limit(&self, bytes: u64) {
        self.0.lock().unwrap().byte_limit = bytes;
    }
    pub fn reserve(self: &Arc<Self>, bytes: u64) -> Option<RetainedCacheLease> {
        let mut usage = self.0.lock().unwrap();
        usage.requests = usage.requests.saturating_add(1);
        let total = usage.bytes.checked_add(bytes);
        let Some(total) = total.filter(|total| bytes != 0 && *total <= usage.byte_limit) else {
            usage.rejected_requests = usage.rejected_requests.saturating_add(1);
            return None;
        };
        usage.bytes = total;
        usage.peak_bytes = usage.peak_bytes.max(total);
        usage.reservations += 1;
        Some(RetainedCacheLease(Arc::new(Charge {
            owner: self.clone(),
            bytes,
        })))
    }
}
struct Charge {
    owner: Arc<RetainedCaches>,
    bytes: u64,
}
impl Drop for Charge {
    fn drop(&mut self) {
        let mut usage = self.owner.0.lock().unwrap();
        usage.bytes -= self.bytes;
        usage.reservations -= 1;
    }
}
/// Clones pin one reservation. Retired caches retain it through submitted GPU
/// work, even after their application owner is removed or the limit is lowered.
#[derive(Clone)]
pub struct RetainedCacheLease(Arc<Charge>);
impl RetainedCacheLease {
    pub fn bytes(&self) -> u64 {
        self.0.bytes
    }
    pub fn retain_for_submission(&self, queue: &wgpu::Queue) {
        let lease = self.clone();
        queue.on_submitted_work_done(move || drop(lease));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_cache_admission_counts_cloned_and_retired_owners_once() {
        let budget = Arc::new(RetainedCaches::default());
        budget.set_limit(64);
        let first = budget.reserve(40).unwrap();
        let queued = first.clone();
        assert!(budget.reserve(25).is_none());
        drop(first);
        assert_eq!(budget.usage().bytes, 40);
        let second = budget.reserve(24).unwrap();
        assert_eq!(budget.usage().reservations, 2);
        budget.set_limit(16);
        assert!(budget.reserve(1).is_none());
        drop(second);
        assert_eq!(budget.usage().bytes, 40);
        drop(queued);
        assert_eq!(budget.usage().bytes, 0);
        assert_eq!(budget.usage().reservations, 0);
        assert_eq!(budget.usage().peak_bytes, 64);
        assert!(budget.reserve(16).is_some());
        assert!(budget.reserve(0).is_none());
    }
}
