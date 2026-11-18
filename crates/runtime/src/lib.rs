//! One CPU local session, shared by terminal, graphical players and Creator Play.
//! There is no surface, UI context, font shaping, GPU device or render callback.
mod clock;
mod events;
mod frame_input;
mod movement;
pub mod diagnostics;
pub mod persistence;
pub use rusterix::server::region::requests;
pub mod prediction;
pub use frame_input::FrameInputError;
pub use prediction::{MotionClient, MotionSettings};
pub use rusterix::movement::MovementCommand;
mod player;
pub use movement::{MovementInput, MovementInputError, MovementIntent, MovementSubmission};
mod pump;
pub mod replication;
pub use player::{
    PlayerControls, PlayerInput, PlayerInputStats, PlayerSession, PlayerSettings, PlayerTransition,
    is_movement_state,
};
pub use replication::{ClientFrame, ClientWorld, ReplicationError, ReplicationPolicy};
mod view;
pub use clock::SessionClock;
pub use events::{EventInterests, RegionEvents, RegionOutputs};
pub use pump::SessionPump;
pub use rusterix::player_start::{PlayerStartInfo, PlayerStartOptions};
pub use rusterix::server::connection::{PlayerConnectionId, PlayerConnectionState, PlayerControl};
use rusterix::{Assets, EntityAction, Server, ServerState};
pub use shared::project::Project;
use std::time::Duration;
use theframework_core::{TheTime, Uuid};
pub use view::{FontSource, RegionInspection, RuntimeView, ScreenSource};

