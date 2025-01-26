use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

pub struct SelectionTool {
    id: TheId,
    click_pos: Vec2<f32>,
    rectangle_undo_map: Map,
}

impl Tool for SelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Select Tool"),
            click_pos: Vec2::zero(),
            rectangle_undo_map: Map::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        if cfg!(target_os = "macos") {
            str!(
                "Selection Tool (S). Hold 'Shift' to add. 'Option' to subtract. Click and drag for multi-selection."
            )
        } else {
            str!("Selection Tool (S). Hold 'Shift' to add. 'Alt' to subtract. Click and drag for multi-selection.")
        }
    }
    fn icon_name(&self) -> String {
        str!("cursor")
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    server_ctx.curr_map_tool_type = MapToolType::Selection;
                }

                // ui.set_widget_context_menu(
                //     "Region Editor View",
                //     Some(TheContextMenu {
                //         items: vec![TheContextMenuItem::new(
                //             "Create Area...".to_string(),
                //             TheId::named("Create Area"),
                //         )],
                //         ..Default::default()
                //     }),
                // );

                // server_ctx.tile_selection = Some(self.tile_selection.clone());

                return true;
            }
            DeActivate => {
                //ui.set_widget_context_menu("Region Editor View", None);
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                server_ctx.curr_map_tool_type = MapToolType::General;
                return true;
            }
            _ => {}
        };
        /*
                if let Some(copied) = &self.copied_region {
                    // Handle copied region

                    if let TileDown(coord, _) = tool_event {
                        // Copy the copied region into the selection.

                        // The tiles in the transformed coord space.
                        let mut tiles = FxHashSet::default();
                        for t in &self.copied_area {
                            tiles.insert((coord.x + t.0, coord.y + t.1));
                        }

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let prev = region.clone();

                            // Copy geometry
                            for geo_obj in copied.geometry.values() {
                                let p = geo_obj.get_position();

                                let toffset = Vec2f::from(p) + vec2f(coord.x as f32, coord.y as f32);
                                let mut c = geo_obj.clone();
                                c.id = Uuid::new_v4();
                                c.set_position(toffset);
                                c.update_area();

                                // Insert into new region
                                region.geometry.insert(c.id, c);
                            }

                            // Copy the tiles
                            for (tile_pos, tile) in &copied.tiles {
                                let p = vec2i(tile_pos.0, tile_pos.1);
                                let pos = p + coord;
                                region.tiles.insert((pos.x, pos.y), tile.clone());
                            }

                            // Copy the heightmap content
                            for (tile_pos, tile) in &copied.heightmap.material_mask {
                                let p = vec2i(tile_pos.0, tile_pos.1);
                                let pos = p + coord;
                                region
                                    .heightmap
                                    .material_mask
                                    .insert((pos.x, pos.y), tile.clone());
                            }

                            region.update_geometry_areas();
                            server.update_region(region);

                            let tiles_vector: Vec<Vec2i> =
                                tiles.into_iter().map(|(x, y)| Vec2i::new(x, y)).collect();

                            // Undo
                            let undo = RegionUndoAtom::RegionEdit(
                                Box::new(prev),
                                Box::new(region.clone()),
                                tiles_vector.clone(),
                            );
                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            // Render
                            PRERENDERTHREAD
                                .lock()
                                .unwrap()
                                .render_region(region.clone(), Some(tiles_vector));
                        }
                    }
                } else {
                    // Handle general selection

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
                }
        */
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
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                if !server_ctx.hover_is_empty() {
                    let prev = map.clone();
                    let arrays = server_ctx.hover_to_arrays();
                    if ui.shift {
                        // Add
                        map.add_to_selection(arrays.0, arrays.1, arrays.2);
                    } else if ui.alt {
                        // Remove
                        map.remove_from_selection(arrays.0, arrays.1, arrays.2);
                    } else {
                        // Replace
                        map.selected_vertices = arrays.0;
                        map.selected_linedefs = arrays.1;
                        map.selected_sectors = arrays.2;
                    }

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }

                self.click_pos = Vec2::new(coord.x as f32, coord.y as f32);
                self.rectangle_undo_map = map.clone();
            }
            MapDragged(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    let click_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        self.click_pos,
                        map,
                        map.subdivisions,
                    );
                    let drag_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );

                    let top_left =
                        Vec2::new(click_pos.x.min(drag_pos.x), click_pos.y.min(drag_pos.y));
                    let bottom_right =
                        Vec2::new(click_pos.x.max(drag_pos.x), click_pos.y.max(drag_pos.y));

                    let selection = server_ctx.geometry_in_rectangle(top_left, bottom_right, map);

                    *map = self.rectangle_undo_map.clone();
                    map.curr_rectangle = Some((
                        vek::Vec2::new(self.click_pos.x, self.click_pos.y),
                        vek::Vec2::new(coord.x as f32, coord.y as f32),
                    ));

                    if ui.shift {
                        // Add
                        map.add_to_selection(selection.0, selection.1, selection.2);
                    } else if ui.alt {
                        // Remove
                        map.remove_from_selection(selection.0, selection.1, selection.2);
                    } else {
                        // Replace
                        map.selected_vertices = selection.0;
                        map.selected_linedefs = selection.1;
                        map.selected_sectors = selection.2;
                    }
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapUp(_) => {
                if map.curr_rectangle.is_some() {
                    map.curr_rectangle = None;

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(self.rectangle_undo_map.clone()),
                        Box::new(map.clone()),
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
            MapHover(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    server_ctx.hover = server_ctx.geometry_at(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                    );

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
            MapDelete => {
                if !map.selected_vertices.is_empty()
                    || !map.selected_linedefs.is_empty()
                    || !map.selected_sectors.is_empty()
                {
                    let prev = map.clone();

                    let vertices = map.selected_vertices.clone();
                    let linedefs = map.selected_linedefs.clone();
                    let sectors = map.selected_sectors.clone();

                    map.delete_elements(&vertices, &linedefs, &sectors);
                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
                    map.selected_sectors.clear();

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
            MapEscape => {
                // Hover is empty, check if we need to clear selection
                if !map.selected_vertices.is_empty()
                    || !map.selected_linedefs.is_empty()
                    || !map.selected_sectors.is_empty()
                {
                    let prev = map.clone();

                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
                    map.selected_sectors.clear();

                    undo_atom = Some(RegionUndoAtom::MapEdit(
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
            }
        }

        undo_atom
    }

    fn handle_event(
        &mut self,
        _event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }
}
