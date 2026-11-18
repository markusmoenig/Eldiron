//! Shared graphical-player application orchestration. The authoritative runtime
//! remains in GameSession; surface, fonts, screen layouts and GPU resources are
//! assembled by the graphical host, including Creator Play and foreign hosts.
use crate::{
    GameSession, PlayerStartInfo, PlayerStartOptions, Project, SessionOptions, SessionUpdate,
};
use rustc_hash::FxHashMap;
use rusterix::{
    EntityAction, Value, input_action::ClientAction, input_binding::ClientCommandBinding,
};
use std::{collections::VecDeque, time::Duration};
use toml::Table;

#[derive(Clone, Debug)]
pub struct PlayerSettings {
    pub viewport: (u32, u32),
    pub grid_size: f32,
    pub window_scale: f32,
    pub target_fps: f64,
    pub start_screen: String,
    pub play_screen: String,
    pub auto_create_player: bool,
    pub play: shared::project_play::ProjectPlaySettings,
}
impl PlayerSettings {
    pub fn parse(source: &str) -> Result<Self, String> {
        let table = source
            .parse::<Table>()
            .map_err(|e| format!("Game configuration: {e}"))?;
        let viewport = table.get("viewport");
        let game = table.get("game");
        let number = |table: Option<&toml::Value>, key: &str, default: f64| {
            let value = table
                .and_then(|t| t.get(key))
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .unwrap_or(default);
            if !value.is_finite() {
                Err(format!("Game setting '{key}' must be finite"))
            } else {
                Ok(value)
            }
        };
        let string = |table: Option<&toml::Value>, key: &str| {
            table
                .and_then(|t| t.get(key))
                .and_then(toml::Value::as_str)
                .unwrap_or("")
                .trim()
                .to_owned()
        };
        let grid = number(viewport, "grid_size", 32.)?.max(1.);
        let cell = string(viewport, "unit").eq_ignore_ascii_case("cell");
        let dimension = |key, default| {
            let size = number(viewport, key, default)?.max(1.) * if cell { grid } else { 1. };
            if !size.is_finite() || size > u32::MAX as f64 {
                return Err(format!("Viewport {key} exceeds supported dimensions"));
            }
            Ok(size.round() as u32)
        };
        let window_scale = number(viewport, "window_scale", 1.)?;
        if grid > f32::MAX as f64 || window_scale > f32::MAX as f64 {
            return Err("Viewport scale/grid exceeds supported values".into());
        }
        let viewport = (dimension("width", 1280.)?, dimension("height", 720.)?);
        if [viewport.0, viewport.1]
            .iter()
            .any(|size| *size as f64 * window_scale.max(0.1) > u32::MAX as f64)
        {
            return Err("Scaled window exceeds supported dimensions".into());
        }
        Ok(Self {
            play: shared::project_play::ProjectPlaySettings::parse(source)?,
            viewport,
            grid_size: grid as f32,
            window_scale: window_scale.max(0.1) as f32,
            target_fps: number(game, "target_fps", 30.)?.clamp(1., 240.),
            start_screen: string(game, "start_screen"),
            play_screen: string(game, "play_screen"),
            auto_create_player: game
                .and_then(|g| g.get("auto_create_player"))
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
        })
    }
    pub fn window_size(&self) -> (u32, u32) {
        (
            (self.viewport.0 as f64 * self.window_scale as f64)
                .round()
                .clamp(1., u32::MAX as f64) as u32,
            (self.viewport.1 as f64 * self.window_scale as f64)
                .round()
                .clamp(1., u32::MAX as f64) as u32,
        )
    }
}
#[derive(Default, Debug)]
pub struct PlayerTransition {
    pub player: Option<PlayerStartInfo>,
    pub screen: Option<String>,
}
/// Input orchestration diagnostics, independent of rendering. Submitted actions
/// include accepted startup queue entries; flushing that queue does not recount
/// them as new input. Coalesced movement states may not all reach the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerInputStats {
    pub submitted_actions: u64,
    pub pending_actions: usize,
    pub held_controls: usize,
}
pub struct PlayerSession {
    pub runtime: GameSession,
    pub settings: PlayerSettings,
    input: PlayerInput,
    remote_mode: bool,
    remote: Option<crate::MotionClient>,
    remote_actions: VecDeque<EntityAction>,
}
impl PlayerSession {
    pub fn new(project: Project, debug: bool) -> Result<Self, String> {
        Self::new_for(project, debug, shared::project_play::LaunchPurpose::Player)
    }
    pub fn new_local_test(project: Project, debug: bool) -> Result<Self, String> {
        Self::new_for(
            project,
            debug,
            shared::project_play::LaunchPurpose::CreatorTest,
        )
    }
    fn new_for(
        project: Project,
        debug: bool,
        purpose: shared::project_play::LaunchPurpose,
    ) -> Result<Self, String> {
        if project.client_release.is_some() && purpose==shared::project_play::LaunchPurpose::CreatorTest {return Err("Public client releases cannot run Creator/local tests".into());}
        let settings = PlayerSettings::parse(&project.config)?;
        let remote_mode = matches!(
            settings.play.launch(purpose)?,
            shared::project_play::LaunchPlan::Remote(_)
        );
        let mut runtime = GameSession::new(project);
        if remote_mode {
            shared::runtime_assets::prepare_client_content(
                &mut runtime.assets,
                &mut runtime.project,
                debug,
            )?;
        } else {
            runtime.start(SessionOptions {
                debug,
                create_local_player: false,
                ..Default::default()
            })?;
        }
        let mut input = PlayerInput::default();
        if remote_mode
            && let Some(map) = runtime
                .project
                .regions
                .iter()
                .find(|r| r.map.name == runtime.current_map)
            && let Ok(prepared) = rusterix::player_start::prepare_player_start(
                &runtime.assets,
                &map.map,
                PlayerStartOptions::default(),
            )
        {
            input.controls.configure(&prepared.info, &runtime.assets);
        }
        Ok(Self {
            runtime,
            settings,
            input,
            remote_mode,
            remote: None,
            remote_actions: VecDeque::new(),
        })
    }
    pub fn is_remote(&self) -> bool {
        self.remote_mode
    }
    pub fn remote(&self) -> Option<&crate::MotionClient> {
        self.remote.as_ref()
    }
    pub fn remote_mut(&mut self) -> Option<&mut crate::MotionClient> {
        self.remote.as_mut()
    }
    pub fn attach_remote(&mut self, client: crate::MotionClient) -> Result<(), String> {
        if !self.remote_mode {
            return Err("Cannot attach a remote client to a local world".into());
        }
        self.remote = Some(client);
        Ok(())
    }
    pub fn take_remote_actions(&mut self) -> Vec<EntityAction> {
        self.remote_actions.drain(..).collect()
    }
    pub fn reset_remote(&mut self) {
        if self.remote_mode {
            self.remote = None;
            self.remote_actions.clear();
        }
    }
    pub fn install_remote_content(
        &mut self,
        reference: crate::replication::RegionContentRef,
    ) -> Result<(), String> {
        let map = &self
            .runtime
            .project
            .regions
            .iter()
            .find(|r| r.map.id == reference.map)
            .ok_or("Missing project region")?
            .map;
        self.remote
            .as_mut()
            .ok_or("No remote client")?
            .install_content(reference, map)
            .map_err(|e| e.to_string())
    }
    pub fn receive_remote_frame(&mut self, frame: &crate::ClientFrame) -> Result<(), String> {
        self.remote
            .as_mut()
            .ok_or("No remote client")?
            .receive(frame, &self.runtime.assets)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn view(&self) -> crate::RuntimeView<'_> {
        if !self.remote_mode {
            return self.runtime.view();
        }
        if let Some(client) = &self.remote
            && let Some(view) = client.view(&self.runtime.project, &self.runtime.assets, None)
        {
            return view;
        }
        crate::RuntimeView::from_replica(
            self.runtime.driver().session_id(),
            None,
            &self.runtime.project,
            &self.runtime.assets,
            None,
            self.runtime.project.time,
            rusterix::ServerState::Off,
            Default::default(),
        )
    }
    pub fn view_region(&self, id: &theframework_core::Uuid) -> Option<crate::RuntimeView<'_>> {
        if !self.remote_mode {
            return self.runtime.view_region(id);
        }
        self.remote
            .as_ref()?
            .view(&self.runtime.project, &self.runtime.assets, Some(*id))
    }
    pub fn take_events(&mut self, id: &theframework_core::Uuid) -> crate::RegionEvents {
        if self.remote_mode {
            self.remote
                .as_mut()
                .map(|c| c.take_events(*id))
                .unwrap_or_default()
        } else {
            self.runtime.take_events(id)
        }
    }
    pub fn client_maps(&self) -> impl Iterator<Item = &rusterix::Map> + Clone {
        let local = self
            .runtime
            .client_world(self.runtime.server.local_connection());
        self.runtime
            .project
            .regions
            .iter()
            .filter_map(move |region| {
                if self.remote_mode {
                    self.remote.as_ref()?.authority().map(region.map.id)
                } else if let Some(client) = local {
                    client.map(region.map.id)
                } else {
                    Some(&region.map)
                }
            })
    }
    pub fn tick_sequence(&self) -> u64 {
        self.remote
            .as_ref()
            .map_or_else(|| self.runtime.tick_sequence(), |c| c.authority().tick())
    }
    pub fn auto_start(&mut self, showing_game: bool) -> Result<PlayerTransition, String> {
        if !self.remote_mode
            && self.settings.auto_create_player
            && (self.settings.start_screen.is_empty() || showing_game)
        {
            let player = self.register(PlayerStartOptions::default())?;
            Ok(PlayerTransition {
                player,
                screen: None,
            })
        } else {
            Ok(PlayerTransition::default())
        }
    }
    fn register(
        &mut self,
        options: PlayerStartOptions<'_>,
    ) -> Result<Option<PlayerStartInfo>, String> {
        let player = self.runtime.start_player(options)?;
        if let Some(info) = &player {
            self.input.controls.configure(info, &self.runtime.assets);
        }
        Ok(player)
    }
    /// UI bindings remain owned by the active screen/application; startup reads
    /// only the named choices and returns a presentation transition.
    pub fn game_command(
        &mut self,
        command: &str,
        bindings: &mut FxHashMap<String, String>,
    ) -> Result<PlayerTransition, String> {
        if self.remote_mode {
            return Err("Online startup commands belong to the connection owner".into());
        }
        let command = command.trim();
        let class = if command == "start" {
            bindings
                .get("start.class")
                .filter(|s| !s.trim().is_empty())
                .cloned()
        } else if let Some(class) = command
            .strip_prefix("start_class.")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(class.to_owned())
        } else {
            return Err(format!("Unknown game command '{command}'"));
        };
        if let Some(class) = &class {
            bindings.insert("start.class".into(), class.clone());
        }
        let name = bindings.get("start.name").cloned();
        let player = self.register(PlayerStartOptions {
            class: class.as_deref(),
            name: name.as_deref(),
            entrance: true,
        })?;
        let screen = player.as_ref().and_then(|_| {
            (!self.settings.play_screen.is_empty()).then(|| self.settings.play_screen.clone())
        });
        Ok(PlayerTransition { player, screen })
    }
    pub fn screen_command(&self, command: &str) -> Result<String, String> {
        command
            .trim()
            .strip_prefix("goto.")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("Unknown screen command '{command}'"))
    }
    pub fn advance(&mut self, elapsed: Duration) -> SessionUpdate {
        if self.remote_mode {
            if let Some(client) = &mut self.remote {
                client.advance(elapsed, &self.runtime.assets);
            }
            return SessionUpdate::default();
        }
        let update = self.runtime.advance(elapsed);
        self.input.flush(&mut self.runtime.server);
        update
    }
    /// Keep startup input until the region acknowledges local registration.
    /// Consecutive movement states collapse so a release during startup cannot
    /// later replay stale held movement. One-shot commands retain order.
    pub fn submit_action(&mut self, action: EntityAction) -> Result<(), String> {
        if self.remote_mode {
            if let Some(client) = &mut self.remote {
                if let Some(intent) = rusterix::movement::MovementIntent::from_action(&action) {
                    client.set_intent(intent);
                } else if let EntityAction::LookYaw(yaw) = action {
                    client.look(yaw);
                } else {
                    if self.remote_actions.len() >= 64 {
                        return Err("Online action queue is busy".into());
                    }
                    self.remote_actions.push_back(action);
                }
            }
            return Ok(());
        }
        let requested = self.runtime.has_started_player();
        self.input
            .submit_action(&mut self.runtime.server, requested, action)
    }
    pub fn input_stats(&self) -> PlayerInputStats {
        self.input.input_stats()
    }
    pub fn pending_action_count(&self) -> usize {
        self.input.pending_action_count()
    }
    pub fn shortcut_labels_for_binding(&self, binding: &ClientCommandBinding) -> Vec<String> {
        self.input.controls.shortcut_labels_for_binding(binding)
    }
    pub fn binding_for_key(&self, key: &str) -> Option<ClientCommandBinding> {
        self.input.controls.binding_for_key(key)
    }
    pub fn control_key(&mut self, key: &str, pressed: bool) -> Option<EntityAction> {
        self.input.controls.control_key(key, pressed)
    }
    pub fn cancel_controls(&mut self) -> EntityAction {
        self.input.controls.cancel_controls()
    }
    pub fn set_isometric_controls(&mut self, isometric: bool) -> Option<EntityAction> {
        self.input.controls.set_isometric_controls(isometric)
    }
}

