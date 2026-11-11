//! Shared graphical-player application orchestration. The authoritative runtime
//! remains in GameSession; surface, fonts, screen layouts and GPU resources are
//! assembled by the graphical host, including Creator Play and foreign hosts.
use eldiron_runtime::{
    GameSession, PlayerStartInfo, PlayerStartOptions, Project, SessionOptions, SessionUpdate,
};
use rustc_hash::FxHashMap;
use rusterix::{
    EntityAction, Value,
    client::{action::ClientAction, command::ClientCommandBinding},
};
use std::time::Duration;
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
pub struct PlayerSession {
    pub runtime: GameSession,
    pub settings: PlayerSettings,
    controls: ClientAction,
}
impl PlayerSession {
    pub fn new(project: Project, debug: bool) -> Result<Self, String> {
        let settings = PlayerSettings::parse(&project.config)?;
        let mut runtime = GameSession::new(project);
        // The application decides whether a start menu defers registration.
        // Server/assets/clock orchestration itself is the same as the terminal.
        runtime.start(SessionOptions {
            debug,
            create_local_player: false,
            ..Default::default()
        })?;
        Ok(Self {
            runtime,
            settings,
            controls: ClientAction::default(),
        })
    }
    pub fn auto_start(&mut self, showing_game: bool) -> Result<PlayerTransition, String> {
        if self.settings.auto_create_player
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
            self.controls
                .init(info.template.clone(), &self.runtime.assets);
            if let Some(input) = &info.input_bindings {
                self.controls.set_input_data(input);
            }
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
        self.runtime.advance(elapsed)
    }
    pub fn binding_for_key(&self, key: &str) -> Option<ClientCommandBinding> {
        self.controls.binding_for_key(key)
    }
    /// Movement aggregation stays shared with the existing client input model.
    /// Non-control bindings are dispatched through the graphical screen/HUD.
    pub fn control_key(&mut self, key: &str, pressed: bool) -> Option<EntityAction> {
        if !matches!(
            self.controls.binding_for_key(key),
            Some(ClientCommandBinding::Control(_))
        ) {
            return None;
        }
        Some(self.controls.user_event(
            if pressed { "key_down" } else { "key_up" }.into(),
            Value::Str(key.into()),
        ))
    }
    pub fn cancel_controls(&mut self) -> EntityAction {
        self.controls.release_all()
    }
    pub fn set_isometric_controls(&mut self, isometric: bool) {
        self.controls.set_isometric(isometric);
    }
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
        controls: ClientAction::default(),
    };
    state.controls.set_input_data(
        "[input]\nw='forward'\na='strafe_left'\ni='ui.inventory'\nf='intent.attack'",
    );
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("a", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("f", true), None);
    assert_eq!(state.control_key("i", true), None);
    assert_eq!(state.cancel_controls(), EntityAction::Off);
    assert_eq!(state.control_key("a", false), Some(EntityAction::Off));
    assert_eq!(state.control_key("w", true), Some(EntityAction::Forward));
    assert_eq!(state.control_key("w", false), Some(EntityAction::Off));
    assert!(matches!(
        state.binding_for_key("i"),
        Some(ClientCommandBinding::Ui(_))
    ));
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
    assert!(
        state
            .runtime
            .assets
            .maps
            .contains_key(&state.runtime.current_map)
    );
    state.runtime.stop();
}
