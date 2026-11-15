//! CPU-only resumable stamp jobs. These futures are deliberately polled by
//! their owner: Pending means the current slice expired, not an external wait.
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use web_time::{Duration, Instant};

/// One owned computation, without document, VM or renderer borrows. Dropping it
/// cancels at the last checkpoint. Native owners may move it into their worker.
pub struct StampWork<T> {
    future: Pin<Box<dyn Future<Output = Result<T, String>> + Send>>,
    complete: bool,
}
impl<T> StampWork<T> {
    pub(super) fn new(future: impl Future<Output = Result<T, String>> + Send + 'static) -> Self {
        Self {
            future: Box::pin(future),
            complete: false,
        }
    }

    /// Advance at most one CPU slice. The owner schedules the next call after
    /// None; no executor, thread, callback or GPU resource is created here.
    pub fn advance(&mut self) -> Option<Result<T, String>> {
        if self.complete {
            return Some(Err("Stamp work was already completed".into()));
        }
        match self
            .future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Pending => None,
            Poll::Ready(result) => {
                self.complete = true;
                Some(result)
            }
        }
    }
}

pub(super) struct Slice {
    enabled: bool,
    operations: usize,
    started: Instant,
}
impl Slice {
    pub fn cooperative() -> Self {
        Self {
            enabled: true,
            operations: 0,
            started: Instant::now(),
        }
    }
    pub fn unlimited() -> Self {
        Self {
            enabled: false,
            ..Self::cooperative()
        }
    }
    pub async fn checkpoint(&mut self) {
        if !self.enabled {
            return;
        }
        self.operations += 1;
        // Check before rejected/transparent pixels too. Neither original
        // canvas area nor a long Bresenham line can bypass the slice boundary.
        if self.operations % 64 == 0
            && (self.operations >= 8192 || self.started.elapsed() >= Duration::from_millis(2))
        {
            let mut yielded = false;
            std::future::poll_fn(|_| {
                if yielded {
                    self.operations = 0;
                    self.started = Instant::now();
                    Poll::Ready(())
                } else {
                    yielded = true;
                    Poll::Pending
                }
            })
            .await;
        }
    }
}

/// Explicit synchronous/reference utilities consume the same kernel to its end.
/// Interactive owners use StampWork instead, preserving a single drawing order.
pub(super) fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(result) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            return result;
        }
    }
}