/// Runtime-owned accepted physical controls and startup action queue.
/// Renderers borrow this owner; hiding/recreating a view cannot replace it.
#[derive(Default)]
pub struct PlayerInput {
    connection: Option<crate::PlayerConnectionId>,
    pub controls: PlayerControls,
    pending_actions: VecDeque<EntityAction>,
    // One bounded stop slot remains available when ordered startup commands
    // fill the queue. A release must never leave an earlier movement active.
    pending_stop: bool,
    submitted_actions: u64,
}
impl PlayerInput {
    /// One input owner per player connection. Explicit owners never fall back
    /// to the implicit local character after disconnect or world replacement.
    pub fn for_connection(connection: crate::PlayerConnectionId) -> Self {
        Self {
            connection: Some(connection),
            ..Default::default()
        }
    }

    fn connection(&self, server: &rusterix::Server) -> crate::PlayerConnectionId {
        self.connection.unwrap_or_else(|| server.local_connection())
    }
    pub fn from_controls(controls: ClientAction) -> Self {
        Self {
            controls: controls.into(),
            ..Default::default()
        }
    }
    pub fn flush(&mut self, server: &mut rusterix::Server) {
        let connection = self.connection(server);
        if server.player_control(connection).is_some() {
            for action in self.pending_actions.drain(..) {
                let _ = server.player_action(connection, action);
            }
            if std::mem::take(&mut self.pending_stop) {
                let _ = server.player_action(connection, EntityAction::Off);
            }
        }
    }
    pub fn submit_action(
        &mut self,
        server: &mut rusterix::Server,
        requested: bool,
        action: EntityAction,
    ) -> Result<(), String> {
        if !requested {
            return Ok(());
        }
        let connection = self.connection(server);
        if server.connection_state(connection).is_none() {
            return Err("Unknown or expired player connection".into());
        }
        if server.player_control(connection).is_some() {
            self.flush(server);
            server.player_action(connection, action)?;
            self.submitted_actions = self.submitted_actions.saturating_add(1);
            return Ok(());
        }
        if is_movement_state(&action) && self.pending_actions.back().is_some_and(is_movement_state)
        {
            self.pending_actions.pop_back();
        }
        if self.pending_actions.len() >= 256 {
            if action == EntityAction::Off {
                self.pending_stop = true;
                self.submitted_actions = self.submitted_actions.saturating_add(1);
                return Ok(());
            }
            return Err(
                "Player registration has not completed; startup input queue is full".into(),
            );
        }
        self.pending_actions.push_back(action);
        self.submitted_actions = self.submitted_actions.saturating_add(1);
        Ok(())
    }
    pub fn input_stats(&self) -> PlayerInputStats {
        PlayerInputStats {
            submitted_actions: self.submitted_actions,
            pending_actions: self.pending_action_count(),
            held_controls: self.controls.held_count(),
        }
    }
    pub fn pending_action_count(&self) -> usize {
        self.pending_actions.len() + usize::from(self.pending_stop)
    }
}

