use crate::misc::UpdateTracker;
use instant::Duration;
use rusterix::server::message::AudioCommand;
use rusterix::{EntityAction, MultipleChoice, Rusterix, Value, server::Message};
use scenevm::{Atom, SceneVM, SceneVMApp, SceneVMRenderCtx};
use shared::{project::Project, rusterix_utils::*};
use std::path::PathBuf;
use toml::Table;
use vek::Vec2;

pub struct EldironPlayerApp {
    pub name: String,
    pub project: Project,
    pub rusterix: Rusterix,
    pub data_path: Option<PathBuf>,
    pub update_tracker: UpdateTracker,
    pub pointer_down: bool,
    pub initialized: bool,
    pub scenevm_bootstrapped: bool,
    pub pending_messages: Vec<Message>,
    pub pending_choices: Vec<MultipleChoice>,
    pub ui_overlay_pixels: Vec<u8>,
    pub ui_overlay_size: (u32, u32),
    pub ui_overlay_display_rect: [f32; 4],
    pub window_scale: f32,
}

impl Default for EldironPlayerApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl EldironPlayerApp {
    pub fn new(data_path: Option<PathBuf>) -> Self {
        Self {
            name: "Eldiron Adventure (SceneVM)".into(),
            project: Project::default(),
            rusterix: Rusterix::default(),
            data_path,
            update_tracker: UpdateTracker::new(),
            pointer_down: false,
            initialized: false,
            scenevm_bootstrapped: false,
            pending_messages: Vec::new(),
            pending_choices: Vec::new(),
            ui_overlay_pixels: Vec::new(),
            ui_overlay_size: (0, 0),
            ui_overlay_display_rect: [0.0, 0.0, 0.0, 0.0],
            window_scale: 1.0,
        }
    }

    pub fn from_args(args: Vec<String>) -> Self {
        if args.len() > 1 {
            let path = PathBuf::from(&args[1]);
            return Self::new(Some(path));
        }
        Self::default()
    }

    fn resolve_data_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.data_path {
            return Some(path.clone());
        }
        Some(PathBuf::from("game.eldiron"))
    }

    fn load_project(&self, path: PathBuf) -> Option<Project> {
        if let Ok(contents) = std::fs::read_to_string(path)
            && let Ok(project) = serde_json::from_str::<Project>(&contents)
        {
            return Some(project);
        }
        None
    }

    fn initial_window_size_from_project(&self) -> Option<(u32, u32)> {
        let path = self.resolve_data_path()?;
        let project = self.load_project(path)?;
        let cfg = project.config.parse::<Table>().ok()?;
        let viewport = cfg.get("viewport")?.as_table()?;

        let w = viewport.get("width")?.as_integer()? as f32;
        let h = viewport.get("height")?.as_integer()? as f32;
        let scale = viewport
            .get("window_scale")
            .and_then(toml::Value::as_float)
            .map(|v| v as f32)
            .or_else(|| {
                viewport
                    .get("window_scale")
                    .and_then(toml::Value::as_integer)
                    .map(|v| v as f32)
            })
            .unwrap_or(1.0)
            .max(0.1);

        let ww = (w * scale).round().max(1.0) as u32;
        let wh = (h * scale).round().max(1.0) as u32;
        Some((ww, wh))
    }

    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        let Some(path) = self.resolve_data_path() else {
            eprintln!("No data file path provided.");
            return;
        };
        let Some(mut project) = self.load_project(path) else {
            eprintln!("Failed to load project file.");
            return;
        };

        self.rusterix.set_tiles(project.tiles.clone(), false);
        start_server(&mut self.rusterix, &mut project, false);
        self.rusterix.clear_say_messages();
        let commands = setup_client(&mut self.rusterix, &mut project);
        self.rusterix.server.process_client_commands(commands);
        self.rusterix.client.server_time = project.time;
        self.project = project;
        self.initialized = true;
        self.scenevm_bootstrapped = false;
        self.pending_messages.clear();
        self.pending_choices.clear();
        self.ui_overlay_pixels.clear();
        self.ui_overlay_size = (0, 0);
        self.ui_overlay_display_rect = [0.0, 0.0, 0.0, 0.0];
    }

    fn update_game_state(&mut self) {
        let game_tick_ms = self.rusterix.client.game_tick_ms.max(1) as u64;
        let tick_period = Duration::from_millis(game_tick_ms);
        let tick_update = self.update_tracker.update(tick_period);

        if tick_update {
            self.rusterix.client.inc_animation_frame();
            self.rusterix.server.system_tick();
        }

        self.rusterix.server.redraw_tick();

        let current_map = self.rusterix.client.current_map.clone();
        for r in &mut self.project.regions {
            self.rusterix.server.apply_entities_items(&mut r.map);
            if r.map.name != current_map {
                continue;
            }

            if let Some(new_region_name) = self.rusterix.update_server() {
                self.rusterix.client.current_map = new_region_name;
            }
            if let Some(time) = self.rusterix.server.get_time(&r.map.id) {
                self.rusterix.client.set_server_time(time);
            }

            rusterix::tile_builder(&mut r.map, &mut self.rusterix.assets);
            let says = self.rusterix.server.get_says(&r.map.id);
            self.rusterix.client.process_messages(&r.map, says);
            self.pending_messages = self.rusterix.server.get_messages(&r.map.id);
            self.pending_choices = self.rusterix.server.get_choices(&r.map.id);

            for cmd in self.rusterix.server.get_audio_commands(&r.map.id) {
                match cmd {
                    AudioCommand::Play {
                        name,
                        bus,
                        gain,
                        looping,
                    } => {
                        self.rusterix.play_audio_on_bus(&name, &bus, gain, looping);
                    }
                    AudioCommand::ClearBus { bus } => {
                        self.rusterix.clear_audio_bus(&bus);
                    }
                    AudioCommand::ClearAll => {
                        self.rusterix.clear_all_audio();
                    }
                    AudioCommand::SetBusVolume { bus, volume } => {
                        self.rusterix.set_audio_bus_volume(&bus, volume);
                    }
                }
            }
            break;
        }
    }

    fn current_region_index(&self) -> Option<usize> {
        let current = &self.rusterix.client.current_map;
        self.project
            .regions
            .iter()
            .position(|r| r.map.name == *current)
    }
}

