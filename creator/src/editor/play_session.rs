//! Temporary CPU facade-storage adapter to the same driver as GameSession.
//! The driver owns clock/output incarnation; renderer and UI never drain it.
use eldiron_runtime::{EventInterests, PlayerInput, SessionClock, SessionPump, SessionUpdate};
use theframework_core::Uuid;
use web_time::{Duration, Instant};

pub(super) struct CreatorPlaySession {
    pub document: Uuid,
    pub pump: SessionPump,
    pub input: PlayerInput,
    pub requested: bool,
    clock: SessionClock,
    last: Instant,
}
impl CreatorPlaySession {
    pub fn new(
        document: Uuid,
        tick_ms: u64,
        controls: rusterix::input_action::ClientAction,
        requested: bool,
    ) -> Self {
        Self {
            document,
            input: PlayerInput::from_controls(controls),
            requested,
            pump: SessionPump::new(EventInterests::default()),
            clock: SessionClock::new(Duration::from_millis(tick_ms.max(1))),
            last: Instant::now(),
        }
    }
    /// Transitional projection for the text command host. Keep source writes
    /// at the CPU owner boundary; TextGameState receives an immutable document.
    pub fn refresh_text_runtime_view(
        project: &mut shared::project::Project,
        bridge: &mut rusterix::Rusterix,
    ) {
        for region in &mut project.regions {
            bridge.server.apply_entities_items(&mut region.map);
            if let Some(time) = bridge.server.get_time(&region.map.id) {
                bridge.client.set_server_time(time);
                project.time = time;
            }
        }
    }
    pub fn text_runtime<'a>(
        play: Option<&'a mut Self>,
        bridge: &'a mut rusterix::Rusterix,
    ) -> crate::textplay::TextGameRuntime<'a> {
        crate::textplay::TextGameRuntime {
            server: &mut bridge.server,
            assets: &bridge.assets,
            time: bridge.client.server_time,
            input: play.map(|play| (&mut play.input, play.requested)),
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
        self.synchronize_controls(project, runtime);
        if runtime.server.state == rusterix::ServerState::Running {
            self.input.flush(&mut runtime.server);
        }
        update
    }
    pub fn submit_action(
        &mut self,
        server: &mut rusterix::Server,
        action: rusterix::EntityAction,
    ) -> Result<(), String> {
        if server.state != rusterix::ServerState::Running && action != rusterix::EntityAction::Off {
            return Ok(());
        }
        if let Err(error) = self.input.submit_action(server, self.requested, action) {
            self.input.controls.cancel_controls();
            // Stop has reserved headroom even when registration is delayed and
            // discrete commands have filled the bounded queue.
            let _ = self
                .input
                .submit_action(server, self.requested, rusterix::EntityAction::Off);
            return Err(error);
        }
        Ok(())
    }
    pub fn cancel_input(&mut self, server: &mut rusterix::Server) {
        let stop = self.input.controls.cancel_controls();
        let _ = self.submit_action(server, stop);
    }
    fn synchronize_controls(
        &mut self,
        project: &shared::project::Project,
        runtime: &mut rusterix::Rusterix,
    ) {
        let isometric=project.regions.iter().find(|r|r.map.name==runtime.client.current_map)
            .and_then(|r|r.map.entities.iter().find(|e|e.is_player()))
            .is_some_and(|e|matches!(e.attributes.get("player_camera"),Some(rusterix::Value::PlayerCamera(camera)) if camera.is_isometric_2d()));
        if let Some(action) = self.input.controls.set_isometric_controls(isometric) {
            let _ = self.submit_action(&mut runtime.server, action);
        }
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
