use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{CODEEDITOR, SIDEBARMODE, TILEDRAWER, TILEFXEDITOR};

pub struct PickerTool {
    id: TheId,
}

impl Tool for PickerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Picker Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Picker Tool (K). Pick content in the region editor.")
    }
    fn icon_name(&self) -> String {
        str!("picker")
    }
    fn accel(&self) -> Option<char> {
        Some('k')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
                return true;
            }
            _ => {
                return false;
            }
        };

        //let mut clicked_tile = false;
        let mut found_geo = false;

        // Check for character at the given position.
        if let Some(c) = server.get_character_at(server_ctx.curr_region, coord) {
            server_ctx.curr_character_instance = Some(c.0);
            server_ctx.curr_character = Some(c.1);
            server_ctx.curr_area = None;
            server_ctx.curr_item_instance = None;
            server_ctx.curr_item = None;

            // Set 3D editing position to Zero.
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                region.editing_position_3d = Vec3f::zero();
                server.set_editing_position_3d(region.editing_position_3d);
            }

            if let Some(layout) = ui.get_list_layout("Region Content List") {
                layout.select_item(c.0, ctx, false);
            }

            if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
            {
                // In Region mode, we need to set the character bundle of the current character instance.
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    if let Some(character) = region.characters.get(&c.0) {
                        for grid in character.instance.grids.values() {
                            if grid.name == "init" {
                                CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                CODEEDITOR.lock().unwrap().code_id = str!("Character Instance");
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Set CodeGrid Panel"),
                                    TheValue::Empty,
                                ));
                                //self.set_editor_group_index(EditorMode::Code, ui, ctx);
                            }
                        }
                    }
                }
            }
            //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
            // In Character mode, we need to set the character bundle of the current character.
            //}
        }
        // Check for an item at the given position.
        else if let Some(c) = server.get_item_at(server_ctx.curr_region, coord) {
            server_ctx.curr_character_instance = None;
            server_ctx.curr_character = None;
            server_ctx.curr_item_instance = Some(c.0);
            server_ctx.curr_item = Some(c.1);
            server_ctx.curr_area = None;

            if let Some(layout) = ui.get_list_layout("Region Content List") {
                layout.select_item(c.0, ctx, false);
            }

            if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                || *SIDEBARMODE.lock().unwrap() == SidebarMode::Item
            {
                // In Region mode, we need to set the character bundle of the current character instance.
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    if let Some(item) = region.items.get(&c.0) {
                        for grid in item.instance.grids.values() {
                            if grid.name == "init" {
                                CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                CODEEDITOR.lock().unwrap().code_id = str!("Item Instance");
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Set CodeGrid Panel"),
                                    TheValue::Empty,
                                ));
                                //self.set_editor_group_index(EditorMode::Code, ui, ctx);
                            }
                        }
                    }
                }
            }
            //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
            // In Character mode, we need to set the character bundle of the current character.
            //}
        } else if let Some(region) = project.get_region(&server_ctx.curr_region) {
            let found_area = false;

            // Check for area at the given position.
            // for area in region.areas.values() {
            //     if area.area.contains(&(coord.x, coord.y)) {
            //         for grid in area.bundle.grids.values() {
            //             if grid.name == "main" {
            //                 if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
            //                     || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
            //                 {
            //                     CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
            //                     ctx.ui.send(TheEvent::Custom(
            //                         TheId::named("Set CodeGrid Panel"),
            //                         TheValue::Empty,
            //                     ));
            //                 }
            //                 found_area = true;
            //                 server_ctx.curr_character_instance = None;
            //                 server_ctx.curr_character = None;
            //                 server_ctx.curr_area = Some(area.id);
            //                 if let Some(layout) = ui.get_list_layout("Region Content List") {
            //                     layout.select_item(area.id, ctx, false);
            //                 }
            //                 break;
            //             }
            //         }
            //     }
            // }

            if !found_area {
                // No area, set the tile.

                server_ctx.curr_character_instance = None;
                if tool_context == ToolContext::TwoD {
                    // Test against object SDFs float position in 2d
                    if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                        if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                            let p = rgba_view.float_pos();
                            if let Some((obj, node_index)) =
                                region.get_closest_geometry(p, server_ctx.curr_layer_role)
                            {
                                if let Some(geo) = region.geometry.get(&obj) {
                                    server_ctx.curr_geo_object = Some(geo.id);
                                    server_ctx.curr_geo_node = Some(geo.nodes[node_index].id);

                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Set Region Modeler"),
                                        TheValue::Empty,
                                    ));
                                    found_geo = true;
                                }
                            }
                        }
                    }
                } else if let Some((obj, node_index)) =
                    region.get_closest_geometry(Vec2f::from(coord), server_ctx.curr_layer_role)
                {
                    if let Some(geo) = region.geometry.get(&obj) {
                        server_ctx.curr_geo_object = Some(geo.id);
                        server_ctx.curr_geo_node = Some(geo.nodes[node_index].id);

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Set Region Modeler"),
                            TheValue::Empty,
                        ));
                        found_geo = true;
                    }
                }

                if !found_geo {
                    if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                        if server_ctx.curr_layer_role == Layer2DRole::FX {
                            // Set the tile preview.
                            if let Some(widget) = ui.get_widget("TileFX RGBA") {
                                if let Some(tile_rgba) = widget.as_rgba_view() {
                                    if let Some(tile) = project.extract_region_tile(
                                        server_ctx.curr_region,
                                        (coord.x, coord.y),
                                    ) {
                                        let preview_size =
                                            TILEFXEDITOR.lock().unwrap().preview_size;
                                        tile_rgba.set_grid(Some(
                                            preview_size / tile.buffer[0].dim().width,
                                        ));
                                        tile_rgba.set_buffer(
                                            tile.buffer[0].scaled(preview_size, preview_size),
                                        );
                                    }
                                }
                            }
                            if let Some(timeline) = &tile.tilefx {
                                TILEFXEDITOR
                                    .lock()
                                    .unwrap()
                                    .set_timeline(timeline.clone(), ui);
                            }
                        } else {
                            for uuid in tile.layers.iter().flatten() {
                                if TILEDRAWER.lock().unwrap().tiles.contains_key(uuid) {
                                    ctx.ui.send(TheEvent::StateChanged(
                                        TheId::named_with_id("Tilemap Tile", *uuid),
                                        TheWidgetState::Selected,
                                    ));
                                    //clicked_tile = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        // MODELFXEDITOR
        //     .lock()
        //     .unwrap()
        //     .set_geo_node_ui(server_ctx, project, ui, ctx);
        // if clicked_tile {
        //     self.set_editor_group_index(EditorMode::Draw, ui, ctx);
        // }
        false
    }
}
