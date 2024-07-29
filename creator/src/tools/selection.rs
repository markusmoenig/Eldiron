use crate::prelude::*;
use ToolEvent::*;

pub struct SelectionTool {
    id: TheId,
}

impl Tool for SelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Select Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Selection Tool (S). Select areas in the region editor.")
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
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
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

                return true;
            }
            DeActivate => {
                server_ctx.tile_selection = None;
                return true;
            }
            TileUp => {
                if let Some(tilearea) = &mut server_ctx.tile_selection {
                    tilearea.ongoing = false;
                }
                return true;
            } // _ => {
              //     return false;
              // }
        };

        let p = (coord.x, coord.y);

        if let Some(tilearea) = &mut server_ctx.tile_selection {
            if !tilearea.ongoing {
                tilearea.start = p;
                tilearea.end = p;
                tilearea.ongoing = true;
            } else {
                tilearea.grow_by(p);
            }
        } else {
            let tilearea = TileArea {
                start: p,
                end: p,
                ..Default::default()
            };
            server_ctx.tile_selection = Some(tilearea);
        }

        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ContextMenuSelected(_widget_id, item_id) => {
                if item_id.name == "Create Area" {
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

                    if let Some(tiles) = &server_ctx.tile_selection {
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
                        }
                    }
                    server_ctx.tile_selection = None;
                }
                true
            }
            _ => false,
        }
    }
}