/// CPU held-key policy shared by Creator Play and the graphical player.
pub struct PlayerControls {
    controls: ClientAction,
    held_controls: Vec<String>,
    last_control_action: EntityAction,
    isometric_controls: bool,
}
impl Default for PlayerControls {
    fn default() -> Self {
        Self::from(ClientAction::default())
    }
}
impl From<ClientAction> for PlayerControls {
    fn from(mut controls: ClientAction) -> Self {
        controls.release_all();
        controls.set_isometric(false);
        Self {
            controls,
            held_controls: Vec::new(),
            last_control_action: EntityAction::Off,
            isometric_controls: false,
        }
    }
}
impl PlayerControls {
    pub fn configure(&mut self, info: &PlayerStartInfo, assets: &rusterix::Assets) {
        self.cancel_controls();
        self.controls.init(info.template.clone(), assets);
        if let Some(input) = &info.input_bindings {
            self.controls.set_input_data(input);
        }
    }
    pub fn set_input_data(&mut self, data: &str) {
        self.cancel_controls();
        self.controls.set_input_data(data);
    }
    pub fn held_count(&self) -> usize {
        self.held_controls.len()
    }
    pub fn shortcut_labels_for_binding(&self, binding: &ClientCommandBinding) -> Vec<String> {
        self.controls.shortcut_labels_for_binding(binding)
    }
    pub fn binding_for_key(&self, key: &str) -> Option<ClientCommandBinding> {
        self.controls.binding_for_key(key)
    }
    /// Accepted physical controls are retained in press order. Rebuilding the
    /// existing reducer from those sources preserves aliases and last-pressed
    /// priority when one of several keys mapped to the same action is released.
    /// Only changed movement states produce actions; explicit Stop always does.
    pub fn control_key(&mut self, key: &str, pressed: bool) -> Option<EntityAction> {
        let Some(ClientCommandBinding::Control(action)) = self.controls.binding_for_key(key) else {
            return None;
        };
        if pressed && action == EntityAction::Off {
            return Some(self.cancel_controls());
        }
        let key = key.to_ascii_lowercase();
        if pressed {
            if self.held_controls.contains(&key) {
                return None;
            }
            self.held_controls.push(key);
        } else {
            let index = self.held_controls.iter().position(|held| held == &key)?;
            self.held_controls.remove(index);
        }
        self.reduce_controls()
    }
    fn reduce_controls(&mut self) -> Option<EntityAction> {
        let mut action = self.controls.release_all();
        for key in &self.held_controls {
            action = self
                .controls
                .user_event("key_down".into(), Value::Str(key.clone()));
        }
        if action == self.last_control_action {
            return None;
        }
        self.last_control_action = action.clone();
        Some(action)
    }
    pub fn cancel_controls(&mut self) -> EntityAction {
        self.held_controls.clear();
        self.last_control_action = EntityAction::Off;
        self.controls.release_all()
    }
    /// A camera convention change re-reduces the same held sources without
    /// inventing presses, so an isometric diagonal cannot remain a cardinal.
    pub fn set_isometric_controls(&mut self, isometric: bool) -> Option<EntityAction> {
        if self.isometric_controls == isometric {
            return None;
        }
        self.isometric_controls = isometric;
        self.controls.set_isometric(isometric);
        self.reduce_controls()
    }
}

