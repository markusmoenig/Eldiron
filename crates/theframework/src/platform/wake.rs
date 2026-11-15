//! Application updates, independent of GPU and presentation frames. Native
//! notifications are thread-safe; browser notifications stay on their event loop.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[cfg(not(target_arch = "wasm32"))]
type Notify = Box<dyn Fn() + Send + Sync>;
#[cfg(target_arch = "wasm32")]
type Notify = Box<dyn Fn()>;

struct State {
    pending: AtomicBool,
    notify: Option<Notify>,
}

/// Clones address one application host. Requests coalesce until the host begins
/// an update; requesting an update does not itself request a rendered frame.
#[derive(Clone)]
pub struct ApplicationWake(Arc<State>);

impl Default for ApplicationWake {
    fn default() -> Self {
        Self(Arc::new(State {
            pending: AtomicBool::new(false),
            notify: None,
        }))
    }
}

impl std::fmt::Debug for ApplicationWake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicationWake").finish_non_exhaustive()
    }
}

impl ApplicationWake {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_notify(notify: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(State {
            pending: AtomicBool::new(false),
            notify: Some(Box::new(notify)),
        }))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_notify(notify: impl Fn() + 'static) -> Self {
        Self(Arc::new(State {
            pending: AtomicBool::new(false),
            notify: Some(Box::new(notify)),
        }))
    }

    pub fn request_update(&self) {
        if !self.0.pending.swap(true, Ordering::AcqRel)
            && let Some(notify) = &self.0.notify
        {
            notify();
        }
    }

    pub(crate) fn take_pending(&self) -> bool {
        self.0.pending.swap(false, Ordering::AcqRel)
    }
}

/// The message is queued before waking its owner. Wrapping an existing sender
/// preserves its queue/error semantics and the producer's cancellation policy.
pub struct WakeSender<T> {
    sender: mpsc::Sender<T>,
    wake: Option<ApplicationWake>,
}

impl<T> Clone for WakeSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wake: self.wake.clone(),
        }
    }
}

impl<T> std::fmt::Debug for WakeSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WakeSender").finish_non_exhaustive()
    }
}

impl<T> From<mpsc::Sender<T>> for WakeSender<T> {
    fn from(sender: mpsc::Sender<T>) -> Self {
        Self::new(sender, None)
    }
}

impl<T> WakeSender<T> {
    pub fn with_wake(mut self, wake: ApplicationWake) -> Self {
        self.wake = Some(wake);
        self
    }

    pub fn new(sender: mpsc::Sender<T>, wake: Option<ApplicationWake>) -> Self {
        Self { sender, wake }
    }

    pub fn send(&self, message: T) -> Result<(), mpsc::SendError<T>> {
        self.sender.send(message)?;
        if let Some(wake) = &self.wake {
            wake.request_update();
        }
        Ok(())
    }
}
