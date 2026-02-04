use crate::prelude::*;
use codegridfxlib::Module;
use std::sync::mpsc::Receiver;

pub struct CodeEditor {
    module: Module,

    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for CodeEditor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            module: Module::new("New Module"),
            event_receiver: None,
        }
    }

    fn window_title(&self) -> String {
        "CodeGridFX".to_string()
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        // Top
        let mut top_canvas = TheCanvas::new();

        let mut menubar = TheMenubar::new(TheId::named("Menubar"));
        menubar.limiter_mut().set_max_height(43);

        let mut build_button = TheMenubarButton::new(TheId::named("Build"));
        build_button.set_icon_name("icon_role_load".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(Vec4::new(40, 5, 20, 0));
        hlayout.add_widget(Box::new(build_button));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        // top_canvas.set_top(menu_canvas);

        ui.canvas.set_top(top_canvas);

        self.module.get_colors(ui);
        self.module.update_routines();
        self.module
            .set_module_type(codegridfxlib::ModuleType::CharacterTemplate);

        ui.canvas.set_center(self.module.build_canvas(ctx));

        let mut node_ui_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named("Node Settings"));
        text_layout.limiter_mut().set_max_width(300);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        node_ui_canvas.set_layout(text_layout);
        ui.canvas.set_right(node_ui_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text("CodeGridFX Tester".to_string());
        status_canvas.set_widget(statusbar);
        ui.canvas.set_bottom(status_canvas);

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw: bool = false;

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.module.handle_event(&event, ui, ctx);

                match event {
                    TheEvent::StateChanged(id, _) => {
                        if id.name == "Build" {
                            println!("Build");
                            let code = self.module.build(true);
                            println!("{}", code);
                        }
                    }
                    _ => {}
                }
            }
        }
        redraw
    }
}
