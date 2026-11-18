pub mod assets;
pub mod avatar_assets;
pub mod connection;
pub mod currency;
pub mod data;
pub mod entity;
pub mod event_observation;
pub mod item;
pub mod message;
pub mod nodes;
pub(crate) mod party;
pub mod py_fn;
pub mod region;
pub mod region_host;
pub mod regionctx;
pub mod persistence;
pub mod world;
pub mod scheduling;
mod ticking;

pub use connection::{PlayerConnectionId, PlayerConnectionState, PlayerControl};
use crossbeam_channel::{Receiver, Sender};
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use toml::Table;

use crate::Command;
use crate::EntityAction;
use crate::prelude::*;
use crate::server::message::{AudioCommand, PaletteRemap2DState, RuntimeRenderState};
use std::sync::{Arc, Mutex, RwLock};
use theframework_core::prelude::*;

// Routing and registration belong to one Server, including Creator Play.
type RegionRegistry = Arc<RwLock<FxHashMap<u32, Sender<RegionMessage>>>>;

// SenderEntityId, SenderItemId, ReceiverId, Message
pub type Message = (Option<u32>, Option<u32>, u32, String, String);
// SenderEntityId, SenderItemId, Message, Category
pub type Say = (Option<u32>, Option<u32>, String, String);
// ContainerItemId, OwnerEntityId
pub type OpenContainerRequest = (u32, Option<u32>);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum ServerState {
    Off,
    Running,
    Paused,
}

pub struct Server {
    scheduling: scheduling::RegionSchedulingPolicy,
    diagnostics: Option<Arc<Mutex<crate::diagnostics::RegionTimings>>>,
    region_pipes: RegionRegistry,
    local_connection: PlayerConnectionId,
    connections: FxHashMap<PlayerConnectionId, PlayerConnectionState>,
    world: Arc<world::SimulationWorld>,
    restore_bootstrap: bool,
    pub node_highlights: Vec<nodes::Highlights>,
    pub node_traces: std::collections::VecDeque<nodes::Trace>,
    pub id_gen: u32,

    /// In debug mode the server sends Eldrin source-line status updates.
    pub debug_mode: bool,
    pub eldrin_debug: EldrinDebugModule,
    pub event_session: Uuid,
    pub event_observations: std::collections::VecDeque<event_observation::EventObservation>,

    /// Maps region uuids to the region id
    pub region_id_map: FxHashMap<Uuid, u32>,
    from_region: Vec<Receiver<RegionMessage>>,

    /// Maps region names to the region id
    pub region_name_id_map: FxHashMap<String, u32>,

    // By region
    pub entities: FxHashMap<u32, Vec<Entity>>,
    pub items: FxHashMap<u32, Vec<Item>>,
    pub messages: FxHashMap<u32, Vec<Message>>,
    pub says: FxHashMap<u32, Vec<Say>>,
    world_texts: FxHashMap<u32, Vec<(u32, crate::world_text::WorldText)>>,
    pub open_container_requests: FxHashMap<u32, Vec<OpenContainerRequest>>,
    addressed_container_requests: FxHashMap<u32, Vec<(u32, OpenContainerRequest)>>,
    pub multiple_choice: FxHashMap<u32, Vec<MultipleChoice>>,
    pub audio_commands: FxHashMap<u32, Vec<AudioCommand>>,
    pub world_render: RuntimeRenderState,
    pub region_render: FxHashMap<u32, RuntimeRenderState>,
    pub times: FxHashMap<u32, TheTime>,
    pub runtime_maps: FxHashMap<u32, Map>,
    runtime_map_versions: FxHashMap<u32, u64>,
    pub runtime_map_position_guards: FxHashMap<u32, u8>,
    pub block_prop_instances: FxHashMap<u32, Vec<BlockPropInstance>>,

    pub state: ServerState,

    pub log: String,
    pub log_changed: bool,
    pub print_log_messages: bool,

