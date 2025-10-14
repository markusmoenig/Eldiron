use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

use crate::editor::CODEEDITOR;

pub struct DataTool {
    id: TheId,
}

impl Tool for DataTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Data Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Data Tool (D).")
    }
    fn icon_name(&self) -> String {
        str!("database")
    }
    fn accel(&self) -> Option<char> {
        Some('D')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            ctx.ui.send(TheEvent::SetStackIndex(
                TheId::named("Main Stack"),
                PanelIndices::DataEditor as usize,
            ));

            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut text = TheText::new(TheId::named("Data Editor Header Text"));
                text.set_text(CODEEDITOR.read().unwrap().last_data_header_text.clone());
                layout.add_widget(Box::new(text));
            }

            return true;
        };

        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

        match map_event {
            MapClicked(_) => {
                if server_ctx.hover.2.is_some() {
                    let prev = map.clone();
                    let mut changed = false;

                    map.selected_entity_item = None;

                    if ui.shift {
                        // Add
                        if let Some(s) = server_ctx.hover.2 {
                            if !map.selected_sectors.contains(&s) {
                                map.selected_sectors.push(s);
                                changed = true;
                            }
                        }
                    } else if ui.alt {
                        // Subtract
                        if let Some(v) = server_ctx.hover.2 {
                            map.selected_sectors.retain(|&selected| selected != v);
                            changed = true;
                        }
                    } else {
                        // Replace
                        if let Some(v) = server_ctx.hover.2 {
                            map.selected_sectors = vec![v];
                            changed = true;
                        } else {
                            map.selected_sectors.clear();
                            changed = true;
                        }
                    }

                    if changed {
                        undo_atom = Some(RegionUndoAtom::MapEdit(
                            Box::new(prev),
                            Box::new(map.clone()),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));

                        for sector in &map.sectors {
                            if Some(sector.id) == server_ctx.hover.2 {
                                // ctx.ui.send(TheEvent::StateChanged(
                                //     TheId::named_with_id(
                                //         "Screen Content List Item",
                                //         sector.creator_id,
                                //     ),
                                //     TheWidgetState::Clicked,
                                // ));
                                if let Some(layout) = ui.get_list_layout("Screen Content List") {
                                    // server_ctx.content_click_from_map = true;
                                    layout.select_item(sector.creator_id, ctx, true);
                                }
                            }
                        }
                    }
                }
            }
            MapHover(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    let h = server_ctx.geometry_at(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                    );
                    server_ctx.hover.2 = h.2;

                    let cp = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Cursor Pos Changed"),
                        TheValue::Float2(cp),
                    ));
                    server_ctx.hover_cursor = Some(cp);
                }
            }
            _ => {}
        }
        undo_atom
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, state) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "Build" && *state == TheWidgetState::Clicked {
                    if let Some(value) = ui.get_widget_value("DataEdit") {
                        if let Some(code) = value.to_string() {
                            // Compile the code to test for errors.
                            let ri = rusterix::RegionInstance::new(0);
                            match ri.execute(&code) {
                                Ok(_) => {
                                    ui.set_widget_value(
                                        "Build Result",
                                        ctx,
                                        TheValue::Text("Build OK".into()),
                                    );
                                }
                                Err(err) => {
                                    ui.set_widget_value(
                                        "Build Result",
                                        ctx,
                                        TheValue::Text(format!("Error: {err}")),
                                    );
                                }
                            }
                            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                                layout.relayout(ctx);
                            }
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "DataEdit" {
                    if let Some(code) = value.to_string() {
                        match server_ctx.cc {
                            ContentContext::CharacterTemplate(uuid) => {
                                if let Some(character) = project.characters.get_mut(&uuid) {
                                    character.data = code;
                                }
                            }
                            ContentContext::ItemTemplate(uuid) => {
                                if let Some(item) = project.items.get_mut(&uuid) {
                                    item.data = code;
                                }
                            }
                            ContentContext::Sector(uuid) => {
                                if let Some(map) = project.get_map_mut(server_ctx) {
                                    for sector in map.sectors.iter_mut() {
                                        if sector.creator_id == uuid {
                                            sector
                                                .properties
                                                .set("data", rusterix::Value::Str(code.clone()));
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
