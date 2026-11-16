//! Opt-in input admission counters distinguish idle work from rejected brush
//! events. No project/geometry data is recorded; disabled in normal sessions.
use std::collections::BTreeMap;
use web_time::{Duration, Instant};

#[derive(Default)]
pub(super) struct InputProfile {
    started: Option<Instant>,
    outcomes: BTreeMap<&'static str, u64>,
}
impl InputProfile {
    pub fn mark(&mut self, paint: bool, outcome: &'static str) {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !paint || !*ENABLED.get_or_init(|| std::env::var_os("ELDIRON_PROFILE_EDITOR").is_some()) {
            return;
        }
        let now = Instant::now();
        let started = self.started.get_or_insert(now);
        *self.outcomes.entry(outcome).or_default() += 1;
        if now.duration_since(*started) >= Duration::from_secs(1) {
            eprintln!("Paint input {:?}", self.outcomes);
            self.outcomes.clear();
            *started = now;
        }
    }
}
