use crate::Embedded;
use crate::prelude::*;
use instant::{Duration, Instant};
use rusterix::server::message::AudioCommand;
use rusterix::{EntityAction, Rusterix, Value};
use shared::{
    iso_paint_render::{IsoPaintRenderCache, IsoPaintRenderer},
    project::Project,
    rusterix_utils::*,
};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

#[derive(Default)]
struct FrameTimingStats {
    window_started: Option<Instant>,
    frames: u32,
    frame_time: Duration,
    input_time: Duration,
    server_time: Duration,
    update_time: Duration,
    sync_time: Duration,
    render_time: Duration,
    present_time: Duration,
    max_frame_time: Duration,
    max_input_time: Duration,
    max_render_time: Duration,
}

#[derive(Default)]
struct FrameTimingSample {
    frame: Duration,
    input: Duration,
    server: Duration,
    update: Duration,
    sync: Duration,
    render: Duration,
    present: Duration,
}

impl FrameTimingStats {
    fn record(&mut self, enabled: bool, sample: FrameTimingSample) {
        if !enabled {
            *self = Self::default();
            return;
        }

        let now = Instant::now();
        let window_started = *self.window_started.get_or_insert(now);
        self.frames = self.frames.saturating_add(1);
        self.frame_time += sample.frame;
        self.input_time += sample.input;
        self.server_time += sample.server;
        self.update_time += sample.update;
        self.sync_time += sample.sync;
        self.render_time += sample.render;
        self.present_time += sample.present;
        self.max_frame_time = self.max_frame_time.max(sample.frame);
        self.max_input_time = self.max_input_time.max(sample.input);
        self.max_render_time = self.max_render_time.max(sample.render);

        let elapsed = now.saturating_duration_since(window_started);
        if elapsed < Duration::from_secs(2) {
            return;
        }

        let frames = self.frames.max(1) as f64;
        let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
        eprintln!(
            "[RenderDebug][ClientFrame] fps={:.1} frames={} avg_ms frame={:.2} input={:.2} server={:.2} update={:.2} sync={:.2} render={:.2} present={:.2} max_ms frame={:.2} input={:.2} render={:.2}",
            frames / elapsed.as_secs_f64().max(f64::EPSILON),
            self.frames,
            ms(self.frame_time) / frames,
            ms(self.input_time) / frames,
            ms(self.server_time) / frames,
            ms(self.update_time) / frames,
            ms(self.sync_time) / frames,
            ms(self.render_time) / frames,
            ms(self.present_time) / frames,
            ms(self.max_frame_time),
            ms(self.max_input_time),
            ms(self.max_render_time),
        );
        *self = Self {
            window_started: Some(now),
            ..Self::default()
        };
    }
}

pub struct Client {
    name: String,
    project: Project,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,

    rusterix: Rusterix,
    iso_paint_overlay_cache: IsoPaintRenderCache,
    cmd_line_path: Option<PathBuf>,
    frame_timing_stats: FrameTimingStats,
}

