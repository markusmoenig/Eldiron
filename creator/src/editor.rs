use crate::prelude::*;
use crate::Embedded;
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

lazy_static! {
    pub static ref CODEEDITOR: Mutex<TheCodeEditor> = Mutex::new(TheCodeEditor::new());
    pub static ref TILEPICKER: Mutex<TilePicker> =
        Mutex::new(TilePicker::new("Main Tile Picker".to_string()));
    pub static ref TILEMAPEDITOR: Mutex<TilemapEditor> = Mutex::new(TilemapEditor::new());
    pub static ref SIDEBARMODE: Mutex<SidebarMode> = Mutex::new(SidebarMode::Region);
    pub static ref TILEDRAWER: Mutex<TileDrawer> = Mutex::new(TileDrawer::new());
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ActiveEditor {
    TileEditor,
    ScreenEditor,
}

pub struct Editor {
    project: Project,
    project_path: Option<PathBuf>,

    active_editor: ActiveEditor,

    sidebar: Sidebar,
    panels: Panels,
    tileeditor: TileEditor,
    screeneditor: ScreenEditor,

    server: Server,
    client: Client,
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

        let client = Client::new();

        Self {
            project: Project::new(),
            project_path: None,

            active_editor: ActiveEditor::TileEditor,

            sidebar: Sidebar::new(),
            panels: Panels::new(),
            tileeditor: TileEditor::new(),
            screeneditor: ScreenEditor::new(),

            server_ctx: ServerContext::default(),
            server,
            client,

            update_tracker: UpdateTracker::new(),
            event_receiver: None,
        }
    }

    fn window_title(&self) -> String {
        "Eldiron Creator".to_string()
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
        set_server_externals();

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
        open_button.set_status_text("Open an existing Eldiron project...");

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_status_text("Save the current project.");
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_status_text("Save the current project to a new file.");
        save_as_button.set_icon_offset(vec2i(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_status_text("Undo the last action.");
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_status_text("Redo the last action.");
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut play_button = TheMenubarButton::new(TheId::named("Play"));
        play_button.set_status_text("Start the server for live editing and debugging.");
        play_button.set_icon_name("play".to_string());

        let mut pause_button = TheMenubarButton::new(TheId::named("Pause"));
        pause_button.set_status_text("Pause. Click for single stepping the server.");
        pause_button.set_icon_name("play-pause".to_string());

        let mut stop_button = TheMenubarButton::new(TheId::named("Stop"));
        stop_button.set_status_text("Stop the server.");
        stop_button.set_icon_name("stop-fill".to_string());

        let mut patreon_button = TheMenubarButton::new(TheId::named("Patreon"));
        patreon_button.set_status_text("Visit my Patreon page.");
        patreon_button.set_icon_name("patreon".to_string());

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(vec4i(10, 2, 10, 1));
        hlayout.add_widget(Box::new(logo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(play_button));
        hlayout.add_widget(Box::new(pause_button));
        hlayout.add_widget(Box::new(stop_button));

        hlayout.add_widget(Box::new(patreon_button));

        hlayout.set_reverse_index(Some(1));

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar
        self.sidebar
            .init_ui(ui, ctx, &mut self.project, &mut self.server);

        // Panels
        self.panels.init_ui(ui, ctx, &mut self.project);

        // Editor
        let mut tab_canvas: TheCanvas = TheCanvas::new();
        let mut tab_layout = TheTabLayout::new(TheId::named("Editor Tab"));

        let game_canvas = self.tileeditor.init_ui(ui, ctx, &mut self.project);
        tab_layout.add_canvas(str!("Game"), game_canvas);

        let screen_canvas = self.screeneditor.init_ui(ui, ctx, &mut self.project);
        tab_layout.add_canvas(str!("Screen"), screen_canvas);

        tab_canvas.set_layout(tab_layout);
        ui.canvas.set_center(tab_canvas);

        // -

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let mut update_server_icons = false;

        let (redraw_update, tick_update) = self.update_tracker.update(
            (1000 / self.project.target_fps) as u64,
            self.project.tick_ms as u64,
        );

        if tick_update {
            // Update the widgets which have anims (if they are visible)
            if let Some(icon_view) = ui.get_widget("Global Icon Preview") {
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
            if let Some(icon_view) = ui.get_widget("Tilemap Selection Preview") {
                if let Some(icon_view) = icon_view.as_icon_view() {
                    icon_view.step();
                    redraw = true;
                }
            }
            if self.server.state == ServerState::Running {
                self.client.tick();
                let debug = self.server.tick();
                if !debug.is_empty() {
                    self.sidebar.add_debug_messages(debug, ui, ctx);
                }
                self.panels
                    .update_code_object(ui, ctx, &mut self.server, &mut self.server_ctx);
                if let Some(update) = self.server.get_region_update(self.server_ctx.curr_region) {
                    self.client.set_region_update(update);
                }

                if let Some(widget) = ui.get_widget("Server Time Slider") {
                    widget.set_value(TheValue::Time(self.server.world.time));
                }
            }

            // Set Debug Data

            let mut debug_entity: Option<Uuid> = None;
            if let Some(id) = self.server_ctx.curr_character_instance {
                debug_entity = Some(id);
            } else if let Some(id) = self.server_ctx.curr_area {
                debug_entity = Some(id);
            }

            if let Some(debug_entity) = debug_entity {
                let mut debug_has_set = false;

                if let Some(debug) = self
                    .server
                    .get_entity_debug_data(self.server_ctx.curr_region, debug_entity)
                {
                    let editor_codegrid_id = CODEEDITOR.lock().unwrap().get_codegrid_id(ui);
                    for debug in debug.values() {
                        if debug.codegrid_id == editor_codegrid_id {
                            CODEEDITOR
                                .lock()
                                .unwrap()
                                .set_debug_module(debug.clone(), ui);
                            debug_has_set = true;
                            break;
                        }
                    }
                }

                if !debug_has_set {
                    CODEEDITOR
                        .lock()
                        .unwrap()
                        .set_debug_module(TheDebugModule::default(), ui);
                }
            }
        }

        if self.active_editor == ActiveEditor::TileEditor
            && redraw_update
            && !self.project.regions.is_empty()
        {
            self.tileeditor
                .redraw_region(ui, &mut self.server, ctx, &self.server_ctx);
            redraw = true;
        } else if self.active_editor == ActiveEditor::ScreenEditor && redraw_update {
            self.screeneditor
                .redraw_screen(ui, &mut self.client, ctx, &self.server_ctx);
            redraw = true;
        }

        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                redraw = self.sidebar.handle_event(
                    &event,
                    ui,
                    ctx,
                    &mut self.project,
                    &mut self.server,
                    &mut self.client,
                    &mut self.server_ctx,
                );
                if self.panels.handle_event(
                    &event,
                    ui,
                    ctx,
                    &mut self.project,
                    &mut self.server,
                    &mut self.server_ctx,
                ) {
                    redraw = true;
                }
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
                if self.screeneditor.handle_event(
                    &event,
                    ui,
                    ctx,
                    &mut self.project,
                    &mut self.client,
                    &mut self.server_ctx,
                ) {
                    redraw = true;
                }
                if TILEMAPEDITOR.lock().unwrap().handle_event(
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
                    TheEvent::IndexChanged(id, index) => {
                        if id.name == "Editor Tab Tabbar" {
                            if index == 0 {
                                self.active_editor = ActiveEditor::TileEditor;
                            } else if index == 1 {
                                self.active_editor = ActiveEditor::ScreenEditor;
                                self.client.set_project(self.project.clone());
                            }
                            redraw = true;
                        }
                    }
                    TheEvent::DialogValueOnClose(role, name, uuid, value) => {
                        //println!("Dialog Value On Close: {} -> {:?}", name, value);

                        if name == "Delete Character Instance ?" {
                            if role == TheDialogButtonRole::Delete {
                                if let Some(region) =
                                    self.project.get_region_mut(&self.server_ctx.curr_region)
                                {
                                    let character_id = uuid;
                                    if region.characters.remove(&character_id).is_some() {
                                        self.server
                                            .remove_character_instance(region.id, character_id);
                                        self.server_ctx.curr_character_instance = None;
                                        self.server_ctx.curr_character = None;
                                        redraw = true;
                                        self.tileeditor.redraw_region(
                                            ui,
                                            &mut self.server,
                                            ctx,
                                            &self.server_ctx,
                                        );

                                        // Remove from the content list
                                        if let Some(list) =
                                            ui.get_list_layout("Region Content List")
                                        {
                                            list.remove(TheId::named_with_id(
                                                "Region Content List Item",
                                                character_id,
                                            ));
                                            ui.select_first_list_item("Region Content List", ctx);
                                        }
                                    }
                                }
                            }
                        } else if name == "Delete Item Instance ?" {
                            if role == TheDialogButtonRole::Delete {
                                if let Some(region) =
                                    self.project.get_region_mut(&self.server_ctx.curr_region)
                                {
                                    let item_id = uuid;
                                    if region.items.remove(&item_id).is_some() {
                                        self.server.remove_character_instance(region.id, item_id);
                                        self.server_ctx.curr_item_instance = None;
                                        self.server_ctx.curr_item = None;
                                        redraw = true;
                                        self.tileeditor.redraw_region(
                                            ui,
                                            &mut self.server,
                                            ctx,
                                            &self.server_ctx,
                                        );

                                        // Remove from the content list
                                        if let Some(list) =
                                            ui.get_list_layout("Region Content List")
                                        {
                                            list.remove(TheId::named_with_id(
                                                "Region Content List Item",
                                                item_id,
                                            ));
                                            ui.select_first_list_item("Region Content List", ctx);
                                        }
                                    }
                                }
                            }
                        } else if name == "Delete Area ?" {
                            if role == TheDialogButtonRole::Delete {
                                let area_id = uuid;

                                if let Some(region) =
                                    self.project.get_region_mut(&self.server_ctx.curr_region)
                                {
                                    if region.areas.remove(&area_id).is_some() {
                                        self.server.remove_area(region.id, area_id);
                                        self.server_ctx.curr_area = None;
                                        redraw = true;
                                        self.tileeditor.redraw_region(
                                            ui,
                                            &mut self.server,
                                            ctx,
                                            &self.server_ctx,
                                        );

                                        // Remove from the content list
                                        if let Some(list) =
                                            ui.get_list_layout("Region Content List")
                                        {
                                            list.remove(TheId::named_with_id(
                                                "Region Content List Item",
                                                area_id,
                                            ));
                                            ui.select_first_list_item("Region Content List", ctx);
                                        }
                                    }
                                }
                            }
                        } else if name == "New Area Name" {
                            // Create a new area

                            if let Some(tiles) = &self.server_ctx.tile_selection {
                                let mut area = Area {
                                    area: tiles.tiles(),
                                    name: value.describe(),
                                    ..Default::default()
                                };

                                let main = TheCodeGrid {
                                    name: "main".into(),
                                    ..Default::default()
                                };

                                area.bundle.insert_grid(main);

                                if let Some(list) = ui.get_list_layout("Region Content List") {
                                    let mut item = TheListItem::new(TheId::named_with_id(
                                        "Region Content List Item",
                                        area.id,
                                    ));
                                    item.set_text(area.name.clone());
                                    item.set_state(TheWidgetState::Selected);
                                    item.add_value_column(100, TheValue::Text("Area".to_string()));

                                    list.deselect_all();
                                    list.add_item(item, ctx);
                                    list.select_item(area.id, ctx, true);
                                }

                                self.server_ctx.curr_area = Some(area.id);
                                self.server_ctx.curr_character_instance = None;
                                self.server_ctx.curr_character = None;

                                if let Some(region) =
                                    self.project.get_region_mut(&self.server_ctx.curr_region)
                                {
                                    region.areas.insert(area.id, area);
                                }
                            }
                            self.server_ctx.tile_selection = None;
                        }
                    }
                    TheEvent::TileEditorDrop(_id, location, drop) => {
                        if drop.id.name.starts_with("Character") {
                            let mut instance = TheCodeBundle::new();

                            let mut init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };
                            init.insert_atom(
                                (0, 0),
                                TheCodeAtom::Set(
                                    ":self.position".to_string(),
                                    TheValueAssignment::Assign,
                                ),
                            );
                            init.insert_atom(
                                (1, 0),
                                TheCodeAtom::Assignment(TheValueAssignment::Assign),
                            );
                            init.insert_atom(
                                (2, 0),
                                TheCodeAtom::Value(TheValue::Position(vec3f(
                                    location.x as f32,
                                    0.0,
                                    location.y as f32,
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

                            // Add the character instance to the region content list

                            let mut name = "Character".to_string();
                            if let Some(character) = self.project.characters.get(&drop.id.uuid) {
                                name = character.name.clone();
                            }

                            if let Some(list) = ui.get_list_layout("Region Content List") {
                                let mut item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    character.id,
                                ));
                                item.set_text(name);
                                item.set_state(TheWidgetState::Selected);
                                item.add_value_column(100, TheValue::Text("Character".to_string()));

                                list.deselect_all();
                                list.add_item(item, ctx);
                                list.select_item(character.id, ctx, true);
                            }

                            // Add the character instance to the project

                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                region.characters.insert(character.id, character.clone());
                            }

                            // Add the character instance to the server

                            self.server_ctx.curr_character = Some(character.character_id);
                            self.server_ctx.curr_character_instance = Some(character.id);
                            self.server_ctx.curr_area = None;
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
                        } else if drop.id.name.starts_with("Item") {
                            let mut instance = TheCodeBundle::new();

                            let mut init = TheCodeGrid {
                                name: "init".into(),
                                ..Default::default()
                            };
                            init.insert_atom(
                                (0, 0),
                                TheCodeAtom::Set(
                                    ":self.position".to_string(),
                                    TheValueAssignment::Assign,
                                ),
                            );
                            init.insert_atom(
                                (1, 0),
                                TheCodeAtom::Assignment(TheValueAssignment::Assign),
                            );
                            init.insert_atom(
                                (2, 0),
                                TheCodeAtom::Value(TheValue::Position(vec3f(
                                    location.x as f32,
                                    0.0,
                                    location.y as f32,
                                ))),
                            );
                            instance.insert_grid(init);

                            // Set the character instance bundle, disabled for now

                            // self.sidebar.code_editor.set_bundle(
                            //     instance.clone(),
                            //     ctx,
                            //     self.sidebar.width,
                            // );

                            let item = Item {
                                id: instance.id,
                                item_id: drop.id.uuid,
                                instance,
                            };

                            // Add the item instance to the region content list

                            let mut name = "Item".to_string();
                            if let Some(item) = self.project.items.get(&drop.id.uuid) {
                                name = item.name.clone();
                            }

                            if let Some(list) = ui.get_list_layout("Region Content List") {
                                let mut list_item = TheListItem::new(TheId::named_with_id(
                                    "Region Content List Item",
                                    item.id,
                                ));
                                list_item.set_text(name);
                                list_item.set_state(TheWidgetState::Selected);
                                list_item.add_value_column(100, TheValue::Text("Item".to_string()));

                                list.deselect_all();
                                list.add_item(list_item, ctx);
                                list.select_item(item.id, ctx, true);
                            }

                            // Add the item instance to the project

                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                region.items.insert(item.id, item.clone());
                            }

                            // Add the character instance to the server

                            self.server_ctx.curr_character = None;
                            self.server_ctx.curr_character_instance = None;
                            self.server_ctx.curr_item = Some(item.item_id);
                            self.server_ctx.curr_item_instance = Some(item.id);
                            self.server_ctx.curr_area = None;

                            self.server_ctx.curr_grid_id = self
                                .server
                                .add_item_instance_to_region(self.server_ctx.curr_region, item);

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
                                self.project_path = Some(p.clone());
                                let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                                self.project =
                                    serde_json::from_str(&contents).unwrap_or(Project::new());
                                self.sidebar.load_from_project(ui, ctx, &self.project);
                                self.tileeditor.load_from_project(ui, ctx, &self.project);
                                let packages = self.server.set_project(self.project.clone());
                                self.client.set_project(self.project.clone());
                                CODEEDITOR.lock().unwrap().set_packages(packages);
                                self.server.state = ServerState::Stopped;
                                update_server_icons = true;
                                redraw = true;
                                self.server_ctx.clear();
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Project loaded successfully.".to_string(),
                                ))
                            }
                        } else if id.name == "Save As" {
                            for p in paths {
                                let json = serde_json::to_string(&self.project);
                                if let Ok(json) = json {
                                    if std::fs::write(p, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Project saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save project!".to_string(),
                                        ))
                                    }
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
                        } else if id.name == "Patreon" {
                            _ = open::that("https://www.patreon.com/eldiron");
                            ctx.ui
                                .set_widget_state("Patreon".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        } else if id.name == "Open" {
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
                            if let Some(path) = &self.project_path {
                                let json = serde_json::to_string(&self.project);
                                if let Ok(json) = json {
                                    if std::fs::write(path, json).is_ok() {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Project saved successfully.".to_string(),
                                        ))
                                    } else {
                                        ctx.ui.send(TheEvent::SetStatusText(
                                            TheId::empty(),
                                            "Unable to save project!".to_string(),
                                        ))
                                    }
                                }
                            }
                        } else if id.name == "Save As" {
                            ctx.ui.save_file_requester(
                                TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                                "Save".into(),
                                TheFileExtension::new(
                                    "Eldiron".into(),
                                    vec!["eldiron".to_string()],
                                ),
                            );
                            ctx.ui
                                .set_widget_state("Save As".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                        // Server
                        else if id.name == "Play" {
                            self.server.start();
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Server has been started.".to_string(),
                            ));
                            self.sidebar.clear_debug_messages(ui, ctx);
                            update_server_icons = true;
                        } else if id.name == "Pause" {
                            if self.server.state == ServerState::Running {
                                self.server.state = ServerState::Paused;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Server has been paused.".to_string(),
                                ));
                                update_server_icons = true;
                            } else if self.server.state == ServerState::Paused {
                                self.client.tick();
                                let debug = self.server.tick();
                                if !debug.is_empty() {
                                    self.sidebar.add_debug_messages(debug, ui, ctx);
                                }
                            }
                        } else if id.name == "Stop" {
                            _ = self.server.set_project(self.project.clone());
                            self.server.stop();
                            update_server_icons = true;
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
                        if id.name == "Add Image" {
                            // Add a new tilemap to the project
                            let asset = Asset {
                                name,
                                id: id.uuid,
                                buffer: AssetBuffer::Image(buffer),
                            };

                            self.project.add_asset(asset);
                            self.client.set_assets(self.project.assets.clone());
                        } else if id.name == "Tilemap Add" {
                            // Add a new tilemap to the project
                            let mut tilemap = Tilemap::new();
                            tilemap.name = name;
                            tilemap.id = id.uuid;
                            tilemap.buffer = buffer;

                            self.project.add_tilemap(tilemap);
                        }
                    }
                    TheEvent::ValueChanged(id, value) => {
                        if id.name == "Server Time Slider" {
                            if let TheValue::Time(time) = value {
                                self.server.set_time(time);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if update_server_icons {
            self.update_server_state_icons(ui);
            redraw = true;
        }
        redraw
    }
}

pub trait EldironEditor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI);
}

impl EldironEditor for Editor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI) {
        if self.server.state == ServerState::Running {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if self.server.state == ServerState::Paused {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if self.server.state == ServerState::Stopped {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop-fill".to_string());
                }
            }
        }
    }
}