pub fn is_movement_state(action: &EntityAction) -> bool {
    matches!(
        action,
        EntityAction::Off
            | EntityAction::Left
            | EntityAction::Right
            | EntityAction::Forward
            | EntityAction::Backward
            | EntityAction::StrafeLeft
            | EntityAction::StrafeRight
            | EntityAction::ForwardLeft
            | EntityAction::ForwardRight
            | EntityAction::BackwardLeft
            | EntityAction::BackwardRight
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cell_viewport_and_window_scale_are_separate_from_renderer_density() {
        let settings=PlayerSettings::parse("[viewport]\nwidth=40\nheight=24\nunit='cell'\ngrid_size=16\nwindow_scale=1.5\n[game]\ntarget_fps=60\nstart_screen='Title'\nplay_screen='World'\nauto_create_player=true").unwrap();
        assert_eq!(settings.viewport, (640, 384));
        assert_eq!(settings.window_size(), (960, 576));
        assert_eq!(settings.grid_size, 16.);
        assert_eq!(settings.target_fps, 60.);
        assert_eq!(settings.start_screen, "Title");
        assert!(settings.auto_create_player);
        assert!(PlayerSettings::parse("[viewport]\nwidth=1e100").is_err());
        assert!(PlayerSettings::parse("[viewport]\nwindow_scale=nan").is_err());
    }
}

#[cfg(test)]
#[test]
fn control_focus_release_keeps_bindings_and_does_not_emit_ui_or_intent_commands() {
    let mut state = PlayerSession {
        runtime: GameSession::new(Project::new()),
        settings: PlayerSettings::parse("").unwrap(),
        remote_mode: false,
        remote: None,
        remote_actions: VecDeque::new(),
        input: PlayerInput::default(),
    };
    state.input.controls.set_input_data(
        "[input]\nw='forward'\na='strafe_left'\ni='ui.inventory'\nf='intent.attack'",
    );
    assert_eq!(
        state.shortcut_labels_for_binding(&ClientCommandBinding::Ui("inventory".into())),
        vec!["I"]
    );
    assert_eq!(
        state.shortcut_labels_for_binding(&ClientCommandBinding::Intent("attack".into())),
        vec!["F"]
    );
    assert_eq!(
        state.control_key("w", false),
        None,
        "a release without an accepted press is not gameplay input"
    );
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("a", true), None);
    assert_eq!(state.control_key("f", true), None);
    assert_eq!(state.control_key("i", true), None);
    assert_eq!(state.cancel_controls(), EntityAction::Off);
    assert_eq!(state.control_key("a", false), None);
    assert_eq!(state.input_stats().held_controls, 0);
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("w", false), Some(EntityAction::Off));
    assert!(matches!(
        state.binding_for_key("i"),
        Some(ClientCommandBinding::Ui(_))
    ));
    for index in 0..256 {
        state
            .input
            .submit_action(
                &mut state.runtime.server,
                true,
                EntityAction::LookYaw(index as f32),
            )
            .unwrap();
    }
    assert!(
        state
            .input
            .submit_action(&mut state.runtime.server, true, EntityAction::LookYaw(999.))
            .is_err()
    );
    state
        .input
        .submit_action(&mut state.runtime.server, true, EntityAction::Off)
        .unwrap();
    assert!(
        state.input.pending_stop,
        "saturated startup input retains an authoritative stop"
    );
    assert_eq!(state.input.pending_action_count(), 257);
    assert_eq!(state.input.pending_actions.len(), 256);
    assert_eq!(
        state.input.pending_actions.front(),
        Some(&EntityAction::LookYaw(0.))
    );
    assert_eq!(
        state.input.pending_actions.back(),
        Some(&EntityAction::LookYaw(255.))
    );
}

