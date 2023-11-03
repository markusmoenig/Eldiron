use crate::prelude::*;
use std::sync::mpsc::Receiver;

pub struct Editor {
    sidebar: Sidebar,
    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            sidebar: Sidebar::new(),
            event_receiver: None,
        }
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.sidebar.init_ui(ui, ctx);
        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                self.sidebar.handle_event(&event, ui, ctx);
                match event {
                    TheEvent::StateChanged(id, state) => {
                        // println!("app Widget State changed {:?}: {:?}", id, state);
                    }
                    TheEvent::FileRequesterResult(id, paths) => {
                        // println!("FileRequester Result {:?} {:?}", id, paths);
                    }
                    _ => {}
                }
            }
        }
        redraw
    }
}

pub trait EldironEditor {}

impl EldironEditor for Editor {}
