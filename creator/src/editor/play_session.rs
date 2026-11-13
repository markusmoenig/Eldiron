//! Creator's runtime projection, separate from the authored Documents owner.
//! The facade still lends server/assets; only this projection receives simulation
//! writes. Presentation borrows it and never drains server channels itself.
use eldiron_runtime::{EventInterests, PlayerInput, SessionClock, SessionPump, SessionUpdate};
use theframework_core::Uuid;
use web_time::{Duration, Instant};

pub(super) struct CreatorPlaySession {
    pub document: Uuid,
    pub project: shared::project::Project,
    pub pump: SessionPump,
    pub input: PlayerInput,
    pub requested: bool,
    clock: SessionClock,
    last: Instant,
}
impl CreatorPlaySession {
    pub fn new(
        document: Uuid,
        project: shared::project::Project,
        tick_ms: u64,
        controls: rusterix::input_action::ClientAction,
        requested: bool,
    ) -> Self {
        Self {
            document,
            project,
            input: PlayerInput::from_controls(controls),
            requested,
            pump: SessionPump::new(EventInterests::default()),
            clock: SessionClock::new(Duration::from_millis(tick_ms.max(1))),
            last: Instant::now(),
        }
    }
    pub fn text_runtime<'a>(
        play: Option<&'a mut Self>,
        authored: &'a shared::project::Project,
        bridge: &'a mut rusterix::Rusterix,
    ) -> (
        &'a shared::project::Project,
        crate::textplay::TextGameRuntime<'a>,
    ) {
        let (project, input) = if let Some(play) = play {
            for region in &mut play.project.regions {
                bridge.server.apply_entities_items(&mut region.map);
                if region.map.name == bridge.client.current_map
                    && let Some(time) = bridge.server.get_time(&region.map.id)
                {
                    bridge.client.set_server_time(time);
                    play.project.time = time;
                }
            }
            // Text command definitions remain live authoring inputs, while room
            // state and entities come exclusively from the runtime projection.
            play.project.authoring.clone_from(&authored.authoring);
            play.project.config.clone_from(&authored.config);
            (&play.project, Some((&mut play.input, play.requested)))
        } else {
            (authored, None)
        };
        (
            project,
            crate::textplay::TextGameRuntime {
                server: &mut bridge.server,
                assets: &bridge.assets,
                time: bridge.client.server_time,
                input,
            },
        )
    }
    pub fn suspend_clock(&mut self) {
        self.clock.reset_elapsed();
        self.last = Instant::now();
    }
    pub fn advance(&mut self, runtime: &mut rusterix::Rusterix) -> SessionUpdate {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let update = self.pump.advance(
            &mut self.clock,
            elapsed,
            &mut self.project,
            &mut runtime.server,
            &mut runtime.assets,
            &mut runtime.client.current_map,
        );
        if let Some(time) = update.time {
            self.project.time = time;
        }
        runtime.prepare_runtime_appearances();
        self.synchronize_controls(runtime);
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
    fn synchronize_controls(&mut self, runtime: &mut rusterix::Rusterix) {
        let isometric=self.project.regions.iter().find(|r|r.map.name==runtime.client.current_map)
            .and_then(|r|r.map.entities.iter().find(|e|e.is_player()))
            .is_some_and(|e|matches!(e.attributes.get("player_camera"),Some(rusterix::Value::PlayerCamera(camera)) if camera.is_isometric_2d()));
        if let Some(action) = self.input.controls.set_isometric_controls(isometric) {
            let _ = self.submit_action(&mut runtime.server, action);
        }
    }
    pub fn flush_command(&mut self, runtime: &mut rusterix::Rusterix) {
        let update = self.pump.update(
            1,
            true,
            &mut self.project,
            &mut runtime.server,
            &mut runtime.assets,
            &mut runtime.client.current_map,
        );
        if let Some(time) = update.time {
            self.project.time = time;
        }
        runtime.prepare_runtime_appearances();
    }

    /// The authoring viewport keeps current geometry, selection, placement and
    /// camera edits. Borrow transient actors and prop state without replacing
    /// that geometry with a server-generated dungeon or writing into its map.
    pub fn authoring_map<'a>(
        &self,
        authored: &'a rusterix::Map,
    ) -> std::borrow::Cow<'a, rusterix::Map> {
        let Some(region) = self
            .project
            .regions
            .iter()
            .find(|r| r.map.id == authored.id)
        else {
            return std::borrow::Cow::Borrowed(authored);
        };
        let mut map = authored.clone();
        map.entities.clone_from(&region.map.entities);
        map.items.clone_from(&region.map.items);
        for prop in &mut map.block_prop_instances {
            if let Some(runtime) = region
                .map
                .block_prop_instances
                .iter()
                .find(|p| p.id == prop.id)
            {
                prop.runtime_state.clone_from(&runtime.runtime_state);
            }
        }
        std::borrow::Cow::Owned(map)
    }
}
