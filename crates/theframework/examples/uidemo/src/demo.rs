use crate::prelude::*;
use std::sync::mpsc::Receiver;
use theframework::prelude::*;

pub struct UIDemo {
    sidebar: Sidebar,
    renderer: Renderer,
    project: Project,

    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for UIDemo {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            sidebar: Sidebar::new(),
            renderer: Renderer::new(),
            project: Project::new(),

            event_receiver: None,
        }
    }

    fn window_title(&self) -> String {
        "UIDemo".to_string()
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        // Menubar
        let mut top_canvas = TheCanvas::new();

        let menubar = TheMenubar::new(TheId::named("Menubar"));

        let mut open_button = TheMenubarButton::new(TheId::named("Open"));
        open_button.set_icon_name("icon_role_load".to_string());

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_icon_offset(Vec2::new(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(Vec4::new(40, 5, 20, 0));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);

        // Right Sidebar
        self.sidebar.init_ui(ui, ctx, &mut self.project);

        // Renderer
        self.renderer.init_ui(ui, ctx, &mut self.project);

        // Copy the command command from the renderer to the sidebar.
        self.sidebar
            .renderer_command
            .clone_from(&self.renderer.renderer_command);

        // Statusbar

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text("Welcome to TheFramework!".to_string());
        status_canvas.set_widget(statusbar);

        ui.set_statusbar_name("Statusbar".to_string());
        ui.canvas.set_top(top_canvas);
        ui.canvas.set_bottom(status_canvas);

        // Create an event listener.
        self.event_receiver = Some(ui.add_state_listener("Main".into()));
    }

    /// Update the UI by handling the events from the UI subsystem.
    /// Return true if the UI needs to be redrawn.
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = self.renderer.check_renderer_update(ui, ctx);

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                // Handle the UI events in the sidebar
                if self
                    .sidebar
                    .handle_event(&event, ui, ctx, &mut self.project)
                {
                    redraw = true;
                }

                // Handle the UI events in the renderer
                if self
                    .renderer
                    .handle_event(&event, ui, ctx, &mut self.project)
                {
                    redraw = true;
                }

                // match event {
                //     _ => {}
                // }
            }
        }

        redraw
    }
}
