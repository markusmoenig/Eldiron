//! CPU-only wake notifications for a device domain. Listeners do not retain its
//! device or any event loop; dropping their lease unregisters the callback.
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};
struct Observer {
    wake: Box<dyn Fn() + Send + Sync>,
}
/// Keep this lease while the host wants device-loss/async-validation wakeups.
/// The callback should schedule host work; inspect is_lost/poll on that host.
pub struct GpuFaultListener {
    _observer: Arc<Observer>,
}
#[derive(Default)]
pub(crate) struct Notifications {
    faulted: AtomicBool,
    observers: Mutex<Vec<Weak<Observer>>>,
}
impl Notifications {
    pub(crate) fn observe(&self, wake: impl Fn() + Send + Sync + 'static) -> GpuFaultListener {
        let observer = Arc::new(Observer {
            wake: Box::new(wake),
        });
        let ready = {
            let mut observers = self.observers.lock().unwrap();
            observers.retain(|observer| observer.strong_count() != 0);
            if self.faulted.load(Ordering::Acquire) {
                true
            } else {
                observers.push(Arc::downgrade(&observer));
                false
            }
        };
        if ready {
            (observer.wake)();
        }
        GpuFaultListener {
            _observer: observer,
        }
    }
    pub(crate) fn signal(&self) {
        if self.faulted.swap(true, Ordering::AcqRel) {
            return;
        }
        let observers = std::mem::take(&mut *self.observers.lock().unwrap());
        // Never invoke host code while retaining the notification lock.
        for observer in observers
            .into_iter()
            .filter_map(|observer| observer.upgrade())
        {
            (observer.wake)();
        }
    }
}
