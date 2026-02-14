use crate::Embedded;
use crate::prelude::*;
use rusterix::{EntityAction, Rusterix, Value};
use shared::{project::Project, rusterix_utils::*};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

pub struct Client {
    name: String,
    project: Project,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,

    rusterix: Rusterix,
    cmd_line_path: Option<PathBuf>,
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
            cmd_line_path: None,
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
            self.rusterix.set_tiles(project.tiles.clone(), false);

            // Init server / client

            start_server(&mut self.rusterix, &mut project, false);
            let commands = setup_client(&mut self.rusterix, &mut project);
            self.rusterix.server.process_client_commands(commands);
            self.rusterix.client.server_time = project.time;
            self.project = project;

            println!("Project loaded successfully");
        } else {
            panic!("No data file!");
        }
    }

    fn default_window_size(&self) -> (usize, usize) {
        (
            self.rusterix.client.viewport.x as usize,
            self.rusterix.client.viewport.y as usize,
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
        let mut redraw = false;

        let target_fps = self.rusterix.client.target_fps.max(1) as u64;
        let game_tick_ms = self.rusterix.client.game_tick_ms.max(1) as u64;
        let (redraw_update, tick_update) =
            self.update_tracker.update(1000 / target_fps, game_tick_ms);

        if tick_update {
            self.rusterix.client.inc_animation_frame();
            self.rusterix.server.system_tick();
        }

        if redraw_update {
            redraw = true;

            self.rusterix.server.redraw_tick();

            for r in &mut self.project.regions {
                self.rusterix.server.apply_entities_items(&mut r.map);

                if r.map.name == self.rusterix.client.current_map {
                    if let Some(new_region_name) = self.rusterix.update_server() {
                        self.rusterix.client.current_map = new_region_name;
                    }
                    if let Some(time) = self.rusterix.server.get_time(&r.map.id) {
                        self.rusterix.client.set_server_time(time);
                    }

                    rusterix::tile_builder(&mut r.map, &mut self.rusterix.assets);
                    let messages = self.rusterix.server.get_messages(&r.map.id);
                    let choices = self.rusterix.server.get_choices(&r.map.id);
                    self.rusterix.draw_game(&r.map, messages, choices);
                    self.rusterix
                        .client
                        .insert_game_buffer(&mut ui.canvas.buffer);
                    break;
                }
            }
        }

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                // println!("Event received {:?}", event);
                match event {
                    TheEvent::Resize => {}
                    TheEvent::MouseDown(coord) => {
                        for r in &mut self.project.regions {
                            self.rusterix.server.apply_entities_items(&mut r.map);

                            if r.map.name == self.rusterix.client.current_map {
                                if let Some(action) = self.rusterix.client.touch_down(coord, &r.map)
                                {
                                    self.rusterix.server.local_player_action(action);
                                }
                            }
                        }
                    }
                    TheEvent::MouseUp(coord) => {
                        for r in &mut self.project.regions {
                            self.rusterix.server.apply_entities_items(&mut r.map);

                            if r.map.name == self.rusterix.client.current_map {
                                self.rusterix.client.touch_up(coord, &r.map);
                                self.rusterix.server.local_player_action(EntityAction::Off);
                            }
                        }
                    }
                    TheEvent::KeyDown(v) => {
                        if let Some(char) = v.to_char() {
                            let action = self
                                .rusterix
                                .client
                                .user_event("key_down".into(), Value::Str(char.to_string()));

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
        true
    }
}

pub trait ClientTrait {
    fn get_data_path(&self) -> Option<PathBuf>;
    fn load_project(&mut self, path: PathBuf) -> Project;
}

impl ClientTrait for Client {
    /// Returns the path to the game data
    fn get_data_path(&self) -> Option<PathBuf> {
        // On WASM just return an empty path.
        #[cfg(target_arch = "wasm32")]
        return Some(PathBuf::new());

        // For now, return only the command line path
        // We will need to adjust this based on platform specific features
        // to hardcode the path
        if let Some(clp) = self.cmd_line_path.clone() {
            return Some(clp);
        }

        None
    }

    /// Load project
    fn load_project(&mut self, path: PathBuf) -> Project {
        // On WASM, do a network request to "game.eldiron" in the same dir as the served page
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use web_sys::XmlHttpRequest;

            // Try to fetch from the same directory as the served page.
            let xhr = XmlHttpRequest::new().expect("XmlHttpRequest not available");
            xhr.open_with_async("GET", "game.eldiron", false)
                .expect("failed to open XHR");
            xhr.send().expect("failed to send XHR");

            // 200..299 considered success
            let status = xhr.status().unwrap_or(0);
            if (200..300).contains(&status) {
                if let Ok(Some(text)) = xhr.response_text() {
                    if let Ok(project) = serde_json::from_str::<Project>(&text) {
                        return project;
                    }
                }
            }

            return Project::default();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(contents) = std::fs::read_to_string(path) {
                if let Ok(project) = serde_json::from_str::<Project>(&contents) {
                    return project;
                }
            }
            Project::default()
        }
    }
}
