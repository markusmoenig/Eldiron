use crate::prelude::*;
use crate::editor::{CODEEDITOR, TILEPICKER};

pub struct Panels {
}

#[allow(clippy::new_without_default)]
impl Panels {
    pub fn new() -> Self {

        CODEEDITOR.lock().unwrap().add_external(TheExternalCode::new(
            "RandWalk".to_string(),
            "Moves the character in a random direction.".to_string(),
        ));

        Self {
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext, _project: &mut Project) {
        let mut canvas = TheCanvas::new();

        //let mut tab_layout = TheTabLayout::new(TheId::named("Browser"));
        //tab_layout.limiter_mut().set_max_height(300);

        let mut shared_layout = TheSharedLayout::new(TheId::named("Shared Panel Layout"));
        shared_layout.limiter_mut().set_max_height(300);

        // Left Stack

        let mut left_canvas = TheCanvas::new();
        let mut left_stack = TheStackLayout::new(TheId::named("Left Stack"));

        left_stack.add_canvas(TILEPICKER.lock().unwrap().build(false));
        left_stack.add_canvas(CODEEDITOR.lock().unwrap().build_canvas(ctx));

        left_canvas.set_layout(left_stack);

        // Right Stack

        let mut right_canvas = TheCanvas::new();
        let mut right_stack = TheStackLayout::new(TheId::named("Right Stack"));
        right_canvas.set_layout(right_stack);

        shared_layout.add_canvas(left_canvas);
        shared_layout.add_canvas(right_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text(
            "Welcome to Eldiron! Visit Eldiron.com for information and example projects."
                .to_string(),
        );
        status_canvas.set_widget(statusbar);

        canvas.set_bottom(status_canvas);
        canvas.set_layout(shared_layout);

        ui.canvas.set_bottom(canvas);
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = CODEEDITOR.lock().unwrap().handle_event(event, ui, ctx);
        if TILEPICKER.lock().unwrap().handle_event(event, ui, ctx) {
            redraw = true;
        }
        match event {
            TheEvent::Custom(id, value) => {
                if id.name == "Update Tiles" {
                }
            }
            _ => {}
        }

        redraw
    }

}