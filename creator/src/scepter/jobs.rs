//! Retained request results prevent duplicate edits and distinguish pending from failure.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Debug)]
enum State {
    Queued,
    Running,
    Complete(Value, Instant),
    Cancelled(Instant),
}
const STORAGE_LIMIT: usize = 64 * 1024 * 1024;
const RESULT_LIMIT: usize = 16 * 1024 * 1024;
#[derive(Debug)]
pub struct Job {
    storage: Arc<AtomicUsize>,
    retained: AtomicUsize,
    state: Mutex<State>,
    changed: Condvar,
}
impl Job {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_storage(Arc::new(AtomicUsize::new(0)), 0)
    }
    fn with_storage(storage: Arc<AtomicUsize>, payload_size: usize) -> Self {
        Self {
            storage,
            retained: AtomicUsize::new(payload_size),
            state: Mutex::new(State::Queued),
            changed: Condvar::new(),
        }
    }
    pub fn begin(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, State::Queued) {
            *state = State::Running;
            true
        } else {
            false
        }
    }
    pub fn finish(&self, mut result: Value) {
        struct Counter(usize);
        impl std::io::Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Counter(0);
        let _ = serde_json::to_writer(&mut count, &result);
        if count.0 > RESULT_LIMIT {
            result = json!({"ok":false,"http_status":413,"error":"Result exceeds 16 MiB; narrow the query or preview dimensions"});
        } else if self
            .storage
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(count.0).filter(|v| *v <= STORAGE_LIMIT)
            })
            .is_err()
        {
            result = json!({"ok":false,"http_status":503,"error":"Scepter result storage is full"});
        } else {
            self.retained.fetch_add(count.0, Ordering::Relaxed);
        }
        *self.state.lock().unwrap() = State::Complete(result, Instant::now());
        self.changed.notify_all();
    }
    pub fn cancel(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if matches!(*state, State::Queued) {
            *state = State::Cancelled(Instant::now());
            self.changed.notify_all();
            true
        } else {
            false
        }
    }
    pub fn response(&self, wait: Duration) -> Value {
        let state = self.state.lock().unwrap();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, wait, |s| matches!(s, State::Queued | State::Running))
            .unwrap();
        match &*state {
            State::Complete(v, _) => v.clone(),
            State::Cancelled(_) => {
                json!({"ok":false,"status":"cancelled","error":"Request cancelled before execution","http_status":409})
            }
            State::Queued => json!({"ok":true,"status":"queued","http_status":202}),
            State::Running => json!({"ok":true,"status":"running","http_status":202}),
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.storage
            .fetch_sub(self.retained.load(Ordering::Relaxed), Ordering::AcqRel);
    }
}
struct Entry {
    payload: String,
    job: Arc<Job>,
    persistent: bool,
}
#[derive(Default)]
pub(super) struct Registry {
    storage: Arc<AtomicUsize>,
    jobs: Mutex<HashMap<String, Entry>>,
}
impl Registry {
    // Never evict a mutation ID: clients may retry old edits. Restarting the
    // service starts a new project session, invalidating their edit guard.
    pub fn submit(
        &self,
        id: &str,
        payload: String,
        persistent: bool,
    ) -> Result<(Arc<Job>, bool), String> {
        let mut jobs = self.jobs.lock().unwrap();
        // Read results may be reclaimed after a minute. Mutation IDs and their
        // payloads survive for the service lifetime, so an expired retry cannot
        // accidentally reapply an edit.
        jobs.retain(|_, entry| {
            entry.persistent
                || match &*entry.job.state.lock().unwrap() {
                    State::Complete(_, at) | State::Cancelled(at) => {
                        at.elapsed() < Duration::from_secs(60)
                    }
                    _ => true,
                }
        });
        if let Some(entry) = jobs.get(id) {
            return if entry.payload == payload {
                Ok((entry.job.clone(), false))
            } else {
                Err("request_id already used for a different request".into())
            };
        }
        if jobs.len() >= 2048 {
            return Err("Request registry is full; restart Creator to begin a new session".into());
        }
        if self
            .storage
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(payload.len())
                    .filter(|v| *v <= STORAGE_LIMIT)
            })
            .is_err()
        {
            return Err("Scepter request storage is full".into());
        }
        let job = Arc::new(Job::with_storage(self.storage.clone(), payload.len()));
        jobs.insert(
            id.into(),
            Entry {
                payload,
                job: job.clone(),
                persistent,
            },
        );
        Ok((job, true))
    }
    pub fn get(&self, id: &str) -> Option<Arc<Job>> {
        self.jobs
            .lock()
            .unwrap()
            .get(id)
            .map(|entry| entry.job.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_reuse_results_and_cancellation_cannot_race_execution() {
        let r = Registry::default();
        let (a, new) = r.submit("a", "payload".into(), true).unwrap();
        assert!(new);
        let (b, new) = r.submit("a", "payload".into(), true).unwrap();
        assert!(!new);
        assert!(Arc::ptr_eq(&a, &b));
        assert!(r.submit("a", "different".into(), true).is_err());
        assert!(a.cancel());
        assert!(!b.begin());
        let (c, _) = r.submit("c", "payload".into(), true).unwrap();
        assert!(c.begin());
        assert!(!c.cancel());
        c.finish(json!({"ok":true,"count":1}));
        assert_eq!(r.get("c").unwrap().response(Duration::ZERO)["count"], 1);
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn result_budget_rejects_storage_growth() {
        let job = Job::with_storage(Arc::new(AtomicUsize::new(STORAGE_LIMIT)), 0);
        assert!(job.begin());
        job.finish(json!({"ok":true,"data":"result"}));
        assert_eq!(job.response(Duration::ZERO)["http_status"], 503);
    }
}
