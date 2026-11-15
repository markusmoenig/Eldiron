//! Browser-local GPU work is owned by the view requesting its result.
//! Dropping the mailbox cancels the future as well as rejecting its result.
use futures::future::{AbortHandle, Abortable};
use std::{future::Future, sync::mpsc};

pub(crate) struct Task<T> {
    receive: mpsc::Receiver<T>,
    abort: AbortHandle,
}
impl<T: 'static> Task<T> {
    pub fn spawn(future: impl Future<Output = T> + 'static) -> Self {
        let (done, receive) = mpsc::channel();
        let (abort, registration) = AbortHandle::new_pair();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(result) = Abortable::new(future, registration).await {
                let _ = done.send(result);
            }
        });
        Self { receive, abort }
    }
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.receive.try_recv()
    }
}
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        // Wake the local executor even when an idle host will not poll again.
        // GPU scopes are popped before their asynchronous completion is awaited.
        self.abort.abort();
    }
}