#[cfg(test)]
#[test]
fn real_starter_defers_menu_registration_and_starts_once_with_shared_runtime() {
    use std::time::Instant;
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_projects/3DStarter.eldiron"),
    )
    .unwrap();
    let mut project = shared::project_io::decode_project(&bytes).unwrap();
    project.migrate_default_ruleset();
    let mut state = PlayerSession::new(project, false).unwrap();
    state.runtime.server.print_log_messages = false;
    state.settings.auto_create_player = true;
    state.settings.start_screen = "Setup".into();
    state.settings.play_screen = "Play".into();
    assert!(state.auto_start(false).unwrap().player.is_none());
    assert!(!state.runtime.has_started_player());
    // A real menu can remain open through many updates. Live region occupants
    // replace the render projection, while startup placement remains available.
    for _ in 0..4 {
        state.runtime.tick_once();
        state.advance(Duration::ZERO);
    }
    // This 3D fixture has no NPC publication to replace its initial entity
    // projection. Supply one through the real CPU pump, as Hideout2D does while
    // its creation menu remains open, so the authored template cannot mask it.
    let map_id = state.runtime.view().map.unwrap().id;
    let region = state.runtime.server.region_id_map[&map_id];
    let mut npc = rusterix::Entity::default();
    npc.id = 1_000_000;
    state.runtime.server.entities.insert(region, vec![npc]);
    state.runtime.poll();
    assert!(
        state
            .runtime
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .all(|entity| !entity.is_player())
    );
    let mut bindings = FxHashMap::default();
    bindings.insert("start.name".into(), "Clæria 東京".into());
    let transition = state.game_command("start", &mut bindings).unwrap();
    assert!(
        transition
            .player
            .as_ref()
            .is_some_and(|p| !p.template.is_empty())
    );
    assert_eq!(transition.screen.as_deref(), Some("Play"));
    state.submit_action(EntityAction::Forward).unwrap();
    state.submit_action(EntityAction::Off).unwrap();
    state.submit_action(EntityAction::LookYaw(0.5)).unwrap();
    assert_eq!(state.pending_action_count(), 2);
    assert_eq!(
        state.input.pending_actions.front(),
        Some(&EntityAction::Off)
    );

    assert!(
        state
            .game_command("start", &mut bindings)
            .unwrap()
            .player
            .is_none()
    );
    assert!(state.auto_start(true).unwrap().player.is_none());
    assert!(state.game_command("invalid", &mut bindings).is_err());
    assert_eq!(state.screen_command("goto.Inventory").unwrap(), "Inventory");
    assert!(state.screen_command("invalid").is_err());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        state.runtime.tick_once();
        state.advance(Duration::ZERO);
        if state.runtime.view().map.is_some_and(|m| {
            m.entities
                .iter()
                .any(|e| e.is_player() && e.attributes.get_str("name") == Some("Clæria 東京"))
        }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "player did not become visible: {}",
            state.runtime.server.log
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(state.runtime.has_started_player());
    assert!(state.runtime.server.has_local_player());
    assert_eq!(state.pending_action_count(), 0);
    assert!(
        state
            .runtime
            .assets
            .maps
            .contains_key(&state.runtime.current_map)
    );
    state.runtime.stop();
}

