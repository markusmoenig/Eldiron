//! One CPU local session, shared by terminal, graphical players and Creator Play.
//! There is no surface, UI context, font shaping, GPU device or render callback.
mod clock;
mod events;
mod pump;
mod player;
pub use player::{
    PlayerControls, PlayerInput, PlayerInputStats, PlayerSession, PlayerSettings, PlayerTransition, is_movement_state,
};
mod view;
pub use clock::SessionClock;
pub use events::{EventInterests, RegionEvents, RegionOutputs};
pub use pump::SessionPump;
pub use rusterix::player_start::{PlayerStartInfo, PlayerStartOptions};
use rusterix::{Assets, EntityAction, Server, ServerState};
pub use shared::project::Project;
use std::time::Duration;
use theframework_core::{TheTime, Uuid};
pub use view::{FontSource, RuntimeView, ScreenSource};

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
    pub region_changed: Option<String>,
    pub time: Option<TheTime>,
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
}
impl GameSession {
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
        }
    }
    pub fn start(&mut self, options: SessionOptions) -> Result<(), String> {
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
        shared::runtime_assets::start_server(
            &mut self.server,
            &mut self.assets,
            &mut self.project,
            options.debug,
        )?;
        self.current_map = start_map;
        self.pump.restart_collection(options.events);
        self.clock = SessionClock::new(Duration::from_millis(tick));
        self.started = true;
        self.local_player_started = false;
        self.clock.reset_elapsed();
        if options.create_local_player {
            if let Err(e) = self.create_local_player() {
                self.stop();
                return Err(e);
            }
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
        let prepared = rusterix::player_start::prepare_player_start(&self.assets, map, options)?;
        self.server.process_client_commands(vec![prepared.command]);
        self.local_player_started = true;
        Ok(Some(prepared.info))
    }
    /// Monotonic count of local system ticks issued by this session. Resets on
    /// successful start; rendering, polling and GPU lifetime do not advance it.
    pub fn tick_sequence(&self) -> u64 {
        self.pump.tick_sequence()
    }
    pub fn has_started_player(&self) -> bool {
        self.local_player_started
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
        self.update(u32::from(self.server.state == ServerState::Running))
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
        self.pump.clear_events();
        self.clock.reset_elapsed();
    }
}
impl Drop for GameSession {
    fn drop(&mut self) {
        self.stop();
    }
}