#[derive(Clone, Copy, Debug)]
pub struct SessionOptions {
    pub debug: bool,
    pub create_local_player: bool,
    pub require_start_region: bool,
    pub events: EventInterests,
}
impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            debug: false,
            create_local_player: true,
            require_start_region: false,
            events: EventInterests::default(),
        }
    }
}
impl SessionOptions {
    pub fn terminal() -> Self {
        Self {
            require_start_region: true,
            events: EventInterests::terminal(),
            ..Default::default()
        }
    }
}
#[derive(Default, Debug)]
pub struct SessionUpdate {
    pub system_ticks: u32,
    pub movement_steps: u32,
    pub region_changed: Option<String>,
    pub time: Option<TheTime>,
    pub replication_error: Option<ReplicationError>,
}
/// The CPU world and document live here. A presenter borrows this state and
/// owns its own visual resources; replacing a surface/device cannot reset it.
pub struct GameSession {
    pub project: Project,
    pub assets: Assets,
    pub server: Server,
    pub current_map: String,
    pub clock: SessionClock,
    pump: SessionPump,
    started: bool,
    local_player_started: bool,
    player_start_sources: rustc_hash::FxHashMap<Uuid, rusterix::player_start::PlayerStartSource>,
}
/// Explicit mutable CPU host borrow. Presenters receive RuntimeView or
/// RegionInspection instead, never this simulation/collection capability.
pub struct SessionParts<'a> {
    pub project: &'a mut Project,
    pub pump: &'a mut SessionPump,
    pub server: &'a mut Server,
    pub assets: &'a mut Assets,
}
impl GameSession {
    pub fn parts(&mut self) -> SessionParts<'_> {
        SessionParts {
            project: &mut self.project,
            pump: &mut self.pump,
            server: &mut self.server,
            assets: &mut self.assets,
        }
    }
    pub fn driver(&self) -> &SessionPump {
        &self.pump
    }
    /// Migration startup adapters may still produce the legacy CPU registration
    /// command. Ownership and duplicate-registration policy remain here.
    pub fn submit_startup_commands(&mut self, commands: Vec<rusterix::Command>) {
        self.local_player_started |= commands.iter().any(|command| {
            matches!(command, rusterix::Command::CreateEntity(_, entity) if entity.is_player())
        });
        self.server.process_client_commands(commands);
    }
    pub fn new(project: Project) -> Self {
        let config = project.config.parse::<toml::Table>().unwrap_or_default();
        let game = config.get("game").and_then(toml::Value::as_table);
        let current_map = game
            .and_then(|t| t.get("start_region"))
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_owned();
        let tick = game
            .and_then(|t| t.get("game_tick_ms"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(250)
            .max(1) as u64;
        Self {
            project,
            assets: Assets::default(),
            server: Server::default(),
            current_map,
            clock: SessionClock::new(Duration::from_millis(tick)),
            pump: SessionPump::new(EventInterests::default()),
            started: false,
            local_player_started: false,
            player_start_sources: Default::default(),
        }
    }
    pub fn start(&mut self, options: SessionOptions) -> Result<(), String> {
        self.start_with_state(options, None)
    }
    pub(crate) fn start_with_state(
        &mut self,
        options: SessionOptions,
        restored: Option<&rusterix::server::persistence::WorldRecord>,
    ) -> Result<(), String> {
        let config = self
            .project
            .config
            .parse::<toml::Table>()
            .map_err(|e| format!("Game configuration: {e}"))?;
        let game = config.get("game").and_then(toml::Value::as_table);
        let start_map = game
            .and_then(|t| t.get("start_region"))
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .to_owned();
        let tick = game
            .and_then(|t| t.get("game_tick_ms"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(250)
            .max(1) as u64;
        if options.require_start_region && start_map.is_empty() {
            return Err("Game config is missing [game].start_region".into());
        }
        if !start_map.is_empty() && !self.project.regions.iter().any(|r| r.name == start_map) {
            return Err(format!("Start region '{}' not found", start_map));
        }
        self.pump.renew_identity();
        shared::runtime_assets::start_server_restored(
            &mut self.server,
            &mut self.assets,
            &mut self.project,
            options.debug,
            restored,
        )?;
        self.player_start_sources = self
            .project
            .regions
            .iter()
            .filter_map(|region| {
                rusterix::player_start::PlayerStartSource::from_map(&self.assets, &region.map)
                    .ok()
                    .map(|source| (region.map.id, source))
            })
            .collect();
        self.current_map = start_map;
        self.pump.restart_collection(options.events);
        self.pump.enable_replication();
        self.clock = SessionClock::new(Duration::from_millis(tick));
        self.started = true;
        self.local_player_started = false;
        self.clock.reset_elapsed();
        if options.create_local_player && restored.is_none() {
            if let Err(e) = self.create_local_player() {
                self.stop();
                return Err(e);
            }
        }
        if let Err(error) = self.refresh_replication() {
            self.stop();
            return Err(error.to_string());
        }
        Ok(())
    }
    pub fn create_local_player(&mut self) -> Result<(), String> {
        let config = self
            .project
            .config
            .parse::<toml::Table>()
            .map_err(|e| format!("Game configuration: {e}"))?;
        if !config
            .get("game")
            .and_then(|g| g.get("auto_create_player"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(());
        }
        self.start_player(PlayerStartOptions::default())?;
        Ok(())
    }
    /// Register once, independently of automatic-start configuration. Character
    /// creation screens use this with their chosen class/name and entrance flag.
    pub fn start_player(
        &mut self,
        options: PlayerStartOptions<'_>,
    ) -> Result<Option<PlayerStartInfo>, String> {
        if !self.started {
            return Err("Start the local session before registering a player".into());
        }
        if self.local_player_started {
            return Ok(None);
        }
        let map = self
            .project
            .regions
            .iter()
            .find(|r| r.map.name == self.current_map)
            .map(|r| &r.map)
            .ok_or_else(|| format!("Start region '{}' not found", self.current_map))?;
        let prepared = if let Some(source) = self.player_start_sources.get(&map.id) {
            source.prepare(map, options)
        } else {
            rusterix::player_start::prepare_player_start(&self.assets, map, options)?
        };
        let rusterix::Command::CreateEntity(region, entity) = prepared.command;
        self.server
            .create_connected_player(self.server.local_connection(), &region, entity)?;
        self.local_player_started = true;
        Ok(Some(prepared.info))
    }

    /// Open an additional player connection into this world, not another world.
    pub fn open_player_connection(&mut self) -> Result<PlayerConnectionId, String> {
        if !self.started {
            return Err("Start the world before opening a player connection".into());
        }
        let connection = self.server.open_player_connection();
        if let Err(error) = self.refresh_replication() {
            self.close_player_connection(connection);
            return Err(error.to_string());
        }
        Ok(connection)
    }

    /// Prepare character choices against immutable authored startup sources.
    /// Registration is acknowledged asynchronously by the region. Repeated
    /// requests on the same connection never create an additional character.
    pub fn start_connected_player(
        &mut self,
        connection: PlayerConnectionId,
        region_name: &str,
        options: PlayerStartOptions<'_>,
    ) -> Result<Option<PlayerStartInfo>, String> {
        if !self.started {
            return Err("Start the world before registering a player".into());
        }
        match self.server.connection_state(connection) {
            Some(PlayerConnectionState::AwaitingCharacter) => {}
            Some(_) => return Ok(None),
            None => return Err("Unknown or expired player connection".into()),
        }
        let map = &self
            .project
            .regions
            .iter()
            .find(|region| region.map.name == region_name)
            .ok_or_else(|| format!("Start region '{region_name}' not found"))?
            .map;
        let prepared = if let Some(source) = self.player_start_sources.get(&map.id) {
            source.prepare(map, options)
        } else {
            rusterix::player_start::prepare_player_start(&self.assets, map, options)?
        };
        let rusterix::Command::CreateEntity(region, entity) = prepared.command;
        self.server
            .create_connected_player(connection, &region, entity)?;
        Ok(Some(prepared.info))
    }

    pub fn connected_player_action(
        &self,
        connection: PlayerConnectionId,
        action: EntityAction,
    ) -> Result<(), String> {
        self.server.player_action(connection, action)
    }
    pub fn submit_movement_input(
        &mut self,
        connection: PlayerConnectionId,
        input: MovementInput,
    ) -> Result<MovementSubmission, MovementInputError> {
        self.pump.submit_movement(&self.server, connection, input)
    }
    /// Opt into fixed input frames. Default embedded play retains its direct
    /// action path; remote/predicted clients consume a fresh baseline here.
    pub fn enable_frame_input(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<(), FrameInputError> {
        let control = self
            .server
            .player_control(connection)
            .ok_or(FrameInputError::UnknownConnection)?;
        if !self.server.set_frame_motion(connection, true) {
            return Err(FrameInputError::UnknownConnection);
        }
        self.pump.enable_frame_input(connection, control);
        self.request_baseline(connection)
            .map_err(|_| FrameInputError::NeedsBaseline)
    }
    pub fn submit_input_frame(
        &mut self,
        connection: PlayerConnectionId,
        command: MovementCommand,
    ) -> Result<MovementSubmission, FrameInputError> {
        self.pump
            .submit_input_frame(&self.server, connection, command)
    }
    pub fn disable_frame_input(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<(), FrameInputError> {
        if !self.server.set_frame_motion(connection, false) {
            return Err(FrameInputError::UnknownConnection);
        }
        self.pump.disable_frame_input(connection);
        self.request_baseline(connection)
            .map_err(|_| FrameInputError::NeedsBaseline)
    }
    /// Last accepted state, not an acknowledgment that simulation processed it.
    pub fn accepted_movement_input(&self, connection: PlayerConnectionId) -> Option<MovementInput> {
        self.server.connection_state(connection)?;
        self.pump.accepted_movement(connection)
    }
    pub fn movement_tick_sequence(&self) -> u64 {
        self.pump.movement_tick_sequence()
    }

    /// Remote action context is recipient-bound. The host supplies an exact
    /// issued choice, never a deserialized client callback/event/price object.
    pub fn prepare_game_request(
        &self,
        connection: PlayerConnectionId,
        world: Uuid,
        generation: u64,
        region: Uuid,
        request: &requests::GameRequest,
        choice: Option<&rusterix::Choice>,
    ) -> Result<Vec<rusterix::EntityAction>, requests::RequestError> {
        use requests::RequestError as E;
        let control = self.server.player_control(connection).ok_or(E::NotControlled)?;
        let replica = self.client_world(connection).ok_or(E::StaleContext)?;
        if self.driver().session_id() != world
            || replica.generation() != generation
            || replica.current_region() != Some(region)
            || replica.controlled_entity() != Some(control.entity_id)
            || self.server.state != rusterix::ServerState::Running
        {
            return Err(E::StaleContext);
        }
        let instance = self.server.instances.iter()
            .find(|i| i.lock().unwrap().id == control.region_id)
            .ok_or(E::NotControlled)?;
        let instance = instance.lock().unwrap();
        if instance.context().is_none_or(|ctx| ctx.lock().unwrap().map.id != region) {
            return Err(E::StaleContext);
        }
        instance.prepare_game_request(control.entity_id, request, choice)
    }
    pub fn close_player_connection(&mut self, connection: PlayerConnectionId) -> bool {
        let closed = self.server.close_player_connection(connection);
        if closed {
            self.pump.remove_replica(connection);
        }
        closed
    }
    fn refresh_replication(&mut self) -> Result<(), ReplicationError> {
        self.pump.publish_replication(
            &self.project,
            &self.server,
            &self.current_map,
            &Default::default(),
        )
    }
    pub fn client_world(&self, connection: PlayerConnectionId) -> Option<&ClientWorld> {
        self.server.connection_state(connection)?;
        self.pump.client_world(connection)
    }
    /// Recipient maps for local presentation services, including off-screen
    /// regions. Unstarted authoring fixtures retain their preview sources.
    pub fn client_maps(&self) -> impl Iterator<Item = &rusterix::Map> + Clone {
        let replica = self.client_world(self.server.local_connection());
        self.project
            .regions
            .iter()
            .filter_map(move |region| match replica {
                Some(client) => client.map(region.map.id),
                None => (!self.started).then_some(&region.map),
            })
    }
    /// Trusted host policy. None restores complete embedded/Creator replicas.
    /// A wire client cannot select regions or inspect another character through it.
    pub fn set_spatial_subscriptions(
        &mut self,
        connection: PlayerConnectionId,
        config: Option<replication::SpatialSubscriptions>,
    ) -> Result<(), ReplicationError> {
        if self.server.connection_state(connection).is_none() {
            return Err(ReplicationError::UnknownConnection);
        }
        if let Some(config) = &config {
            if config
                .regions
                .iter()
                .chain(&config.prefetch)
                .any(|id| !self.project.regions.iter().any(|r| r.map.id == *id))
            {
                return Err(ReplicationError::InvalidFrame(
                    "Unknown subscription region".into(),
                ));
            }
        }
        self.pump.set_spatial(connection, config)?;
        self.refresh_replication()
    }
    pub fn prefetched_content(
        &self,
        connection: PlayerConnectionId,
    ) -> Vec<replication::RegionContentRef> {
        self.pump
            .prefetch(connection)
            .iter()
            .filter_map(|id| {
                let region = self.project.regions.iter().find(|r| r.map.id == *id)?;
                let runtime = self.server.region_id_map.get(id)?;
                Some(replication::RegionContentRef {
                    map: *id,
                    map_revision: region.map.changed,
                    runtime_revision: self.server.runtime_map_version(*runtime),
                })
            })
            .collect()
    }
    /// Opt in to bounded timing samples; disabled for local play by default.
    pub fn enable_diagnostics(&mut self,enabled:bool) {
        self.server.enable_diagnostics(enabled);
        self.pump.enable_diagnostics(enabled);
    }
    pub fn diagnostics(&self)->Option<diagnostics::SessionDiagnostics> {
        self.pump.diagnostics(&self.server)
    }
    /// Explicitly opt custom public HUD/appearance attributes into replication.
    /// Call after start; changing the policy replaces all connection baselines.
    pub fn set_replication_policy(
        &mut self,
        policy: ReplicationPolicy,
    ) -> Result<(), ReplicationError> {
        self.pump.set_replication_policy(policy)?;
        self.refresh_replication()
    }
    /// Subscribe independently of local presentation. The first frame is a full
    /// current baseline; transient events from before subscription are omitted.
    pub fn subscribe_replication(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<(), ReplicationError> {
        self.require_connection(connection)?;
        if self.pump.frame_input_enabled(connection) {
            self.request_baseline(connection)?;
        }
        self.pump.subscribe_replication(connection)
    }
    /// Build an independent predicted client for an in-process/Creator test
    /// connection. Its inputs and packets can be delayed independently by a
    /// transport/fault harness; embedded play is unchanged unless opted in.
    pub fn create_motion_client(
        &mut self,
        connection: PlayerConnectionId,
        settings: MotionSettings,
    ) -> Result<MotionClient, String> {
        self.enable_frame_input(connection)
            .map_err(|e| e.to_string())?;
        self.subscribe_replication(connection)
            .map_err(|e| e.to_string())?;
        let frame = self
            .take_client_frames(connection)
            .map_err(|e| e.to_string())?
            .pop()
            .ok_or("No motion baseline")?;
        let mut client = MotionClient::new(frame.world, frame.connection, settings);
        self.install_motion_content(&mut client, &frame)
            .map_err(|e| e.to_string())?;
        client
            .receive(&frame, &self.assets)
            .map_err(|e| e.to_string())?;
        Ok(client)
    }
    /// Trusted in-process content provisioner. Network clients load sanitized
    /// content packages separately; no authored Map/Assets is a network packet.
    pub fn install_motion_content(
        &self,
        client: &mut MotionClient,
        frame: &ClientFrame,
    ) -> Result<(), ReplicationError> {
        let references: Vec<_> = match &frame.update {
            replication::WorldUpdate::Baseline(regions) => regions.iter().map(|r|r.content).collect(),
            replication::WorldUpdate::Delta(regions) => regions.iter().map(|r|r.content).collect(),
        };
        for reference in references {
            let region = self.project.regions.iter().find(|r|r.map.id==reference.map)
                .ok_or(ReplicationError::MissingContent(reference))?;
            client.install_content(reference,&region.map)?;
            if let Some(id)=self.server.region_id_map.get(&region.map.id)
                && let Some(instance)=self.server.instances.iter().find(|i|i.lock().unwrap().id==*id)
                && let Some(context)=instance.lock().unwrap().context()
            {
                let context=context.lock().unwrap();
                // The zero-latency Creator/test adapter supplies the same whole
                // authoritative collision used by network packages. In particular,
                // a spatially filtered prop list must never erase real blockers.
                client.install_authoritative_collision(reference,context.mapmini.clone(),context.collision_world.clone());
            }
        }
        Ok(())
    }
    /// Zero-latency in-process adapter for a predicted player/Creator test.
    /// The host advances even when this client's presentation is suspended.
    /// A dedicated/remote host drives its clock separately and transports these
    /// same commands and frames rather than calling this convenience adapter.
    pub fn advance_motion_client(
        &mut self,
        connection: PlayerConnectionId,
        client: &mut MotionClient,
        elapsed: Duration,
    ) -> Result<SessionUpdate, String> {
        if !client
            .authority()
            .belongs_to(self.pump.session_id(), connection.uuid())
        {
            return Err(ReplicationError::WrongConnection.to_string());
        }
        // Collect control/teleport/pause changes before sampling input against
        // the old scope. Polling grants no simulation time.
        if let Some(error) = self.poll().replication_error {
            return Err(error.to_string());
        }
        self.deliver_motion_frames(connection, client)?;
        if client.needs_baseline() {
            self.request_baseline(connection)
                .map_err(|e| e.to_string())?;
            self.deliver_motion_frames(connection, client)?;
        }
        client.advance(elapsed, &self.assets);
        for command in client.pending_inputs() {
            match self.submit_input_frame(connection, command) {
                Ok(_) => {}
                Err(FrameInputError::Backpressure) => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        let update = self.advance(elapsed);
        if let Some(error) = &update.replication_error {
            return Err(error.to_string());
        }
        self.deliver_motion_frames(connection, client)?;
        if client.needs_baseline() {
            self.request_baseline(connection)
                .map_err(|e| e.to_string())?;
            self.deliver_motion_frames(connection, client)?;
        }
        Ok(update)
    }
    fn deliver_motion_frames(
        &mut self,
        connection: PlayerConnectionId,
        client: &mut MotionClient,
    ) -> Result<(), String> {
        let frames = match self.take_client_frames(connection) {
            Ok(frames) => frames,
            Err(ReplicationError::Backpressure) => {
                self.subscribe_replication(connection)
                    .map_err(|e| e.to_string())?;
                self.take_client_frames(connection)
                    .map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        };
        for frame in frames {
            self.install_motion_content(client, &frame)
                .map_err(|e| e.to_string())?;
            match client.receive(&frame, &self.assets) {
                Ok(_) => {}
                Err(ReplicationError::NeedsBaseline) => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }
    pub fn take_client_frames(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<Vec<ClientFrame>, ReplicationError> {
        self.require_connection(connection)?;
        self.pump.take_client_frames(connection)
    }
    pub fn request_baseline(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<(), ReplicationError> {
        self.require_connection(connection)?;
        self.pump.request_baseline(connection)?;
        self.refresh_replication()
    }
    pub fn take_connection_events(
        &mut self,
        connection: PlayerConnectionId,
        region: Uuid,
    ) -> Result<RegionEvents, ReplicationError> {
        self.require_connection(connection)?;
        Ok(self
            .pump
            .client_world_mut(connection)
            .ok_or(ReplicationError::UnknownConnection)?
            .take_events(region))
    }
    pub fn take_connection_event_overflow(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<bool, ReplicationError> {
        self.require_connection(connection)?;
        Ok(self
            .pump
            .client_world_mut(connection)
            .ok_or(ReplicationError::UnknownConnection)?
            .take_event_overflow())
    }
    fn require_connection(&self, connection: PlayerConnectionId) -> Result<(), ReplicationError> {
        self.server
            .connection_state(connection)
            .map(|_| ())
            .ok_or(ReplicationError::UnknownConnection)
    }
    /// Monotonic count of local system ticks issued by this session. Resets on
    /// successful start; rendering, polling and GPU lifetime do not advance it.
    pub fn tick_sequence(&self) -> u64 {
        self.pump.tick_sequence()
    }
    pub fn has_started_player(&self) -> bool {
        self.local_player_started
    }
    pub fn is_started(&self) -> bool {
        self.started
    }
    pub fn action(&mut self, action: EntityAction) {
        self.server.local_player_action(action);
    }
    pub fn advance(&mut self, elapsed: Duration) -> SessionUpdate {
        let update = self.pump.advance(
            &mut self.clock,
            elapsed,
            &mut self.project,
            &mut self.server,
            &mut self.assets,
            &mut self.current_map,
        );
        if let Some(time) = update.time {
            self.project.time = time;
        }
        update
    }
    /// Explicit deterministic step for command-driven hosts and startup settling.
    pub fn tick_once(&mut self) -> SessionUpdate {
        if self.server.state == ServerState::Running {
            self.advance(self.clock.step())
        } else {
            self.poll()
        }
    }
    /// Polling cannot issue a system tick; output stays owned until consumed.
    pub fn poll(&mut self) -> SessionUpdate {
        self.update(0)
    }
    fn update(&mut self, ticks: u32) -> SessionUpdate {
        let update = self.pump.update(
            ticks,
            true,
            &mut self.project,
            &mut self.server,
            &mut self.assets,
            &mut self.current_map,
        );
        if let Some(time) = update.time {
            self.project.time = time;
        }
        update
    }
    pub fn take_events(&mut self, id: &Uuid) -> RegionEvents {
        self.pump.take_events(id)
    }
    pub fn take_messages(&mut self, id: &Uuid) -> Vec<rusterix::server::Message> {
        self.pump
            .pending_events(id)
            .map(|e| std::mem::take(&mut e.messages))
            .unwrap_or_default()
    }
    pub fn take_says(&mut self, id: &Uuid) -> Vec<rusterix::server::Say> {
        self.pump
            .pending_events(id)
            .map(|e| std::mem::take(&mut e.says))
            .unwrap_or_default()
    }
    pub fn stop(&mut self) {
        if self.started {
            self.pump.invalidate();
            self.server.stop();
            self.started = false;
            self.local_player_started = false;
        }
        self.player_start_sources.clear();
        self.pump.clear_events();
        self.clock.reset_elapsed();
    }
}
impl Drop for GameSession {
    fn drop(&mut self) {
        self.stop();
    }
}
