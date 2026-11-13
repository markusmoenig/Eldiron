//! Temporary CPU facade-storage adapter to the same driver as GameSession.
//! The driver owns clock/output incarnation; renderer and UI never drain it.
use eldiron_runtime::{EventInterests, SessionClock, SessionPump, SessionUpdate};
use theframework_core::Uuid;
use web_time::{Duration, Instant};

pub(super) struct CreatorPlaySession {
    pub document: Uuid,
    pub pump: SessionPump,
    clock: SessionClock,
    last: Instant,
}
impl CreatorPlaySession {
    pub fn new(document: Uuid, tick_ms: u64) -> Self {
        Self {
            document,
            pump: SessionPump::new(EventInterests::default()),
            clock: SessionClock::new(Duration::from_millis(tick_ms.max(1))),
            last: Instant::now(),
        }
    }
    pub fn suspend_clock(&mut self) {
        self.clock.reset_elapsed();
        self.last = Instant::now();
    }
    pub fn advance(
        &mut self,
        project: &mut shared::project::Project,
        runtime: &mut rusterix::Rusterix,
    ) -> SessionUpdate {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let update = self.pump.advance(
            &mut self.clock,
            elapsed,
            project,
            &mut runtime.server,
            &mut runtime.assets,
            &mut runtime.client.current_map,
        );
        runtime.prepare_runtime_appearances();
        update
    }
    pub fn flush_command(
        &mut self,
        project: &mut shared::project::Project,
        runtime: &mut rusterix::Rusterix,
    ) {
        self.pump.update(
            1,
            true,
            project,
            &mut runtime.server,
            &mut runtime.assets,
            &mut runtime.client.current_map,
        );
        runtime.prepare_runtime_appearances();
    }
}
