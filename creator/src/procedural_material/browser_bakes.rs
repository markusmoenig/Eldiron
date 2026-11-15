//! Cooperative browser asset jobs. The browser thread owns its GPU objects;
//! callbacks publish only addressed immutable CPU assets to the document owner.
pub(super) use super::bake_types::*;
#[cfg(feature = "native-render")]
mod worker {
    use super::*;
    use std::{
        cell::RefCell,
        collections::VecDeque,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
    };
    const MAX_PENDING: usize = 64;
    #[derive(Default)]
    struct Inbox {
        jobs: VecDeque<Job>,
        wake: Option<futures::channel::oneshot::Sender<()>>,
        closed: bool,
    }
    type Space = Rc<RefCell<Option<futures::channel::oneshot::Sender<()>>>>;
    pub(in crate::procedural_material) struct Results {
        receiver: mpsc::Receiver<Completed>,
        space: Space,
    }
    impl Results {
        pub fn try_iter(&self) -> mpsc::TryIter<'_, Completed> {
            if let Some(wake) = self.space.borrow_mut().take() {
                let _ = wake.send(());
            }
            self.receiver.try_iter()
        }
    }
    pub(in crate::procedural_material) struct Worker {
        inbox: Rc<RefCell<Inbox>>,
        canceled: Arc<AtomicBool>,
        abort: futures::future::AbortHandle,
        pub results: Results,
    }
    impl Worker {
        pub fn new_asset_bakes_with_wake(
            context: Arc<renderkit::GpuContext>,
            wake: Option<theframework::platform::ApplicationWake>,
        ) -> Self {
            let inbox = Rc::new(RefCell::new(Inbox::default()));
            let queue = inbox.clone();
            let canceled = Arc::new(AtomicBool::new(false));
            let cancel = canceled.clone();
            let (send, receiver) = mpsc::sync_channel(2);
            let space = Space::default();
            let available = space.clone();
            let results = Results { receiver, space };
            let (abort, registration) = futures::future::AbortHandle::new_pair();
            let run = async move {
                let mut baker = super::super::asset_bake::Baker::new(context, cancel);
                loop {
                    let (job, wait) = {
                        let mut queue = queue.borrow_mut();
                        if queue.closed {
                            break;
                        }
                        if let Some(job) = queue.jobs.pop_front() {
                            (Some(job), None)
                        } else {
                            let (done, ready) = futures::channel::oneshot::channel();
                            queue.wake = Some(done);
                            (None, Some(ready))
                        }
                    };
                    let Some(job) = job else {
                        if wait.unwrap().await.is_err() {
                            break;
                        }
                        continue;
                    };
                    let frame = baker.bake(&job).await;
                    if queue.borrow().closed {
                        break;
                    }
                    let mut completed = Completed {
                        pixels_per_tile: job.pixels_per_tile,
                        tile: job.tile,
                        bundle: job.bundle,
                        revision: job.revision,
                        frame,
                    };
                    loop {
                        match send.try_send(completed) {
                            Ok(()) => {
                                if let Some(wake) = &wake {
                                    wake.request_update();
                                }
                                break;
                            }
                            Err(mpsc::TrySendError::Disconnected(_)) => return,
                            Err(mpsc::TrySendError::Full(result)) => {
                                completed = result;
                                let (done, ready) = futures::channel::oneshot::channel();
                                *available.borrow_mut() = Some(done);
                                if ready.await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(async move {
                let _ = futures::future::Abortable::new(run, registration).await;
            });
            Self {
                inbox,
                canceled,
                abort,
                results,
            }
        }
        pub fn send(&self, job: Job) -> bool {
            let mut queue = self.inbox.borrow_mut();
            if queue.closed {
                return false;
            }
            if let Some(existing) = queue
                .jobs
                .iter_mut()
                .find(|existing| existing.tile == job.tile)
            {
                *existing = job;
            } else if queue.jobs.len() < MAX_PENDING {
                queue.jobs.push_back(job);
            } else {
                return false;
            }
            if let Some(wake) = queue.wake.take() {
                let _ = wake.send(());
            }
            true
        }
    }
    impl Drop for Worker {
        fn drop(&mut self) {
            self.canceled.store(true, Ordering::Release);
            self.abort.abort();
            let mut queue = self.inbox.borrow_mut();
            queue.closed = true;
            queue.jobs.clear();
            if let Some(wake) = queue.wake.take() {
                let _ = wake.send(());
            }
        }
    }
}
#[cfg(feature = "native-render")]
pub(super) use worker::Worker;