impl Client {
    fn process_pending_events(&mut self) {
        let mut events = Vec::new();
        if let Some(receiver) = &mut self.event_receiver {
            events.extend(receiver.try_iter());
        }

        // A slow frame can queue many pointer moves. Preserve ordering while
        // reducing each consecutive run to its newest coordinate.
        let mut coalesced = Vec::with_capacity(events.len());
        let mut pending_hover = None;
        for event in events {
            if let TheEvent::Hover(coord) = event {
                pending_hover = Some(coord);
            } else {
                if let Some(coord) = pending_hover.take() {
                    coalesced.push(TheEvent::Hover(coord));
                }
                coalesced.push(event);
            }
        }
        if let Some(coord) = pending_hover {
            coalesced.push(TheEvent::Hover(coord));
        }

        for event in coalesced {
            match event {
                TheEvent::Resize => {}
                TheEvent::MouseDown(coord) => {
                    for r in &mut self.project.regions {
                        self.rusterix.server.apply_entities_items(&mut r.map);

                        if r.map.name == self.rusterix.client.current_map {
                            if let Some(action) = self.rusterix.client_touch_down(coord, &r.map) {
                                self.rusterix.server.local_player_action(action);
                            }
                        }
                    }
                }
                TheEvent::MouseUp(coord) => {
                    for r in &mut self.project.regions {
                        self.rusterix.server.apply_entities_items(&mut r.map);

                        if r.map.name == self.rusterix.client.current_map {
                            if let Some(action) = self.rusterix.client_touch_up(coord, &r.map) {
                                self.rusterix.server.local_player_action(action);
                            }
                            self.rusterix.server.local_player_action(EntityAction::Off);
                        }
                    }
                }
                TheEvent::Hover(coord) => {
                    for r in &mut self.project.regions {
                        if r.map.name == self.rusterix.client.current_map {
                            self.rusterix.client_touch_hover(coord, &r.map);
                        }
                    }
                }
                TheEvent::KeyDown(v) => {
                    let key = if let Some(char) = v.to_char() {
                        Some(char.to_string())
                    } else {
                        v.to_key_code().and_then(|code| match code {
                            TheKeyCode::Return => Some("enter".to_string()),
                            TheKeyCode::Delete => Some("backspace".to_string()),
                            TheKeyCode::Escape => Some("escape".to_string()),
                            TheKeyCode::Space => Some("space".to_string()),
                            _ => None,
                        })
                    };
                    if let Some(key) = key {
                        let action = self
                            .rusterix
                            .client
                            .user_event("key_down".into(), Value::Str(key));

                        self.rusterix.server.local_player_action(action);
                    }
                }
                TheEvent::KeyCodeDown(v) => {
                    let key = v.to_key_code().and_then(|code| match code {
                        TheKeyCode::Return => Some("enter".to_string()),
                        TheKeyCode::Delete => Some("backspace".to_string()),
                        TheKeyCode::Escape => Some("escape".to_string()),
                        TheKeyCode::Space => Some("space".to_string()),
                        _ => None,
                    });
                    if let Some(key) = key {
                        let action = self
                            .rusterix
                            .client
                            .user_event("key_down".into(), Value::Str(key));

                        self.rusterix.server.local_player_action(action);
                    }
                }
                TheEvent::KeyUp(v) => {
                    if let Some(char) = v.to_char() {
                        let action = self
                            .rusterix
                            .client
                            .user_event("key_up".into(), Value::Str(char.to_string()));

                        self.rusterix.server.local_player_action(action);
                    }
                }
                _ => {}
            }
        }
    }
}

impl TheTrait for Client {
    fn new() -> Self
    where
        Self: Sized,
    {
        let game_name = "Eldiron Adventure";
        let project = Project::default();
        let rusterix = Rusterix::default();

        Self {
            name: game_name.into(),
            project,

            update_tracker: UpdateTracker::new(),
            event_receiver: None,

            rusterix,
            iso_paint_overlay_cache: IsoPaintRenderCache::default(),
            cmd_line_path: None,
            frame_timing_stats: FrameTimingStats::default(),
        }
    }

    /// Set the command line arguments
    fn set_cmd_line_args_early(&mut self, args: Vec<String>) {
        // Assign the path
        if args.len() > 1 {
            #[allow(irrefutable_let_patterns)]
            if let Ok(path) = PathBuf::from_str(&args[1]) {
                self.cmd_line_path = Some(path);
            }
        }

        // Load the game data path
        if let Some(path) = self.get_data_path() {
            let mut project = self.load_project(path);
            self.rusterix.assets.ruleset_palette = project.palette.clone();
            self.rusterix.assets.palette = project.art_palette.clone();
            self.rusterix.assets.palette_materials = project
                .art_palette_materials
                .iter()
                .map(|m| m.rmoe_values())
                .collect();
            self.rusterix.assets.palette_material_ids = project
                .art_palette_materials
                .iter()
                .map(|m| m.material_id())
                .collect();
            self.rusterix.set_tiles_for_maps(
                project.tiles.clone(),
                false,
                project.regions.iter().map(|region| &region.map),
            );

            // Init server / client

            start_server(&mut self.rusterix, &mut project, false);
            self.rusterix.clear_say_messages();
            let commands = setup_client(&mut self.rusterix, &mut project);
            self.rusterix.server.process_client_commands(commands);
            warmup_runtime(&mut self.rusterix, &mut project, 3);
            self.rusterix.client.server_time = project.time;
            self.project = project;

            println!("Project loaded successfully");
        } else {
            panic!("No data file!");
        }
    }

