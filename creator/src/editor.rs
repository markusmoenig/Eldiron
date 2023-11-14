use crate::{prelude::*, browser::Browser};
use std::sync::mpsc::Receiver;

pub struct Editor {
    project: Project,

    sidebar: Sidebar,
    browser: Browser,
    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            sidebar: Sidebar::new(),
            browser: Browser::new(),
            event_receiver: None,

            project: Project::default(),
        }
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
        save_as_button.set_icon_offset(vec2i(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(vec4i(40, 5, 20, 0));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar

        self.sidebar.init_ui(ui, ctx, &mut self.project);

        // Browser

        self.browser.init_ui(ui, ctx, &mut self.project);

        // Main

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.sidebar.handle_event(&event, ui, ctx, &mut self.project);
                match event {

                    TheEvent::FileRequesterResult(id, paths) => {
                        if id.name == "Open" {
                            for p in paths {
                                let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                                self.project = serde_json::from_str(&contents).unwrap_or(Project::default());
                                self.sidebar.load_from_project(ui, ctx, &self.project);
                                redraw = true;
                            }
                        } else if id.name == "Save" {
                            for p in paths {
                                let json = serde_json::to_string(&self.project).unwrap();
                                std::fs::write(p, json).expect("Unable to write file");
                            }
                        }
                    }
                    TheEvent::StateChanged(id, _state) => {

                        // Open / Save Project

                        if id.name == "Open" {
                            ctx.ui.open_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Open".into(),
                                TheFileExtension::new("Eldiron".into(), vec!["eldiron".to_string()]),
                            );
                            ctx.ui
                                .set_widget_state("Open".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else
                        if id.name == "Save" {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Save".into(),
                                TheFileExtension::new("Eldiron".into(), vec!["eldiron".to_string()]),
                            );
                            ctx.ui
                                .set_widget_state("Save".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                    }
                    TheEvent::ImageDecodeResult(id, name, buffer) => {
                        // Add a new tilemap to the project
                        if id.name == "Tiles Add" {

                            let mut tilemap = Tilemap::default();
                            tilemap.name = name;
                            tilemap.id = id.uuid;
                            tilemap.buffer = buffer;

                            self.project.add_tilemap(tilemap);
                        }
                    }
                    TheEvent::ValueChanged(id, value) => {
                        //println!("{:?} {:?}", id, value);
                        if id.name == "Tiles Name Edit" {
                            if let Some(list_id) = self.sidebar.get_selected_in_list_layout(ui, "Tiles List") {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else
                        if id.name == "Tiles Item" {
                            for t in &mut self.project.tilemaps {
                                if t.id == id.uuid {
                                    if let Some(text) = value.to_string() {
                                        t.name = text;
                                    }
                                }
                            }
                        }
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
