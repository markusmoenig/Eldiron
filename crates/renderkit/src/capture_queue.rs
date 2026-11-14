//! Aggregate staging limits for explicit image/depth captures in one device
//! domain. Submitted work pins a reservation even after its consumer cancels.
use crate::{RenderError, Result};
use std::sync::{Arc, Mutex};

// Match the explicitly requested device's default max_buffer_size, so one
// otherwise valid large export remains admissible without chunking its image.
const BYTE_LIMIT: u64 = 256 * 1024 * 1024;
const REQUEST_LIMIT: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureQueueUsage {
    pub bytes: u64,
    pub requests: u32,
    pub byte_limit: u64,
    pub request_limit: u32,
}
pub(crate) struct CaptureQueue(Mutex<CaptureQueueUsage>);
impl Default for CaptureQueue {
    fn default() -> Self {
        Self(Mutex::new(CaptureQueueUsage {
            bytes: 0,
            requests: 0,
            byte_limit: BYTE_LIMIT,
            request_limit: REQUEST_LIMIT,
        }))
    }
}
impl CaptureQueue {
    pub(crate) fn usage(&self) -> CaptureQueueUsage {
        *self.0.lock().unwrap()
    }
    pub(crate) fn reserve(self: &Arc<Self>, bytes: u64) -> Result<CaptureLease> {
        let mut usage = self.0.lock().unwrap();
        if bytes == 0
            || bytes > usage.byte_limit.saturating_sub(usage.bytes)
            || usage.requests >= usage.request_limit
        {
            return Err(RenderError("capture queue staging budget exhausted".into()));
        }
        usage.bytes += bytes;
        usage.requests += 1;
        Ok(CaptureLease(Arc::new(Reservation {
            queue: self.clone(),
            bytes,
        })))
    }
}
struct Reservation {
    queue: Arc<CaptureQueue>,
    bytes: u64,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut usage = self.queue.0.lock().unwrap();
        usage.bytes -= self.bytes;
        usage.requests -= 1;
    }
}
/// Reserve before creating staging storage. Hold through consumption or
/// cancellation, and call `retain_for_submission` immediately after submitting
/// the encoder that uses it. This accounts for padded allocation bytes, not
/// just the logical color/depth payload in the transfer counters.
pub struct CaptureLease(Arc<Reservation>);
impl CaptureLease {
    /// GPU completion and consumer retirement must both precede reuse of the
    /// reservation. Dropping a ticket alone cannot bound queued GPU allocations.
    pub fn retain_for_submission(&self, queue: &wgpu::Queue) {
        queue.on_submitted_work_done(self.completion());
    }
    fn completion(&self) -> impl FnOnce() + Send + 'static {
        let reservation = self.0.clone();
        move || drop(reservation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_and_gpu_completion_both_gate_capacity_reuse() {
        let queue = Arc::new(CaptureQueue::default());
        let lease = queue.reserve(BYTE_LIMIT).unwrap();
        let done = lease.completion();
        assert!(queue.reserve(1).is_err());
        drop(lease);
        assert_eq!(queue.usage().bytes, BYTE_LIMIT);
        assert!(queue.reserve(1).is_err());
        done();
        assert_eq!(queue.usage().requests, 0);
        let retained = queue.reserve(BYTE_LIMIT).unwrap();
        retained.completion()();
        assert_eq!(queue.usage().bytes, BYTE_LIMIT);
        drop(retained);
        assert_eq!(queue.usage().bytes, 0);
    }
    #[test]
    fn requests_and_bytes_are_shared_bounded_and_rejection_is_atomic() {
        let queue = Arc::new(CaptureQueue::default());
        assert!(queue.reserve(0).is_err());
        assert!(queue.reserve(u64::MAX).is_err());
        let leases: Vec<_> = (0..REQUEST_LIMIT)
            .map(|_| queue.reserve(1).unwrap())
            .collect();
        let full = queue.usage();
        assert!(queue.clone().reserve(1).is_err());
        assert_eq!(queue.usage(), full);
        drop(leases);
        assert_eq!(queue.usage().bytes, 0);
        assert_eq!(queue.usage().requests, 0);
    }
}