impl SceneVMApp for EldironPlayerApp {
    fn initial_window_size(&self) -> Option<(u32, u32)> {
        if let Some(size) = self.initial_window_size_from_project() {
            return Some(size);
        }
        let w = self.rusterix.client.viewport.x;
        let h = self.rusterix.client.viewport.y;
        if w <= 1 || h <= 1 {
            Some((1280, 720))
        } else {
            Some((w as u32, h as u32))
        }
    }

    fn window_title(&self) -> Option<String> {
        Some(self.name.clone())
    }

    fn target_fps(&self) -> Option<f32> {
        // Keep GPU load close to the classic client behavior.
        Some(30.0)
    }

    fn init(&mut self, _vm: &mut SceneVM, _size: (u32, u32)) {
        self.initialize();
    }

    fn needs_update(&mut self, _vm: &SceneVM) -> bool {
        true
    }

    fn update(&mut self, _vm: &mut SceneVM) {
        if !self.initialized {
            self.initialize();
        }
        if self.initialized {
            self.update_game_state();
        }
    }

    fn set_scale(&mut self, scale: f32) {
        self.window_scale = scale.max(0.0001);
    }

    fn render(&mut self, _vm: &mut SceneVM, ctx: &mut dyn SceneVMRenderCtx) {
        if !self.initialized {
            return;
        }
        // Use the runner-provided SceneVM (which owns the window surface) for this frame.
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);

        // One-time bootstrap on the active window-backed VM:
        // upload atlas/tiles so static chunk geometry can render.
        if !self.scenevm_bootstrapped {
            self.rusterix.set_tiles(self.project.tiles.clone(), false);
            self.rusterix.scene_handler.mark_dynamics_dirty();
            self.scenevm_bootstrapped = true;
        }

