//! One CPU local session, shared by terminal, graphical players and Creator Play.
//! There is no surface, UI context, font shaping, GPU device or render callback.
mod clock;
mod events;
pub use clock::SessionClock;
pub use events::{EventInterests, RegionEvents};
use rustc_hash::FxHashMap;
use rusterix::{Assets, Command, EntityAction, Server, ServerState};
pub use shared::project::Project;
use std::time::Duration;
use theframework_core::{TheTime, Uuid};

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
    events: FxHashMap<Uuid, RegionEvents>,
    interests: EventInterests,
    started: bool,
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
            events: FxHashMap::default(),
            interests: EventInterests::default(),
            started: false,
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
        shared::runtime_assets::start_server(
            &mut self.server,
            &mut self.assets,
            &mut self.project,
            options.debug,
        )?;
        self.current_map = start_map;
        self.clock = SessionClock::new(Duration::from_millis(tick));
        self.started = true;
        self.interests = options.events;
        self.events.clear();
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
        let region = self
            .project
            .regions
            .iter()
            .find(|r| r.map.name == self.current_map)
            .ok_or_else(|| format!("Start region '{}' not found", self.current_map))?;
        let players = self
            .assets
            .entities
            .iter()
            .filter_map(|(name, (_, data))| {
                let table = data.parse::<toml::Table>().ok()?;
                table
                    .get("attributes")?
                    .get("player")?
                    .as_bool()?
                    .then_some(name)
            })
            .collect::<Vec<_>>();
        let entity = region
            .map
            .entities
            .iter()
            .find(|e| {
                e.get_attr_string("class_name")
                    .is_some_and(|name| players.iter().any(|p| **p == name))
            })
            .cloned()
            .ok_or_else(|| {
                format!(
                    "No auto-created player entity found in start region '{}'",
                    self.current_map
                )
            })?;
        self.server
            .process_client_commands(vec![Command::CreateEntity(region.map.id, entity)]);
        Ok(())
    }
    pub fn action(&mut self, action: EntityAction) {
        self.server.local_player_action(action);
    }
    pub fn advance(&mut self, elapsed: Duration) -> SessionUpdate {
        let ticks = if self.server.state == ServerState::Running {
            self.clock.advance(elapsed)
        } else {
            self.clock.reset_elapsed();
            0
        };
        self.update(ticks)
    }
    /// Explicit deterministic step for command-driven hosts and startup settling.
    pub fn tick_once(&mut self) -> SessionUpdate {
        self.update(u32::from(self.server.state == ServerState::Running))
    }
    /// Drain worker results even on a frame with no simulation tick. Every output
    /// channel is collected once and remains owned until its consumer takes it.
    pub fn poll(&mut self) -> SessionUpdate {
        self.update(0)
    }
    fn update(&mut self, ticks: u32) -> SessionUpdate {
        for _ in 0..ticks {
            self.server.system_tick();
        }
        self.server.redraw_tick();
        let changed = self.server.update(&mut self.assets);
        if let Some(name) = &changed {
            self.current_map = name.clone();
        }
        let mut time = None;
        for region in &mut self.project.regions {
            self.server.apply_entities_items(&mut region.map);
            if region.map.name == self.current_map {
                time = self.server.get_time(&region.map.id);
                if let Some(t) = time {
                    self.project.time = t;
                }
            }
            self.events.entry(region.map.id).or_default().collect(
                &mut self.server,
                &region.map.id,
                self.interests,
            );
        }
        SessionUpdate {
            system_ticks: ticks,
            region_changed: changed,
            time,
        }
    }
    pub fn take_events(&mut self, id: &Uuid) -> RegionEvents {
        self.events.remove(id).unwrap_or_default()
    }
    pub fn take_messages(&mut self, id: &Uuid) -> Vec<rusterix::server::Message> {
        self.events
            .get_mut(id)
            .map(|e| std::mem::take(&mut e.messages))
            .unwrap_or_default()
    }
    pub fn take_says(&mut self, id: &Uuid) -> Vec<rusterix::server::Say> {
        self.events
            .get_mut(id)
            .map(|e| std::mem::take(&mut e.says))
            .unwrap_or_default()
    }
    pub fn stop(&mut self) {
        if self.started {
            self.server.stop();
            self.started = false;
        }
        self.events.clear();
        self.clock.reset_elapsed();
    }
}
impl Drop for GameSession {
    fn drop(&mut self) {
        self.stop();
    }
}
