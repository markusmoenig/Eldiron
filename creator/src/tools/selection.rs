use crate::prelude::*;
use ToolEvent::*;

pub struct SelectionTool {
    id: TheId,

    tile_selection: TileSelection,
}

impl Tool for SelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Select Tool"),
            tile_selection: TileSelection::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        if cfg!(target_os = "macos") {
            str!("Selection Tool (S). Select and Cut / Copy. Hold 'Shift' to add. 'Option' to subtract. 'Escape' to clear.")
        } else {
            str!("Selection Tool (S). Select and Cut / Copy. Hold 'Shift' to add. 'Alt' to subtract. 'Escape' to clear.")
        }
    }
    fn icon_name(&self) -> String {
        str!("selection")
    }
    fn accel(&self) -> Option<char> {
        Some('s')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut create_area_button =
                        TheTraybarButton::new(TheId::named("Editor Create Area"));
                    create_area_button.set_text(str!("Create Area..."));
                    create_area_button.limiter_mut().set_max_width(140);
                    create_area_button
                        .set_status_text("Creates a new area for the current selection.");
                    create_area_button.set_disabled(self.tile_selection.tiles.is_empty());

                    let mut clear_area_button =
                        TheTraybarButton::new(TheId::named("Editor Clear Selection"));
                    clear_area_button.set_text(str!("Clear"));
                    //clear_area_button.limiter_mut().set_max_width(140);
                    clear_area_button
                        .set_status_text("Clears the current selection. Shortcut: 'Escape'.");

                    layout.add_widget(Box::new(create_area_button));
                    layout.add_widget(Box::new(clear_area_button));

                    layout.set_reverse_index(Some(1));
                }

                ui.set_widget_context_menu(
                    "Region Editor View",
                    Some(TheContextMenu {
                        items: vec![TheContextMenuItem::new(
                            "Create Area...".to_string(),
                            TheId::named("Create Area"),
                        )],
                        ..Default::default()
                    }),
                );

                server_ctx.tile_selection = Some(self.tile_selection.clone());

                return true;
            }
            DeActivate => {
                server_ctx.tile_selection = None;
                ui.set_widget_context_menu("Region Editor View", None);
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                return true;
            }
            _ => {}
        };

        if let TileDown(coord, _) = tool_event {
            let p = (coord.x, coord.y);

            let mut mode = TileSelectionMode::Additive;
            let mut tiles: FxHashSet<(i32, i32)> = FxHashSet::default();

            if ui.shift {
                tiles = self.tile_selection.tiles.clone();
            } else if ui.alt {
                tiles = self.tile_selection.tiles.clone();
                mode = TileSelectionMode::Subtractive;
            }

            let tile_area = TileSelection {
                mode,
                rect_start: p,
                rect_end: p,
                tiles,
            };
            server_ctx.tile_selection = Some(tile_area);
        }
        if let TileDrag(coord, _) = tool_event {
            let p = (coord.x, coord.y);
            if let Some(tile_selection) = &mut server_ctx.tile_selection {
                tile_selection.grow_rect_by(p);
            }
        }
        if let TileUp = tool_event {
            if let Some(tile_selection) = &mut server_ctx.tile_selection {
                self.tile_selection.tiles = tile_selection.merged();
            }

            ui.set_widget_disabled_state(
                "Editor Create Area",
                ctx,
                self.tile_selection.tiles.is_empty(),
            );
        }

        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::Cut => {
                println!("Cut");

                false
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                if *code == TheKeyCode::Escape {
                    self.tile_selection = TileSelection::default();
                    server_ctx.tile_selection = Some(self.tile_selection.clone());
                    ui.set_widget_disabled_state(
                        "Editor Create Area",
                        ctx,
                        self.tile_selection.tiles.is_empty(),
                    );
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Editor Clear Selection" {
                    self.tile_selection = TileSelection::default();
                    server_ctx.tile_selection = Some(self.tile_selection.clone());
                    ui.set_widget_disabled_state(
                        "Editor Create Area",
                        ctx,
                        self.tile_selection.tiles.is_empty(),
                    );

                    true
                } else if id.name == "Editor Create Area" {
                    open_text_dialog(
                        "New Area Name",
                        "Area Name",
                        "New Area",
                        Uuid::new_v4(),
                        ui,
                        ctx,
                    );

                    true
                } else {
                    false
                }
            }
            TheEvent::ContextMenuSelected(_widget_id, item_id) => {
                if item_id.name == "Create Area" && !self.tile_selection.tiles.is_empty() {
                    open_text_dialog(
                        "New Area Name",
                        "Area Name",
                        "New Area",
                        Uuid::new_v4(),
                        ui,
                        ctx,
                    );
                }
                true
            }
            TheEvent::DialogValueOnClose(_role, name, _uuid, value) => {
                if name == "New Area Name" {
                    // Create a new area

                    if !self.tile_selection.tiles.is_empty() {
                        let mut area = Area {
                            area: self.tile_selection.tiles.clone(),
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
                            item.set_context_menu(Some(TheContextMenu {
                                items: vec![TheContextMenuItem::new(
                                    "Delete Area...".to_string(),
                                    TheId::named("Sidebar Delete Area"),
                                )],
                                ..Default::default()
                            }));

                            list.deselect_all();
                            list.add_item(item, ctx);
                            list.select_item(area.id, ctx, true);
                        }

                        server_ctx.curr_area = Some(area.id);
                        server_ctx.curr_character_instance = None;
                        server_ctx.curr_character = None;

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.areas.insert(area.id, area);
                            server.update_region(region);
                        }
                        server_ctx.tile_selection = None;
                    }
                }
                true
            }
            _ => false,
        }
    }
}
