use crate::prelude::*;
use std::sync::mpsc::Receiver;
use crate::Embedded;

pub struct Editor {
    project: Project,

    sidebar: Sidebar,
    browser: Browser,
    tileeditor: TileEditor,

    server: Server,
    server_ctx: ServerContext,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut server = Server::new();
        server.debug_mode = true;

        Self {
            project: Project::new(),

            sidebar: Sidebar::new(),
            browser: Browser::new(),
            tileeditor: TileEditor::new(),

            server_ctx: ServerContext::default(),
            server,

            update_tracker: UpdateTracker::new(),
            event_receiver: None,
        }
    }

    fn window_title(&mut self) -> String {
        "Eldiron".to_string()
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {

        // Embedded Icons

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

        // ---

        ui.set_statusbar_name("Statusbar".to_string());

        // Menubar
        let mut top_canvas = TheCanvas::new();

        let menubar = TheMenubar::new(TheId::named("Menubar"));

        let mut logo_button = TheMenubarButton::new(TheId::named("Logo"));
        logo_button.set_icon_name("logo".to_string());
        logo_button.set_status_text("Open the Eldiron Website ...");

        let mut open_button = TheMenubarButton::new(TheId::named("Open"));
        open_button.set_icon_name("icon_role_load".to_string());
        open_button.set_status_text("Open an existing project...");

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_icon_offset(vec2i(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_status_text("Undo the last action.");
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_status_text("Redo the last action.");
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut patreon_button = TheMenubarButton::new(TheId::named("Patreon"));
        patreon_button.set_status_text("Visit my Patreon page.");
        patreon_button.set_icon_name("patreon".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(vec4i(10, 5, 20, 0));
        hlayout.add_widget(Box::new(logo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(patreon_button));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar
        self.sidebar.init_ui(ui, ctx, &mut self.project);

        // Browser
        self.browser.init_ui(ui, ctx, &mut self.project);

        // TileEditor
        self.tileeditor.init_ui(ui, ctx, &mut self.project);

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        if self.update_tracker.update(250) {
            // Update the widgets which have anims (if they are visible)
            if let Some(icon_view) = ui.get_widget("Tilemap Editor Icon View") {
                if let Some(icon_view) = icon_view.as_icon_view() {
                    icon_view.step();
                    redraw = true;
                }
            }
            if let Some(icon_view) = ui.get_widget("Icon Preview") {
                if let Some(icon_view) = icon_view.as_icon_view() {
                    icon_view.step();
                    redraw = true;
                }
            }
            self.server.tick();
            if self.server_ctx.curr_character_instance.is_some() {
                let debug = self.server.get_region_debug_codegrid(
                    self.server_ctx.curr_region,
                    self.sidebar.code_editor.get_codegrid_id(ui),
                );
                self.sidebar.code_editor.set_debug_module(debug, ui);
            }
            self.tileeditor
                .redraw_region(ui, &mut self.server, ctx, &self.server_ctx);
        }

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.sidebar.handle_event(
                    &event,
                    ui,
                    ctx,
                    &mut self.project,
                    &mut self.server,
                    &mut self.server_ctx,
                );
                if self.tileeditor.handle_event(
                    &event,
                    ui,
                    ctx,
                    &mut self.project,
                    &mut self.server,
                    &mut self.server_ctx,
                ) {
                    redraw = true;
                }
                match event {
                    TheEvent::TileEditorDrop(_id, location, drop) => {
                        if drop.id.name.starts_with("Character") {
                            let mut instance = TheCodeBundle::new();

                            let mut init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };
                            init.insert_atom(
                                (0, 0),
                                TheCodeAtom::ObjectSet("self".to_string(), "position".to_string()),
                            );
                            init.insert_atom((1, 0), TheCodeAtom::Assignment("=".to_string()));
                            init.insert_atom(
                                (2, 0),
                                TheCodeAtom::Value(TheValue::Position(vec3f(
                                    location.x as f32,
                                    location.y as f32,
                                    0.0,
                                ))),
                            );
                            instance.insert_grid(init);

                            // Set the character instance bundle, disabled for now

                            // self.sidebar.code_editor.set_bundle(
                            //     instance.clone(),
                            //     ctx,
                            //     self.sidebar.width,
                            // );

                            let character = Character {
                                id: instance.id,
                                character_id: drop.id.uuid,
                                instance,
                            };

                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                region.characters.insert(character.id, character.clone());
                            }

                            self.server_ctx.curr_character = Some(character.character_id);
                            self.server_ctx.curr_character_instance = Some(character.id);
                            //self.sidebar.deselect_all("Character List", ui);

                            self.server_ctx.curr_grid_id =
                                self.server.add_character_instance_to_region(
                                    self.server_ctx.curr_region,
                                    character,
                                );

                            // Set the character instance debug info, disabled for now

                            // if let Some(curr_grid_id) = self.server_ctx.curr_grid_id {
                            //     let debug_module = self.server.get_region_debug_module(
                            //         self.server_ctx.curr_region,
                            //         curr_grid_id,
                            //     );

                            //     self.sidebar.code_editor.set_debug_module(debug_module, ui);
                            // }
                        }
                    }
                    TheEvent::FileRequesterResult(id, paths) => {
                        if id.name == "Open" {
                            for p in paths {
                                let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                                self.project =
                                    serde_json::from_str(&contents).unwrap_or(Project::new());
                                self.sidebar.load_from_project(ui, ctx, &self.project);
                                self.tileeditor.load_from_project(ui, ctx, &self.project);
                                self.server.set_project(self.project.clone());
                                redraw = true;
                            }
                        } else if id.name == "Save" {
                            for p in paths {
                                let json = serde_json::to_string(&self.project); //.unwrap();
                                                                                 //println!("{:?}", json.err());
                                if let Ok(json) = json {
                                    std::fs::write(p, json).expect("Unable to write file");
                                }
                            }
                        }
                    }
                    TheEvent::StateChanged(id, _state) => {
                        // Open / Save Project

                        if id.name == "Logo" {
                            _ = open::that("https://eldiron.com");
                            ctx.ui
                                .set_widget_state("Logo".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }

                        else if id.name == "Patreon" {
                            _ = open::that("https://www.patreon.com/eldiron");
                            ctx.ui
                                .set_widget_state("Patreon".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }

                        else if id.name == "Open" {
                            ctx.ui.open_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Open".into(),
                                TheFileExtension::new(
                                    "Eldiron".into(),
                                    vec!["eldiron".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Open".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else if id.name == "Save" {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Save".into(),
                                TheFileExtension::new(
                                    "Eldiron".into(),
                                    vec!["eldiron".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Save".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else {
                            let mut data: Option<(TheId, String)> = None;
                            if id.name == "Undo" && ctx.ui.undo_stack.has_undo() {
                                data = Some(ctx.ui.undo_stack.undo());
                            } else if id.name == "Redo" && ctx.ui.undo_stack.has_redo() {
                                data = Some(ctx.ui.undo_stack.redo());
                            }

                            if let Some((id, json)) = data {
                                #[allow(clippy::single_match)]
                                match id.name.as_str() {
                                    "RegionChanged" => {
                                        let region = Region::from_json(json.as_str());
                                        for (index, r) in self.project.regions.iter().enumerate() {
                                            if r.id == region.id {
                                                self.server.update_region(&region);
                                                if region.id == self.server_ctx.curr_region {
                                                    self.tileeditor.redraw_region(
                                                        ui,
                                                        &mut self.server,
                                                        ctx,
                                                        &self.server_ctx,
                                                    );
                                                }
                                                self.project.regions[index] = region;
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                redraw = true;
                            }
                        }
                    }
                    TheEvent::ImageDecodeResult(id, name, buffer) => {
                        // Add a new tilemap to the project
                        if id.name == "Tilemap Add" {
                            let mut tilemap = Tilemap::new();
                            tilemap.name = name;
                            tilemap.id = id.uuid;
                            tilemap.buffer = buffer;

                            self.project.add_tilemap(tilemap);
                        }
                    }
                    TheEvent::ValueChanged(id, value) => {
                        //println!("{:?} {:?}", id, value);
                        if id.name == "Region Name Edit" {
                            if let Some(list_id) =
                                self.sidebar.get_selected_in_list_layout(ui, "Region List")
                            {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else if id.name == "Region Item" {
                            for r in &mut self.project.regions {
                                if r.id == id.uuid {
                                    if let Some(text) = value.to_string() {
                                        r.name = text;
                                    }
                                }
                            }
                        } else if id.name == "Character Name Edit" {
                            if let Some(list_id) = self
                                .sidebar
                                .get_selected_in_list_layout(ui, "Character List")
                            {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else if id.name == "Character Item" {
                            if let Some(character) = self.project.characters.get_mut(&id.uuid) {
                                if let Some(text) = value.to_string() {
                                    character.name = text;
                                }
                            }
                        } else if id.name == "Item Name Edit" {
                            if let Some(list_id) =
                                self.sidebar.get_selected_in_list_layout(ui, "Item List")
                            {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else if id.name == "Item Item" {
                            if let Some(item) = self.project.items.get_mut(&id.uuid) {
                                if let Some(text) = value.to_string() {
                                    item.name = text;
                                }
                            }
                        } else if id.name == "Tilemap Name Edit" {
                            if let Some(list_id) =
                                self.sidebar.get_selected_in_list_layout(ui, "Tilemap List")
                            {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else if id.name == "Tilemap Item" {
                            for t in &mut self.project.tilemaps {
                                if t.id == id.uuid {
                                    if let Some(text) = value.to_string() {
                                        t.name = text;
                                    }
                                }
                            }
                        } else if id.name == "Code Name Edit" {
                            if let Some(list_id) =
                                self.sidebar.get_selected_in_list_layout(ui, "Code List")
                            {
                                ctx.ui.send(TheEvent::SetValue(list_id.uuid, value));
                            }
                        } else if id.name == "Code Item" {
                            if let Some(code) = self.project.codes.get_mut(&id.uuid) {
                                if let Some(text) = value.to_string() {
                                    code.name = text;
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
