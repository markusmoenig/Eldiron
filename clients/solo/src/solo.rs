use crate::Embedded;
use crate::prelude::*;
use std::sync::mpsc::Receiver;
//use std::sync::Mutex;
use rusterix::{EntityAction, Rusterix, Value};
use shared::{project::Project, rusterix_utils::*};

pub struct Solo {
    name: String,
    project: Project,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,

    rusterix: Rusterix,
}

impl TheTrait for Solo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut game_name = "Eldiron Solo Adventure";
        let mut project = Project::default();
        let mut rusterix = Rusterix::default();
        let mut cloned;

        for file in Embedded::iter() {
            let name = file.as_ref().to_string();
            cloned = name.clone();

            // Get the embedded project
            if name.ends_with(".eldiron") {
                game_name = cloned.split(".").next().unwrap_or_default();
                if let Some(file) = Embedded::get(&name) {
                    if let Ok(str_slice) = std::str::from_utf8(&file.data) {
                        let json = str_slice.to_string();
                        let emd_project: Option<Project> = serde_json::from_str(&json).ok();
                        if let Some(emd_project) = emd_project {
                            project = emd_project;

                            let tiles = project.extract_tiles();
                            rusterix.assets.set_rgba_tiles(tiles.clone());

                            // Init server / client

                            start_server(&mut rusterix, &mut project);
                            let commands = setup_client(&mut rusterix, &mut project);
                            rusterix.server.process_client_commands(commands);
                            rusterix.client.server_time = project.time;

                            println!("Project loaded successfully ({name}).");
                        } else {
                            println!("Failed to load project ({name}).");
                        }
                    }
                }
                break;
            }
        }

        Self {
            name: game_name.into(),
            project,

            update_tracker: UpdateTracker::new(),
            event_receiver: None,

            rusterix,
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

        // -
        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        let (redraw_update, tick_update) = self.update_tracker.update(
            (1000 / self.rusterix.client.target_fps) as u64,
            self.rusterix.client.game_tick_ms as u64,
        );

        if tick_update {
            self.rusterix.client.inc_animation_frame();
        }

        if redraw_update {
            redraw = true;

            for r in &mut self.project.regions {
                self.rusterix.server.apply_entities_items(&mut r.map);

                if r.map.name == self.rusterix.client.current_map {
                    self.rusterix.update_server();

                    if let Some(time) = self.rusterix.server.get_time(&r.map.id) {
                        self.rusterix.client.server_time = time;
                    }

                    rusterix::tile_builder(&mut r.map, &mut self.rusterix.assets);
                    let messages = self.rusterix.server.get_messages(&r.map.id);
                    self.rusterix.draw_game(&r.map, messages);
                    self.rusterix
                        .client
                        .insert_game_buffer(&mut ui.canvas.buffer);
                    break;
                }
            }
        }

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                //println!("Event received {:?}", event);
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

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }
}

// pub trait SoloTrait {
//fn update_server_state_icons(&mut self, ui: &mut TheUI);
// }

//impl SoloTrait for Solo {}