    fn default_window_size(&self) -> (usize, usize) {
        let scale = self.rusterix.client.window_scale();
        (
            ((self.rusterix.client.viewport.x as f32 * scale)
                .round()
                .max(1.0)) as usize,
            ((self.rusterix.client.viewport.y as f32 * scale)
                .round()
                .max(1.0)) as usize,
        )
    }

    fn window_title(&self) -> String {
        self.name.clone()
    }

    fn window_icon(&self) -> Option<(Vec<u8>, u32, u32)> {
        if let Some(file) = Embedded::get("window_logo.png") {
            let data = std::io::Cursor::new(file.data);

            let decoder = png::Decoder::new(data);
            if let Ok(mut reader) = decoder.read_info() {
                let mut buf = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut buf).unwrap();
                let bytes = &buf[..info.buffer_size()];

                Some((bytes.to_vec(), info.width, info.height))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        ctx.set_cursor_visible(false);

        for file in Embedded::iter() {
            let name = file.as_ref();
            if name.ends_with(".png") {
                if let Some(file) = Embedded::get(name) {
                    let data = std::io::Cursor::new(file.data);

                    let decoder = png::Decoder::new(data);
                    if let Ok(mut reader) = decoder.read_info() {
                        let mut buf = vec![0; reader.output_buffer_size()];
                        let info = reader.next_frame(&mut buf).unwrap();
                        let bytes = &buf[..info.buffer_size()];

                        let mut cut_name = name.replace("icons/", "");
                        cut_name = cut_name.replace(".png", "");

                        ctx.ui.add_icon(
                            cut_name.to_string(),
                            TheRGBABuffer::from(bytes.to_vec(), info.width, info.height),
                        );
                    }
                }
            }
        }

        // Set the server time
        for region in &self.project.regions {
            if region.name == self.rusterix.client.current_map {
                // println!("{}", self.project.time.to_time24());
                self.rusterix.server.set_time(&region.id, self.project.time);
            }
            break;
        }

        // -
        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let frame_started = Instant::now();
        let game_tick_ms = self.rusterix.client.game_tick_ms.max(1) as u64;
        let tick_period = Duration::from_millis(game_tick_ms);
        let tick_update = self.update_tracker.update(tick_period);
        let redraw = true;

        // Read the newest pointer state before potentially expensive server and
        // renderer work, reducing input-to-frame latency.
        let input_started = Instant::now();
        self.process_pending_events();
        let input_time = input_started.elapsed();

        let server_started = Instant::now();
        if tick_update {
            self.rusterix.client.inc_animation_frame();
            self.rusterix.server.system_tick();
        }

        self.rusterix.server.redraw_tick();
        let server_time = server_started.elapsed();

        let update_started = Instant::now();
        if let Some(new_region_name) = self.rusterix.update_server() {
            self.rusterix.client.current_map = new_region_name;
        }
        let update_time = update_started.elapsed();

        let canvas_dim = *ui.canvas.buffer.dim();
        self.rusterix.resize_client_surface((
            canvas_dim.width.max(1) as u32,
            canvas_dim.height.max(1) as u32,
        ));

        let mut sync_time = Duration::ZERO;
        let mut render_time = Duration::ZERO;
        let mut present_time = Duration::ZERO;
        for r in &mut self.project.regions {
            let sync_started = Instant::now();
            self.rusterix.server.apply_entities_items(&mut r.map);

            if r.map.name == self.rusterix.client.current_map {
                if let Some(time) = self.rusterix.server.get_time(&r.map.id) {
                    self.rusterix.client.set_server_time(time);
                }

                let messages = self.rusterix.server.get_messages(&r.map.id);
                let says = self.rusterix.server.get_says(&r.map.id);
                let choices = self.rusterix.server.get_choices(&r.map.id);
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
                let mut iso_paint = r.iso_paint.clone();
                sync_time += sync_started.elapsed();

                let render_started = Instant::now();
                self.rusterix.draw_game_with_widget_overlays(
                    &r.map,
                    messages,
                    says,
                    choices,
                    |widget, scene_handler| {
                        let render_dim = widget.render_surface_dim();
                        if render_dim.width <= 0 || render_dim.height <= 0 {
                            return false;
                        }
                        let camera = widget.camera_d3.as_scenevm_camera_for_surface(
                            render_dim.width as f32,
                            render_dim.height as f32,
                        );
                        let active_vm = scene_handler.vm.active_vm_index();
                        scene_handler.vm.set_active_vm(0);
                        let view = widget.camera_d3.view_matrix_for_surface(
                            render_dim.width as f32,
                            render_dim.height as f32,
                        );
                        let render_proj = widget
                            .camera_d3
                            .projection_matrix(render_dim.width as f32, render_dim.height as f32);
                        let camera_scale = Some(widget.camera_d3.scale());
                        IsoPaintRenderer::upload_overlay_cached(
                            &mut self.iso_paint_overlay_cache,
                            r.id,
                            0,
                            &mut iso_paint,
                            &mut scene_handler.vm,
                            camera,
                            view,
                            render_proj,
                            render_dim.width as u32,
                            render_dim.height as u32,
                            camera_scale,
                        );
                        scene_handler.vm.set_active_vm(active_vm);
                        false
                    },
                    |_, _| {},
                );
                render_time += render_started.elapsed();

                let present_started = Instant::now();
                self.rusterix
                    .client
                    .insert_game_buffer(&mut ui.canvas.buffer);
                present_time += present_started.elapsed();
                break;
            }
            sync_time += sync_started.elapsed();
        }

        let timings_enabled = self.rusterix.client.frame_timings_enabled();
        self.frame_timing_stats.record(
            timings_enabled,
            FrameTimingSample {
                frame: frame_started.elapsed(),
                input: input_time,
                server: server_time,
                update: update_time,
                sync: sync_time,
                render: render_time,
                present: present_time,
            },
        );

        redraw
    }

    fn touch_dragged(&mut self, x: f32, y: f32, _ctx: &mut TheContext) -> bool {
        let coord = Vec2::new(x as i32, y as i32);
        for r in &mut self.project.regions {
            if r.map.name == self.rusterix.client.current_map {
                self.rusterix.client_touch_dragged(coord, &r.map);
            }
        }
        true
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), _ctx: &mut TheContext) -> bool {
        self.rusterix.client.scroll_messages(delta.1)
    }

