use crate::prelude::*;
use std::sync::mpsc::Receiver;

pub struct Browser {
    state_receiver: Option<Receiver<TheEvent>>,
}

#[allow(clippy::new_without_default)]
impl Browser {
    pub fn new() -> Self {
        Self {
            state_receiver: None,
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, project: &mut Project) {
        let mut canvas = TheCanvas::new();

        let mut tab_layout = TheTabLayout::new(TheId::named("Browser"));
        tab_layout.limiter_mut().set_max_height(300);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar =  TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text("Welcome to Eldiron! Visit Eldiron.com for information and example projects.".to_string());
        status_canvas.set_widget(statusbar);

        canvas.set_bottom(status_canvas);
        canvas.set_layout(tab_layout);

        ui.canvas.set_bottom(canvas);
    }

    #[allow(clippy::single_match)]
    pub fn update_ui(&mut self, _ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        if let Some(receiver) = &mut self.state_receiver {
            while let Ok(event) = receiver.try_recv() {
                match event {
                    /*
                    TheEvent::StateChanged(id, _state) => {
                        //println!("app Widget State changed {:?}: {:?}", id, state);

                        if id.name == "Open" {
                            ctx.ui.open_file_requester(TheId::new("MyID".into()), "Open".into(), vec![] );
                            ctx.ui.set_widget_state("Open".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                        } else if id.name == "Cube" {
                            ctx.ui
                                .set_widget_state("Sphere".to_string(), TheWidgetState::None);
                            ctx.ui
                                .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 0));
                        } else if id.name == "Sphere" {
                            ctx.ui
                                .set_widget_state("Cube".to_string(), TheWidgetState::None);
                            ctx.ui
                                .send(TheEvent::SetStackIndex(self.stack_layout_id.clone(), 1));
                        }

                        redraw = true;
                    }
                    TheEvent::FileRequesterResult(id, paths) => {
                        println!("FileRequester Result {:?} {:?}", id, paths);
                    }*/
                    _ => {}
                }
            }
        }
        redraw
    }
}