        let size = ctx.size();
        let current_map = self.rusterix.client.current_map.clone();
        if let Some(region_index) = self
            .project
            .regions
            .iter()
            .position(|r| r.map.name == current_map)
        {
            let prepared = {
                let map = &self.project.regions[region_index].map;
                self.rusterix.prepare_game_scene_for_present(map, size)
            };

            if prepared {
                let viewport_size = (
                    self.rusterix.client.viewport.x.max(1) as u32,
                    self.rusterix.client.viewport.y.max(1) as u32,
                );
                let (scale, offset_x, offset_y) =
                    self.rusterix.presentation_transform_for_surface(size);
                let game_rect = self.rusterix.game_widget_rect().unwrap_or_else(|| {
                    rusterix::Rect::new(0.0, 0.0, viewport_size.0 as f32, viewport_size.1 as f32)
                });
                let mapped_game_rect = [
                    (offset_x + game_rect.x * scale) * self.window_scale,
                    (offset_y + game_rect.y * scale) * self.window_scale,
                    (game_rect.width * scale * self.window_scale).max(1.0),
                    (game_rect.height * scale * self.window_scale).max(1.0),
                ];

                self.rusterix.scene_handler.vm.set_active_vm(0);
                self.rusterix
                    .scene_handler
                    .vm
                    .execute(Atom::SetViewportRect2D(Some(mapped_game_rect)));

                let messages = std::mem::take(&mut self.pending_messages);
                let choices = std::mem::take(&mut self.pending_choices);
                let overlay_size = viewport_size;
                {
                    let map = &self.project.regions[region_index].map;
                    let overlay = self.rusterix.draw_ui_overlay_only(
                        map,
                        messages,
                        choices,
                        overlay_size.0,
                        overlay_size.1,
                    );
                    let src = overlay.pixels();
                    self.ui_overlay_pixels.resize(src.len(), 0);
                    self.ui_overlay_pixels.copy_from_slice(src);
                }

                let display_rect = [
                    offset_x * self.window_scale,
                    offset_y * self.window_scale,
                    overlay_size.0 as f32 * scale * self.window_scale,
                    overlay_size.1 as f32 * scale * self.window_scale,
                ];
                self.rusterix.scene_handler.vm.set_rgba_overlay_bytes(
                    overlay_size.0,
                    overlay_size.1,
                    &self.ui_overlay_pixels,
                    display_rect,
                );
                self.ui_overlay_size = overlay_size;
                self.ui_overlay_display_rect = display_rect;

                let _ = ctx.present(&mut self.rusterix.scene_handler.vm);
            } else {
                self.rusterix.scene_handler.vm.clear_rgba_overlay();
            }
        } else {
            self.rusterix.scene_handler.vm.clear_rgba_overlay();
        }

        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
    }

    fn mouse_down(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
        self.pointer_down = true;
        self.rusterix.scene_handler.vm.set_active_vm(0);
        let coord = Vec2::new(x as i32, y as i32);
        if let Some(idx) = self.current_region_index() {
            let map = &mut self.project.regions[idx].map;
            self.rusterix.server.apply_entities_items(map);
            if let Some(action) = self.rusterix.client.touch_down(coord, map) {
                self.rusterix.server.local_player_action(action);
            }
        }
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
    }

    fn mouse_up(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
        self.pointer_down = false;
        self.rusterix.scene_handler.vm.set_active_vm(0);
        let coord = Vec2::new(x as i32, y as i32);
        if let Some(idx) = self.current_region_index() {
            let map = &mut self.project.regions[idx].map;
            self.rusterix.server.apply_entities_items(map);
            self.rusterix.client.touch_up(coord, map);
            self.rusterix.server.local_player_action(EntityAction::Off);
        }
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
    }

    fn mouse_move(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
        self.rusterix.scene_handler.vm.set_active_vm(0);
        let coord = Vec2::new(x as i32, y as i32);
        if let Some(idx) = self.current_region_index() {
            let map = &mut self.project.regions[idx].map;
            if self.pointer_down {
                self.rusterix.client_touch_dragged(coord, map);
            } else {
                self.rusterix.client_touch_hover(coord, map);
            }
        }
        std::mem::swap(_vm, &mut self.rusterix.scene_handler.vm);
    }

    fn scroll(&mut self, _vm: &mut SceneVM, _dx: f32, _dy: f32) {}

    fn key_down(&mut self, _vm: &mut SceneVM, key: &str) {
        on_key_event(self, key, true);
    }

    fn key_up(&mut self, _vm: &mut SceneVM, key: &str) {
        on_key_event(self, key, false);
    }

    fn pinch(&mut self, _vm: &mut SceneVM, _scale: f32, _center: (f32, f32)) {}
}

pub fn on_key_event(app: &mut EldironPlayerApp, key: &str, is_down: bool) {
    let event = if is_down { "key_down" } else { "key_up" };
    let action = app
        .rusterix
        .client
        .user_event(event.into(), Value::Str(key.to_string()));
    app.rusterix.server.local_player_action(action);
}