    fn hover(&mut self, x: f32, y: f32, _ctx: &mut TheContext) -> bool {
        let coord = Vec2::new(x as i32, y as i32);
        for r in &mut self.project.regions {
            if r.map.name == self.rusterix.client.current_map {
                self.rusterix.client_touch_hover(coord, &r.map);
            }
        }
        true
    }

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        self.rusterix.client.hover_tooltip_pending()
    }

    fn target_fps(&self) -> f64 {
        60.0
    }
}

pub trait ClientTrait {
    fn get_data_path(&self) -> Option<PathBuf>;
    fn load_project(&mut self, path: PathBuf) -> Project;
}

impl ClientTrait for Client {
    /// Returns the path to the game data
    fn get_data_path(&self) -> Option<PathBuf> {
        #[cfg(target_arch = "wasm32")]
        {
            Some(PathBuf::new())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(clp) = self.cmd_line_path.clone() {
                return Some(clp);
            }

            None
        }
    }

    /// Load project
    fn load_project(&mut self, path: PathBuf) -> Project {
        // On WASM, do a network request to "game.eldiron" in the same dir as the served page
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::XmlHttpRequest;
            let _ = path;

            // Try to fetch from the same directory as the served page.
            let xhr = XmlHttpRequest::new().expect("XmlHttpRequest not available");
            xhr.open_with_async("GET", "game.eldiron", false)
                .expect("failed to open XHR");
            xhr.send().expect("failed to send XHR");

            // 200..299 considered success
            let status = xhr.status().unwrap_or(0);
            if (200..300).contains(&status) {
                if let Ok(Some(text)) = xhr.response_text() {
                    if let Ok(mut project) = serde_json::from_str::<Project>(&text) {
                        project.migrate_default_ruleset();
                        let _ = project.sync_ruleset_items();
                        return project;
                    }
                }
            }

            return Project::default();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(contents) = std::fs::read_to_string(path) {
                if let Ok(mut project) = serde_json::from_str::<Project>(&contents) {
                    project.migrate_default_ruleset();
                    let _ = project.sync_ruleset_items();
                    return project;
                }
            }
            Project::default()
        }
    }
}