    pub instances: Vec<Arc<Mutex<RegionInstance>>>,
    last_visual_update_at: Instant,
    legacy_clock: crate::simulation::SimulationClock,
    last_legacy_advance_at: Option<Instant>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let local_connection = PlayerConnectionId::new();
        Self {
            diagnostics:None,
            region_pipes: Default::default(),
            local_connection,
            connections: [(local_connection, PlayerConnectionState::AwaitingCharacter)]
                .into_iter()
                .collect(),
            world: Default::default(),
            restore_bootstrap: false,
            node_traces: Default::default(),
            node_highlights: Default::default(),
            // 0 is reserved as NO_ID / None sentinel.
            id_gen: 1,
            scheduling: Default::default(),

            debug_mode: false,
            eldrin_debug: EldrinDebugModule::default(),
            event_observations: Default::default(),
            event_session: Uuid::new_v4(),

            region_id_map: FxHashMap::default(),
            region_name_id_map: FxHashMap::default(),
            from_region: vec![],

            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            messages: FxHashMap::default(),
            says: FxHashMap::default(),
            world_texts: FxHashMap::default(),
            open_container_requests: FxHashMap::default(),
            addressed_container_requests: FxHashMap::default(),
            multiple_choice: FxHashMap::default(),
            audio_commands: FxHashMap::default(),
            world_render: RuntimeRenderState::default(),
            region_render: FxHashMap::default(),
            times: FxHashMap::default(),
            runtime_maps: FxHashMap::default(),
            runtime_map_versions: FxHashMap::default(),
            runtime_map_position_guards: FxHashMap::default(),
            block_prop_instances: FxHashMap::default(),

            state: ServerState::Off,

            log: String::new(),
            log_changed: true,
            print_log_messages: true,

            instances: vec![],
            last_visual_update_at: Instant::now(),
            legacy_clock: crate::simulation::SimulationClock::new(std::time::Duration::from_millis(250)),
            last_legacy_advance_at: None,
        }
    }

    /// Queue edits at this world's normal region update boundaries.
    pub fn publish_node_graph(&self, owner: String, graph: serde_json::Value) {
        if let Ok(pipes) = self.region_pipes.read() {
            for sender in pipes.values() {
                let _ = sender.send(RegionMessage::UpdateNodeGraph(owner.clone(), graph.clone()));
            }
        }
    }
    pub fn publish_rules(&self, rules: Table) {
        if let Ok(pipes) = self.region_pipes.read() {
            for sender in pipes.values() {
                let _ = sender.send(RegionMessage::UpdateRules(rules.clone()));
            }
        }
    }
    fn follows_local_player(&self, region: Option<u32>, entity: Option<u32>) -> bool {
        match self.player_control(self.local_connection) {
            Some(player) => region == Some(player.region_id) || entity == Some(player.entity_id),
            None => !self
                .connections
                .values()
                .any(|state| state.control().is_some()),
        }
    }

    /// The implicit single-player/reference-host connection. Its handle changes
    /// on clear/restart, just like explicitly opened connections.
    pub fn local_connection(&self) -> PlayerConnectionId {
        self.local_connection
    }

    pub fn open_player_connection(&mut self) -> PlayerConnectionId {
        let id = PlayerConnectionId::new();
        self.connections
            .insert(id, PlayerConnectionState::AwaitingCharacter);
        id
    }

    pub fn connection_state(&self, id: PlayerConnectionId) -> Option<PlayerConnectionState> {
        self.connections.get(&id).copied()
    }

    pub fn player_connections(&self) -> impl Iterator<Item = (PlayerConnectionId, PlayerConnectionState)> + '_ {
        self.connections.iter().map(|(id, state)| (*id, *state))
    }

    pub fn runtime_map_version(&self, region: u32) -> u64 {
        self.runtime_map_versions.get(&region).copied().unwrap_or(0)
    }

    pub fn player_container_access(&self, connection: PlayerConnectionId) -> Option<OpenContainerRequest> {
        let player = self.player_control(connection)?;
        self.instances.iter().find_map(|instance| {
            let instance = instance.lock().ok()?;
            (instance.id == player.region_id).then(|| instance.active_container_for(player.entity_id)).flatten()
        })
    }
    pub fn player_choice_is_active(&self, connection: PlayerConnectionId, choice: &Choice) -> bool {
        let Some(player) = self.player_control(connection) else { return false; };
        self.instances.iter().any(|instance| {
            let Ok(instance) = instance.lock() else { return false; };
            instance.id == player.region_id && instance.choice_is_active(player.entity_id, choice)
        })
    }

    pub fn player_control(&self, id: PlayerConnectionId) -> Option<PlayerControl> {
        self.connection_state(id)?.control()
    }

    pub fn local_player_id(&self) -> Option<u32> {
        self.player_control(self.local_connection)
            .map(|control| control.entity_id)
    }

    pub fn player_region_name(&self, id: PlayerConnectionId) -> Option<&str> {
        let control = self.player_control(id)?;
        self.region_name_id_map
            .iter()
            .find_map(|(name, region)| (*region == control.region_id).then_some(name.as_str()))
    }

    /// Submit only a host-prepared character template. This is not a public
    /// network CreateEntity command: a network host must validate creation
    /// choices and prepare the template before calling this boundary.
    pub fn create_connected_player(
        &mut self,
        id: PlayerConnectionId,
        region: &Uuid,
        entity: Entity,
    ) -> Result<(), String> {
        if self.connection_state(id) != Some(PlayerConnectionState::AwaitingCharacter) {
            return Err("Player connection is unknown or already has a character request".into());
        }
        let region_id = *self
            .region_id_map
            .get(region)
            .ok_or("Unknown player start region")?;
        let sender = self
            .region_pipes
            .read()
            .map_err(|_| "Region routing unavailable")?
            .get(&region_id)
            .cloned()
            .ok_or("Player start region is not running")?;
        sender
            .send(RegionMessage::CreateConnectedPlayer(id, entity))
            .map_err(|_| "Player start region is not running")?;
        self.connections
            .insert(id, PlayerConnectionState::Spawning { region_id });
        Ok(())
    }

    fn register_connected_player(
        &mut self,
        id: PlayerConnectionId,
        region_id: u32,
        entity_id: u32,
    ) {
        if self.player_control(id)
            == Some(PlayerControl {
                region_id,
                entity_id,
            })
        {
            return;
        }
        let accepted = matches!(self.connection_state(id),
            Some(PlayerConnectionState::Spawning { region_id: expected }) if expected == region_id)
            || (id == self.local_connection
                && self.connection_state(id) == Some(PlayerConnectionState::AwaitingCharacter));
        if accepted {
            self.connections.insert(
                id,
                PlayerConnectionState::Active(PlayerControl {
                    region_id,
                    entity_id,
                }),
            );
            let _ = self.send_player_message(
                region_id,
                RegionMessage::ShowStartupSectorDescription(entity_id),
            );
        } else {
            // A connection can close while its spawn is queued. Never attach
            // that late acknowledgement to another connection or leave motion on.
            let _ = self.send_player_message(
                region_id,
                RegionMessage::UserAction(entity_id, EntityAction::Off),
            );
        }
    }

    fn send_player_message(&self, region: u32, message: RegionMessage) -> Result<(), String> {
        self.region_pipes
            .read()
            .map_err(|_| "Region routing unavailable")?
            .get(&region)
            .ok_or("Player region is not running")?
            .send(message)
            .map_err(|_| "Player region is not running".into())
    }

    /// Actor identity is resolved here, never supplied by the requester.
    pub fn player_action(
        &self,
        id: PlayerConnectionId,
        action: EntityAction,
    ) -> Result<(), String> {
        let player = self
            .player_control(id)
            .ok_or("Player connection has no registered character")?;
        self.send_player_message(
            player.region_id,
            RegionMessage::UserAction(player.entity_id, action),
        )
    }

    pub fn player_event(
        &self,
        id: PlayerConnectionId,
        event: String,
        value: Value,
    ) -> Result<(), String> {
        let player = self
            .player_control(id)
            .ok_or("Player connection has no registered character")?;
        self.send_player_message(
            player.region_id,
            RegionMessage::UserEvent(player.entity_id, event, value),
        )
    }

    pub fn player_teleport(
        &self,
        id: PlayerConnectionId,
        sector: String,
        region: String,
    ) -> Result<(), String> {
        let player = self
            .player_control(id)
            .ok_or("Player connection has no registered character")?;
        self.send_player_message(
            player.region_id,
            RegionMessage::TeleportEntity(player.entity_id, sector, region),
        )
    }

    pub fn player_teleport_pos(
        &self,
        id: PlayerConnectionId,
        position: Vec2<f32>,
    ) -> Result<(), String> {
        let player = self
            .player_control(id)
            .ok_or("Player connection has no registered character")?;
        self.send_player_message(
            player.region_id,
            RegionMessage::TeleportEntityPos(player.entity_id, position),
        )
    }

    /// Detach control and stop held motion. Character persistence/despawn policy
    /// is a host concern; closing a connection does not destroy its character.
    pub fn close_player_connection(&mut self, id: PlayerConnectionId) -> bool {
        let Some(state) = self.connections.remove(&id) else {
            return false;
        };
        if let Some(player) = state.control() {
            let _ = self.send_player_message(
                player.region_id,
                RegionMessage::UserAction(player.entity_id, EntityAction::Off),
            );
        }
        true
    }

    /// Clear the log
    pub fn clear_log(&mut self) {
        self.log = String::new();
    }

    /// Retrieve the log
    pub fn get_log(&mut self) -> String {
        self.log_changed = false;
        self.log.clone()
    }

    /// Set the server state.
    pub fn set_state(&mut self, state: ServerState) {
        self.state = state;
    }

    /// Create the given region instance.
    pub fn create_region_instance(
        &mut self,
        name: String,
        map: Map,
        assets: &Assets,
        config_toml: String,
    ) {
        self.create_region_instance_with_shared_assets(
            name,
            map,
            &assets::RegionAssets::new(assets),
            config_toml,
        );
    }

    pub fn create_region_instance_with_shared_assets(
        &mut self,
        name: String,
        map: Map,
        assets: &assets::RegionAssets,
        config_toml: String,
    ) {
        let mut region_instance =
            RegionInstance::in_world(self.instances.len() as u32, self.world.clone());
        // region_instance.id = self.get_next_id();

        self.region_id_map.insert(map.id, region_instance.id);
        self.region_name_id_map
            .insert(name.clone(), region_instance.id);

        if let Ok(mut pipes) = self.region_pipes.write() {
            pipes.insert(region_instance.id, region_instance.to_sender.clone());
        }

        self.from_region.push(region_instance.from_receiver.clone());

        region_instance.restoring = self.restore_bootstrap;
        region_instance.init_with_shared_assets(name, map, assets, config_toml, self.debug_mode);
        self.instances.push(Arc::new(Mutex::new(region_instance)));
    }

    pub fn enable_diagnostics(&mut self,enabled:bool) {
        self.diagnostics=enabled.then(||Arc::new(Mutex::new(Default::default())));
        for instance in &self.instances {
            if let Some(ctx) = instance.lock().unwrap().context() { ctx.lock().unwrap().diagnostics = enabled.then(Default::default); }
        }
    }
    pub fn region_diagnostics(&self)->Vec<crate::diagnostics::RegionTimingSummary> {
        let mut summaries = self.diagnostics.as_ref().map(|d|crate::diagnostics::region_summaries(&d.lock().unwrap())).unwrap_or_default();
        for instance in &self.instances {
            let instance = instance.lock().unwrap();
            if let Some(summary) = summaries.iter_mut().find(|s| s.region == instance.id)
                && let Some(ctx) = instance.context()
                && let Some(details) = &ctx.lock().unwrap().diagnostics {
                summary.details = details.borrow().summary();
            }
        }
        summaries
    }
    pub fn set_frame_motion(&self, connection: PlayerConnectionId, enabled: bool) -> bool {
        let Some(control) = self.player_control(connection) else { return false; };
        self.instances.iter().any(|instance| {
            let mut region = instance.lock().unwrap();
            region.id == control.region_id && region.set_frame_motion(control.entity_id, enabled)
        })
    }
    pub fn player_motion_snapshot(
        &self,
        connection: PlayerConnectionId,
    ) -> Option<crate::movement::MotionSnapshot> {
        let control = self.player_control(connection)?;
        self.instances.iter().find_map(|instance| {
            let region = instance.lock().ok()?;
            (region.id == control.region_id)
                .then(|| region.frame_motion_snapshot(control.entity_id))
                .flatten()
        })
    }
    pub fn apply_movement_frame(
        &self,
        connection: PlayerConnectionId,
        command: crate::movement::MovementCommand,
    ) -> bool {
        let Some(control) = self.player_control(connection) else { return false; };
        self.instances.iter().any(|instance| {
            let mut region = instance.lock().unwrap();
            region.id == control.region_id && region.apply_movement_frame(control.entity_id, command)
        })
    }
    /// Compatibility adapter for hosts that still own a Rusterix facade.
    /// Wall time is measured only here, never by regions or redraw callbacks.
    pub fn advance_wall_clock(&mut self, game_step: std::time::Duration) -> u32 {
        let now = Instant::now();
        let elapsed = self.last_legacy_advance_at.replace(now)
            .map(|previous| now.saturating_duration_since(previous)).unwrap_or_default();
        self.advance_elapsed(elapsed, game_step)
    }
    /// Explicit host time for reference bootstrap/manual stepping. Canonical
    /// GameSession hosts schedule through their owned SessionClock instead.
    pub fn advance_elapsed(&mut self, elapsed: std::time::Duration, game_step: std::time::Duration) -> u32 {
        self.redraw_tick();
        if self.legacy_clock.step() != game_step {
            self.legacy_clock = crate::simulation::SimulationClock::new(game_step);
        }
        if self.state != ServerState::Running {
            self.legacy_clock.reset_elapsed();
            return 0;
        }
        let mut ticks = 0;
        for step in self.legacy_clock.schedule(elapsed) {
            match step {
                crate::simulation::ScheduledStep::Game => { self.system_tick(); ticks += 1; }
                crate::simulation::ScheduledStep::Movement => self.movement_tick(),
            }
        }
        ticks
    }

    /// Process a set of commands from a client.
    pub fn process_client_commands(&mut self, commands: Vec<Command>) {
        for cmd in commands {
            match cmd {
                Command::CreateEntity(id, entity) => {
                    if entity.is_player() {
                        let _ = self.create_connected_player(self.local_connection, &id, entity);
                        continue;
                    }
                    if let Some(region_id) = self.region_id_map.get(&id) {
                        if let Ok(pipe) = self.region_pipes.read() {
                            if let Some(sender) = pipe.get(region_id) {
                                match sender.send(RegionMessage::CreateEntity(*region_id, entity)) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("{:?}", err.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get entities and items for a given region.
    pub fn get_entities_items(
        &self,
        region_id: &Uuid,
    ) -> (Option<&Vec<Entity>>, Option<&Vec<Item>>) {
        let mut rc: (Option<&Vec<Entity>>, Option<&Vec<Item>>) = (None, None);

        rc.0 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.entities.get(region_id)
        } else {
            None
        };

        rc.1 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.items.get(region_id)
        } else {
            None
        };

        rc
    }

    /// Apply entities and items for a given region.
    pub fn apply_entities_items(&self, map: &mut Map) {
        if let Some(region_id) = self.region_id_map.get(&map.id) {
            if let Some(runtime_map) = self.runtime_maps.get(region_id) {
                *map = runtime_map.clone();
            }

            if let Some(entities) = self.entities.get(region_id) {
                map.entities = entities.clone();
            }

            if let Some(items) = self.items.get(region_id) {
                map.items = items.clone();
            }

            if let Some(instances) = self.block_prop_instances.get(region_id) {
                map.block_prop_instances = instances.clone();
            }
        };
    }

    /// Get messages for a given region and clear them.
    pub fn get_messages(&mut self, region_id: &Uuid) -> Vec<Message> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let messages = self.messages.get(region_id).cloned();
            self.messages.remove(region_id);
            messages.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Drain private world text for a region; the client checks the recipient.
    pub fn get_world_texts(
        &mut self,
        region_id: &Uuid,
    ) -> Vec<(u32, crate::world_text::WorldText)> {
        self.region_id_map
            .get(region_id)
            .and_then(|id| self.world_texts.remove(id))
            .unwrap_or_default()
    }

    /// Get says for a given region and clear them.
    pub fn get_says(&mut self, region_id: &Uuid) -> Vec<Say> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let says = self.says.get(region_id).cloned();
            self.says.remove(region_id);
            says.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Get queued container panel requests for a given region and clear them.
    pub fn take_container_recipients(&mut self, region_id: &Uuid) -> Vec<(u32, OpenContainerRequest)> {
        self.region_id_map.get(region_id).and_then(|id| self.addressed_container_requests.remove(id)).unwrap_or_default()
    }

    pub fn get_open_container_requests(&mut self, region_id: &Uuid) -> Vec<OpenContainerRequest> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            // Legacy collectors do not consume recipient metadata separately.
            self.addressed_container_requests.remove(region_id);
            let requests = self.open_container_requests.get(region_id).cloned();
            self.open_container_requests.remove(region_id);
            requests.unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Get multi-choice for a given region and clear them.
    pub fn get_choices(&mut self, region_id: &Uuid) -> Vec<MultipleChoice> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let choices = self.multiple_choice.get(region_id).cloned();
            self.multiple_choice.remove(region_id);
            choices.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Get queued audio commands for a given region and clear them.
    pub fn get_audio_commands(&mut self, region_id: &Uuid) -> Vec<AudioCommand> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let commands = self.audio_commands.get(region_id).cloned();
            self.audio_commands.remove(region_id);
            commands.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Get the current time for the given region.
    pub fn get_time(&self, region_id: &Uuid) -> Option<TheTime> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Some(time) = self.times.get(region_id) {
                return Some(*time);
            }
        }
        None
    }

    /// Get the effective runtime render state for the given region.
    pub fn get_render_state(&self, region_id: &Uuid) -> RuntimeRenderState {
        self.region_id_map
            .get(region_id)
            .map(|region_id| {
                self.world_render
                    .clone()
                    .merged(self.region_render.get(region_id).cloned())
            })
            .unwrap_or_else(|| self.world_render.clone())
    }

    /// Set the current time for the given region.
    pub fn set_time(&mut self, region_id: &Uuid, time: TheTime) -> TheTime {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Ok(pipe) = self.region_pipes.read() {
                if let Some(sender) = pipe.get(region_id) {
                    self.times.clear();
                    match sender.send(RegionMessage::Time(*region_id, time)) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{:?}", err.to_string());
                        }
                    }
                }
            }
        }
        TheTime::default()
    }

    /// Retrieves all messages from the regions. Returns the name of the new region should the
    /// players region change.
    pub fn update(&mut self, assets: &mut Assets) -> Option<String> {
        let mut rc: Option<String> = None;
        self.eldrin_debug.clear();
        let now = Instant::now();
        let visual_dt = now
            .saturating_duration_since(self.last_visual_update_at)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        self.last_visual_update_at = now;

        for receiver in self.from_region.clone() {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    RegionMessage::RegisterPlayer(region_id, entity_id) => {
                        self.register_connected_player(self.local_connection, region_id, entity_id);
                    }
                    RegionMessage::RegisterConnectedPlayer(connection, region_id, entity_id) => {
                        self.register_connected_player(connection, region_id, entity_id);
                    }
                    RegionMessage::EntitiesUpdate(id, serialized_updates) => {
                        let updates: Vec<EntityUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| EntityUpdate::unpack(&data))
                            .collect();
                        let runtime_map = self.runtime_maps.get(&id);
                        let guard_runtime_positions = self
                            .runtime_map_position_guards
                            .get(&id)
                            .copied()
                            .unwrap_or(0)
                            > 0;

                        if let Some(entities) = self.entities.get_mut(&id) {
                            Self::process_entity_updates(
                                entities,
                                updates,
                                assets,
                                runtime_map,
                                guard_runtime_positions,
                            );
                        } else {
                            let mut entities = vec![];
                            Self::process_entity_updates(
                                &mut entities,
                                updates,
                                assets,
                                runtime_map,
                                guard_runtime_positions,
                            );
                            self.entities.insert(id, entities);
                        }

                        if let Some(guard) = self.runtime_map_position_guards.get_mut(&id) {
                            *guard = guard.saturating_sub(1);
                            if *guard == 0 {
                                self.runtime_map_position_guards.remove(&id);
                            }
                        }
                    }
                    RegionMessage::ItemsUpdate(id, serialized_updates) => {
                        let updates: Vec<ItemUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| ItemUpdate::unpack(&data))
                            .collect();

                        if let Some(items) = self.items.get_mut(&id) {
                            Self::process_item_updates(items, updates);
                        } else {
                            let mut items = vec![];
                            Self::process_item_updates(&mut items, updates);
                            self.items.insert(id, items);
                        }
                    }
                    RegionMessage::RemoveItem(region_id, item_id) => {
                        if let Some(items) = self.items.get_mut(&region_id) {
                            items.retain(|item| item.id != item_id);
                        }
                    }
                    RegionMessage::RemoveEntity(region_id, entity_id) => {
                        // Death stashes the body: drop the client-side copy so it
                        // stops rendering and being pickable. Bringing the body
                        // back needs no counterpart, because entity updates are
                        // upserts and re-create it.
                        if let Some(entities) = self.entities.get_mut(&region_id) {
                            entities.retain(|entity| entity.id != entity_id);
                        }
                    }
                    RegionMessage::OpenContainer(region_id, recipient, item_id, owner_entity_id) => {
                        self.open_container_requests
                            .entry(region_id)
                            .or_default()
                            .push((item_id, owner_entity_id));
                        self.addressed_container_requests.entry(region_id).or_default()
                            .push((recipient, (item_id, owner_entity_id)));
                    }
                    RegionMessage::LogMessage(message) => {
                        if self.print_log_messages {
                            println!("{}", message);
                        }
                        if self.log.is_empty() {
                            self.log = message;
                        } else {
                            self.log += &format!("{}{}", "\n", message);
                        }
                        self.log_changed = true;
                    }
                    RegionMessage::Message(
                        id,
                        sender_entity,
                        sender_item,
                        receiver_id,
                        message,
                        category,
                    ) => {
                        // println!(
                        //     "({:?}, {:?}) -> {}: {}",
                        //     sender_entity, sender_item, receiver_id, message
                        // );
                        //

                        if let Some(messages) = self.messages.get_mut(&id) {
                            messages.push((
                                sender_entity,
                                sender_item,
                                receiver_id,
                                message,
                                category,
                            ));
                        } else {
                            let messages =
                                vec![(sender_entity, sender_item, receiver_id, message, category)];
                            self.messages.insert(id, messages);
                        }
                    }
                    RegionMessage::WorldText(id, receiver, text) => {
                        // Text clients and journals retain the same description; graphical
                        // clients render the structured anchor instead of a message widget.
                        let (entity, item) = match &text.anchor {
                            crate::world_text::TextAnchor::Entity(id) => (Some(*id), None),
                            crate::world_text::TextAnchor::Item(id) => (None, Some(*id)),
                            _ => (None, None),
                        };
                        self.messages.entry(id).or_default().push((
                            entity,
                            item,
                            receiver,
                            text.text.clone(),
                            "look".into(),
                        ));
                        let pending = self.world_texts.entry(id).or_default();
                        if pending.len() >= 64 {
                            pending.remove(0);
                        }
                        pending.push((receiver, text));
                    }
                    RegionMessage::Say(id, sender_entity, sender_item, message, category) => {
                        if let Some(says) = self.says.get_mut(&id) {
                            says.push((sender_entity, sender_item, message, category));
                        } else {
                            self.says
                                .insert(id, vec![(sender_entity, sender_item, message, category)]);
                        }
                    }
                    RegionMessage::MultipleChoice(choices) => {
                        if let Some(multi_choice) = self.multiple_choice.get_mut(&choices.region) {
                            multi_choice.push(choices.clone());
                        } else {
                            let multi_choice = vec![choices.clone()];
                            self.multiple_choice.insert(choices.region, multi_choice);
                        }
                    }
                    RegionMessage::AudioCmd(region_id, cmd) => {
                        if let Some(commands) = self.audio_commands.get_mut(&region_id) {
                            commands.push(cmd);
                        } else {
                            self.audio_commands.insert(region_id, vec![cmd]);
                        }
                    }
                    RegionMessage::SetPaletteRemap2D(region_id, start_index, end_index, mode) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        let palette = state
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.start_index = start_index.min(255);
                        palette.end_index = end_index.min(255);
                        palette.mode = mode;
                    }
                    RegionMessage::SetPaletteRemap2DBlend(region_id, blend) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        let palette = state
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.blend = blend.clamp(0.0, 1.0);
                    }
                    RegionMessage::SetWorldPaletteRemap2D(start_index, end_index, mode) => {
                        let palette = self
                            .world_render
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.start_index = start_index.min(255);
                        palette.end_index = end_index.min(255);
                        palette.mode = mode;
                    }
                    RegionMessage::SetWorldPaletteRemap2DBlend(blend) => {
                        let palette = self
                            .world_render
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.blend = blend.clamp(0.0, 1.0);
                    }
                    RegionMessage::SetRenderValue(region_id, name, value) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        state.render.set(&name, value);
                    }
                    RegionMessage::SetWorldRenderValue(name, value) => {
                        self.world_render.render.set(&name, value);
                    }
                    RegionMessage::SetPostValue(region_id, name, value) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        state.post.set(&name, value);
                    }
                    RegionMessage::SetWorldPostValue(name, value) => {
                        self.world_render.post.set(&name, value);
                    }
                    RegionMessage::Time(id, time) => {
                        self.times.insert(id, time);
                    }
                    RegionMessage::TransferEntity(
                        from_region_id,
                        mut entity,
                        dest_region_name,
                        dest_sector_name,
                    ) => {
                        // A disconnect can race with an already queued transfer.
                        // The destination receives stopped motion even if the
                        // release sent to the source can no longer find its body.
                        if entity.is_player()
                            && !self.connections.values().any(|state| {
                                state
                                    .control()
                                    .is_some_and(|control| control.entity_id == entity.id)
                            })
                        {
                            entity.action = EntityAction::Off;
                        }
                        // If we cannot find the destination region, send the entity back from where it came
                        let mut dest_id = from_region_id;
                        if let Some(region_id) = self.region_name_id_map.get(&dest_region_name) {
                            dest_id = *region_id;
                        }

                        let mut removed_local: Option<Entity> = None;
                        // Remove entity from the old region
                        if let Some(entities) = self.entities.get_mut(&from_region_id) {
                            if let Some(pos) = entities.iter().position(|e| e.id == entity.id) {
                                removed_local = Some(entities.remove(pos));
                            }
                        }

                        // Add entity to the dest region
                        if let Some(mut removed_local) = removed_local {
                            if let Some(dest_map) = self
                                .runtime_maps
                                .get(&dest_id)
                                .or_else(|| assets.maps.get(&dest_region_name))
                                && let Some(dest_center) =
                                    dest_map.named_area_center(&dest_sector_name)
                            {
                                removed_local.set_pos_xz(dest_center);
                                removed_local.mark_all_dirty();
                            }
                            if let Some(entities) = self.entities.get_mut(&dest_id) {
                                entities.push(removed_local);
                            } else {
                                self.entities.insert(dest_id, vec![removed_local]);
                            }
                            if self.follows_local_player(None, Some(entity.id)) {
                                rc = Some(dest_region_name.clone());
                            }
                        }

                        // Change the local player reference to the new region
                        for state in self.connections.values_mut() {
                            if let PlayerConnectionState::Active(player) = state
                                && player.entity_id == entity.id
                            {
                                player.region_id = dest_id;
                            }
                        }

                        // Messages emitted during the same script event before the deferred
                        // transfer is processed are initially bucketed under the source region.
                        // Move player-addressed messages with the entity so UI widgets in the
                        // destination region still receive them.
                        if from_region_id != dest_id {
                            let mut moved_messages = Vec::new();
                            if let Some(messages) = self.messages.get_mut(&from_region_id) {
                                let mut kept = Vec::with_capacity(messages.len());
                                for message in messages.drain(..) {
                                    if message.2 == entity.id {
                                        moved_messages.push(message);
                                    } else {
                                        kept.push(message);
                                    }
                                }
                                *messages = kept;
                            }
                            if !moved_messages.is_empty() {
                                self.messages
                                    .entry(dest_id)
                                    .or_default()
                                    .extend(moved_messages);
                            }
                        }

                        if let Ok(pipe) = self.region_pipes.read() {
                            if let Some(sender) = pipe.get(&dest_id) {
                                match sender.send(RegionMessage::TransferEntity(
                                    dest_id,
                                    entity.clone(),
                                    dest_region_name.clone(),
                                    dest_sector_name.clone(),
                                )) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("{:?}", err.to_string());
                                    }
                                }
                            }
                        }
                    }
                    RegionMessage::MapUpdate(id, map) => {
                        *self.runtime_map_versions.entry(id).or_default() += 1;
                        if let Some(region_id) = self.region_id_map.get(&map.id).copied() {
                            self.entities.insert(region_id, map.entities.clone());
                            self.items.insert(region_id, map.items.clone());
                            self.block_prop_instances
                                .insert(region_id, map.block_prop_instances.clone());
                            self.runtime_maps.insert(region_id, map.clone());
                            self.runtime_map_position_guards.insert(region_id, 4);
                        }
                        assets.maps.insert(map.name.clone(), map.clone());
                        if self.follows_local_player(Some(id), None) {
                            rc = Some(map.name.clone());
                        }
                    }
                    RegionMessage::BlockPropInstancesUpdate(id, instances) => {
                        // Layout/material identity changes need a new static
                        // reference; live collision has an independent stamp.
                        let art_changed = self.block_prop_instances.get(&id).is_none_or(|old| {
                            old.len() != instances.len() || old.iter().zip(&instances).any(|(a,b)| {
                                a.id != b.id || a.asset_id != b.asset_id || a.overrides != b.overrides || a.parameter_overrides != b.parameter_overrides
                            })
                        });
                        if art_changed { *self.runtime_map_versions.entry(id).or_default() += 1; }
                        self.block_prop_instances.insert(id, instances);
                    }
                    RegionMessage::NodeHighlights(updates) => {
                        for update in updates {
                            self.node_highlights.retain(|old| {
                                old.event.map != update.event.map
                                    || old.graph != update.graph
                                    || old.actor.identity != update.actor.identity
                            });
                            if !update.active.nodes.is_empty() || !update.recent.nodes.is_empty() {
                                self.node_highlights.push(update);
                            }
                        }
                    }
                    RegionMessage::NodeTraces(traces) => {
                        for trace in traces {
                            if self.node_traces.len() >= 512 {
                                self.node_traces.pop_front();
                            }
                            self.node_traces.push_back(trace);
                        }
                    }
                    RegionMessage::EventObservations(events) => {
                        for event in events {
                            event_observation::retain_observation(
                                &mut self.event_observations,
                                event,
                            );
                        }
                    }
                    RegionMessage::EldrinDebugData(data) => {
                        self.eldrin_debug.merge(&data);
                    }
                    _ => {}
                }
            }
        }

        for entities in self.entities.values_mut() {
            for entity in entities.iter_mut() {
                entity.advance_position_interpolation(visual_dt);
            }
        }

        rc
    }

    fn client_entity_interp_duration(assets: &Assets) -> f32 {
        let Ok(config) = assets.config.parse::<Table>() else {
            return 0.0;
        };
        let game = config.get("game").and_then(|v| v.as_table());
        let simulation_mode = game
            .and_then(|table| table.get("simulation_mode"))
            .and_then(|value| value.as_str())
            .unwrap_or("realtime");
        if simulation_mode.eq_ignore_ascii_case("realtime") {
            return 0.0;
        }
        game.and_then(|table| table.get("game_tick_ms"))
            .and_then(|value| value.as_integer())
            .map(|ms| (ms.max(1) as f32) / 1000.0)
            .unwrap_or(0.25)
    }

    /// Update existing entities (or create new ones if they do not exist).
    pub fn process_entity_updates(
        entities: &mut Vec<Entity>,
        updates: Vec<EntityUpdate>,
        assets: &mut Assets,
        runtime_map: Option<&Map>,
        guard_runtime_positions: bool,
    ) {
        let interp_duration = Self::client_entity_interp_duration(assets);
        // Create a mapping from entity ID to index for efficient lookup
        let mut entity_map: FxHashMap<u32, usize> = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = entity_map.get(&update.id) {
                if guard_runtime_positions
                    && entities[index].is_player()
                    && let Some(position) = update.position
                {
                    let pos_2d = Vec2::new(position.x, position.z);
                    let outside_runtime_map = runtime_map
                        .map(|map| map.find_sector_at(pos_2d).is_none())
                        .unwrap_or(false);
                    let moved_far =
                        (position - entities[index].position).magnitude_squared() > 0.25;
                    if outside_runtime_map && moved_far {
                        continue;
                    }
                }

                let previous_position = entities[index].position;
                let new_position = update.position;
                let snap_position = update.snap_position;
                let should_interpolate = interp_duration > 0.0
                    && new_position.is_some()
                    && !snap_position
                    && !entities[index].is_player();
                // Entity exists, apply the update
                if entities[index].apply_update(update) {
                    assets.entity_tiles.remove(&entities[index].id);
                }
                if should_interpolate && let Some(target_position) = new_position {
                    entities[index].begin_position_interpolation(
                        previous_position,
                        target_position,
                        interp_duration,
                    );
                }
            } else {
                // Entity does not exist, create a new one
                let mut new_entity = Entity {
                    id: update.id,
                    ..Default::default()
                };
                new_entity.apply_update(update);

                // Add to the entity list
                let new_entity_id = new_entity.id;
                entities.push(new_entity);

                // Update the map with the new entitys ID
                entity_map.insert(new_entity_id, entities.len() - 1);
            }
        }
    }

    /// Update existing items (or create new ones if they do not exist).
    pub fn process_item_updates(items: &mut Vec<Item>, updates: Vec<ItemUpdate>) {
        // Create a mapping from entity ID to index for efficient lookup
        let mut item_map: FxHashMap<u32, usize> = items
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = item_map.get(&update.id) {
                // Entity exists, apply the update
                items[index].apply_update(update);
            } else {
                // Entity does not exist, create a new one
                let mut new_item = Item {
                    id: update.id,
                    ..Default::default()
                };
                new_item.apply_update(update);

                // Add to the item list
                let new_entity_id = new_item.id;
                items.push(new_item);

                // Update the map with the new items ID
                item_map.insert(new_entity_id, items.len() - 1);
            }
        }
    }

    /// Compatibility entry points address only the implicit local connection.
    pub fn local_player_event(&mut self, event: String, value: Value) {
        let _ = self.player_event(self.local_connection, event, value);
    }

    pub fn has_local_player(&self) -> bool {
        self.player_control(self.local_connection).is_some()
    }

    pub fn local_player_action(&mut self, action: EntityAction) {
        let _ = self.player_action(self.local_connection, action);
    }

    /// Trusted local host operations; these are not exposed as wire requests.
    pub fn local_player_teleport(&mut self, sector_name: String, region_name: String) {
        let _ = self.player_teleport(self.local_connection, sector_name, region_name);
    }

    pub fn local_player_teleport_pos(&mut self, position: Vec2<f32>) {
        let _ = self.player_teleport_pos(self.local_connection, position);
    }

    /// Pause all region instances.
    pub fn pause(&mut self) {
        self.legacy_clock.reset_elapsed();
        self.last_legacy_advance_at = None;
        if let Ok(pipes) = self.region_pipes.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Pause);
            }
        }
        self.state = ServerState::Paused;
    }

    /// Continue all region instances.
    pub fn continue_instances(&mut self) {
        self.last_legacy_advance_at = None;
        if let Ok(pipes) = self.region_pipes.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Continue);
            }
        }
        self.state = ServerState::Running;
    }

    /// Shuts down all region instances.
    pub fn stop(&mut self) {
        if let Ok(pipes) = self.region_pipes.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Quit);
            }
        }
        self.clear();
    }

    /// Shuts down all region instances.
    pub fn clear(&mut self) {
        self.scheduling = Default::default();
        self.legacy_clock.reset_elapsed();
        self.last_legacy_advance_at = None;
        self.node_traces.clear();
        self.node_highlights.clear();
        if let Ok(mut pipes) = self.region_pipes.write() {
            pipes.clear();
        }
        self.connections.clear();
        self.local_connection = PlayerConnectionId::new();
        self.connections.insert(
            self.local_connection,
            PlayerConnectionState::AwaitingCharacter,
        );
        self.event_session = Uuid::new_v4();
        self.event_observations.clear();
        self.entities.clear();
        self.items.clear();
        self.messages.clear();
        self.says.clear();
        self.world_texts.clear();
        self.open_container_requests.clear();
        self.addressed_container_requests.clear();
        self.audio_commands.clear();
        self.world_render = RuntimeRenderState::default();
        self.region_render.clear();
        self.runtime_maps.clear();
        self.runtime_map_versions.clear();
        self.runtime_map_position_guards.clear();
        self.block_prop_instances.clear();
        self.multiple_choice.clear();
        self.id_gen = 1;
        self.region_id_map.clear();
        self.region_name_id_map.clear();
        self.state = ServerState::Off;
        self.from_region.clear();
        self.times.clear();
        self.clear_log();

        // A retained old context cannot mutate a subsequently started world.
        self.world = Default::default();

        self.instances.clear();
    }

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_region_and_player_ids_route_only_to_the_owning_server() {
        let mut first = Server::new();
        let mut second = Server::new();
        let region = Uuid::new_v4();
        let (first_tx, first_rx) = crossbeam_channel::unbounded();
        let (second_tx, second_rx) = crossbeam_channel::unbounded();
        for (server, sender) in [(&mut first, first_tx), (&mut second, second_tx)] {
            server.region_id_map.insert(region, 0);
            server.region_pipes.write().unwrap().insert(0, sender);
            server.connections.insert(
                server.local_connection,
                PlayerConnectionState::Active(PlayerControl {
                    region_id: 0,
                    entity_id: 7,
                }),
            );
        }
        first.local_player_action(EntityAction::Forward);
        assert!(matches!(
            first_rx.try_recv().unwrap(),
            RegionMessage::UserAction(7, EntityAction::Forward)
        ));
        assert!(second_rx.is_empty());
        first.pause();
        assert!(matches!(first_rx.try_recv().unwrap(), RegionMessage::Pause));
        assert!(second_rx.is_empty());
        first.publish_node_graph("entity:hero".into(), serde_json::json!({"version": 1}));
        assert!(matches!(
            first_rx.try_recv().unwrap(),
            RegionMessage::UpdateNodeGraph(_, _)
        ));
        assert!(second_rx.is_empty());
        first.publish_rules(Table::new());
        assert!(matches!(
            first_rx.try_recv().unwrap(),
            RegionMessage::UpdateRules(_)
        ));
        assert!(second_rx.is_empty());
        second.stop();
        assert!(matches!(second_rx.try_recv().unwrap(), RegionMessage::Quit));
        assert!(first_rx.is_empty());
        assert!(first.has_local_player());
        assert!(!second.has_local_player());
        first.local_player_action(EntityAction::Off);
        assert!(matches!(
            first_rx.try_recv().unwrap(),
            RegionMessage::UserAction(7, EntityAction::Off)
        ));
    }

    #[test]
    fn world_state_and_runtime_ids_share_within_a_session_and_reset_without_revoking_old_handles() {
        let mut first = Server::new();
        let second = Server::new();
        let retained = first.world.clone();
        let same_session = retained.clone();
        retained.set("quest", Value::Int(4));
        assert_eq!(same_session.get("quest"), Some(Value::Int(4)));
        assert_eq!(second.world.get("quest"), None);
        assert_eq!(retained.next_id(), 1);
        assert_eq!(same_session.next_id(), 2);
        assert_eq!(second.world.next_id(), 1);
        second.world.set("quest", Value::Int(9));
        first.clear();
        assert_eq!(first.world.get("quest"), None);
        assert_eq!(first.world.next_id(), 1);
        assert_eq!(retained.get("quest"), Some(Value::Int(4)));
        assert_eq!(retained.next_id(), 3);
        assert_eq!(second.world.get("quest"), Some(Value::Int(9)));
        assert_eq!(second.world.next_id(), 2);
    }

    #[test]
    fn clear_removes_generated_runtime_maps() {
        let mut server = Server::new();
        server.runtime_maps.insert(7, Map::default());
        server.runtime_map_position_guards.insert(7, 4);

        server.clear();

        assert!(server.runtime_maps.is_empty());
        assert!(server.runtime_map_position_guards.is_empty());
    }

    /// Death takes a body out of the world, and clients render from
    /// `Server::entities`. Removing the body there is what stops a dead
    /// character from rendering, exactly like `RemoveItem` does for items.
    #[test]
    fn remove_entity_message_drops_the_body_from_the_client_model() {
        let mut server = Server::new();
        let (sender, receiver) = crossbeam_channel::unbounded();
        server.from_region.push(receiver);

        let mut body = Entity::new();
        body.id = 5;
        let mut bystander = Entity::new();
        bystander.id = 6;
        server.entities.insert(7, vec![body, bystander]);

        sender
            .send(RegionMessage::RemoveEntity(7, 5))
            .expect("send removal");

        let mut assets = Assets::default();
        server.update(&mut assets);

        let entities = &server.entities[&7];
        assert!(
            entities.iter().all(|entity| entity.id != 5),
            "the dead body has to leave the client's entity list"
        );
        assert!(
            entities.iter().any(|entity| entity.id == 6),
            "an unrelated body stays"
        );
    }
}
