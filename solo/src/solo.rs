use crate::prelude::*;
use crate::Embedded;
use std::sync::mpsc::Receiver;
//use std::sync::Mutex;
use shared::project::Project;

pub struct Solo {
    project: Project,
    curr_screen: Uuid,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for Solo {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            project: Project::new(),
            curr_screen: Uuid::nil(),

            update_tracker: UpdateTracker::new(),
            event_receiver: None,
        }
    }

    fn default_window_size(&self) -> (usize, usize) {
        (1280, 720)
    }

    fn window_title(&self) -> String {
        "Eldiron Solo Adventure".to_string()
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

            // Get the embedded project
            if name.ends_with(".eldiron") {
                if let Some(file) = Embedded::get(name) {
                    if let Ok(str_slice) = std::str::from_utf8(&file.data) {
                        let json = str_slice.to_string();
                        let project: Option<Project> = serde_json::from_str(&json).ok();
                        if let Some(project) = project {
                            self.project = project;

                            // Init server / client

                            println!("Project loaded successfully ({}).", name);
                        } else {
                            println!("Failed to load project ({}).", name);
                        }
                    }
                }
            } else if name.ends_with(".png") {
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

        // ---

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        let (redraw_update, _tick_update) = self.update_tracker.update(
            (1000 / self.project.target_fps) as u64,
            self.project.tick_ms as u64,
        );

        // if tick_update

        if redraw_update {
            redraw = true;

            // Todo: Get the Screen ID from the Game settings
            // Right now we just take the first screen
            let screen_id;
            if let Some(screen) = self.project.screens.keys().next() {
                screen_id = *screen;
                self.curr_screen = screen_id;
            }

            //self.client.draw_screen(&screen_id, &mut ui.canvas.buffer);
        }

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                //println!("Event received {:?}", event);
                match event {
                    TheEvent::Resize => {}
                    TheEvent::MouseDown(_coord) => {
                        // self.client.touch_down(&self.curr_screen, coord);
                    }
                    TheEvent::MouseUp(_coord) => {
                        // self.client.touch_up(&self.curr_screen);
                    }
                    TheEvent::KeyDown(v) => {
                        if let Some(_c) = v.to_char() {
                            // self.client.key_down(&self.curr_screen, c);
                        }
                    }
                    _ => {}
                }
            }
        }

        redraw
    }
}

pub trait EldironEditor {
    //fn update_server_state_icons(&mut self, ui: &mut TheUI);
}

//impl EldironEditor for Solo {}