#[cfg(test)]
#[test]
fn physical_control_aliases_keep_other_sources_held_and_restore_remaining_press_order() {
    let mut state = PlayerSession {
        runtime: GameSession::new(Project::new()),
        settings: PlayerSettings::parse("").unwrap(),
        remote_mode: false,
        remote: None,
        remote_actions: VecDeque::new(),
        input: PlayerInput::default(),
    };
    state
        .input
        .controls
        .set_input_data("[input]\nw='forward'\nup='forward'\ns='backward'\na='left'\nspace='none'");
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("up", true), None);
    assert_eq!(state.control_key("w", false), None);
    assert_eq!(state.input_stats().held_controls, 1);
    assert_eq!(state.control_key("up", false), Some(EntityAction::Off));
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("s", true), Some(EntityAction::Backward));
    assert_eq!(state.control_key("up", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("up", false), Some(EntityAction::Backward));
    assert_eq!(state.control_key("s", false), Some(EntityAction::Forward));
    assert_eq!(
        state.control_key("w", true),
        None,
        "duplicate press is not a new source"
    );
    assert_eq!(state.control_key("a", true), Some(EntityAction::Left));
    assert_eq!(
        state.set_isometric_controls(true),
        Some(EntityAction::ForwardLeft)
    );
    assert_eq!(state.control_key("a", false), Some(EntityAction::Forward));
    assert_eq!(state.set_isometric_controls(false), None);
    assert_eq!(state.control_key("space", true), Some(EntityAction::Off));
    assert_eq!(state.input_stats().held_controls, 0);
    assert_eq!(state.control_key("w", false), None);
    assert_eq!(state.control_key("space", false), None);
}
