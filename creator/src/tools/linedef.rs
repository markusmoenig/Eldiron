use crate::prelude::*;
use shared::server::prelude::MapToolType;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct LinedefTool {
    id: TheId,
    click_pos: Vec2f,
    rectangle_undo_map: Map,
    rectangle_mode: bool,
}

impl Tool for LinedefTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Linedef Tool"),
            click_pos: Vec2f::zero(),
            rectangle_undo_map: Map::default(),
            rectangle_mode: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Linedef Tool (L). Create line definitions and sectors.")
    }
    fn icon_name(&self) -> String {
        str!("line-segment")
    }
    fn accel(&self) -> Option<char> {
        Some('l')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let _coord = match tool_event {
            TileDown(_, c) => c,
            TileDrag(_, c) => c,
            Activate => {
                // Display the tile edit panel.
                ctx.ui
                    .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));

                if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
                    layout.set_mode(TheSharedHLayoutMode::Right);
                    ctx.ui.relayout = true;
                }

                server_ctx.curr_character_instance = None;
                server_ctx.curr_item_instance = None;
                server_ctx.curr_area = None;
                server_ctx.curr_map_tool_type = MapToolType::Linedef;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.selected_vertices.clear();
                    region.map.selected_sectors.clear();
                    server.update_region(region);
                }

                return true;
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.clear_temp();
                    server.update_region(region);
                }
                return true;
            }
            _ => {
                return false;
            }
        };

        /*
        // When we draw in 2D, move the 3D view to the pen position
        if tool_context == ToolContext::TwoD && server_ctx.curr_character_instance.is_none() {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                region.editing_position_3d = Vec3f::new(coord.x as f32, 0.0, coord.y as f32);
                server.set_editing_position_3d(region.editing_position_3d);
            }
        }

        if let Some(curr_tile_id) = server_ctx.curr_tile_id {
            if let Some(rgb_tile) = TILEDRAWER.lock().unwrap().tiles.get(&curr_tile_id) {
                let is_billboard = rgb_tile.billboard;
                if server_ctx.curr_layer_role == Layer2DRole::FX {
                    // Set the tile preview.
                    if let Some(widget) = ui.get_widget("TileFX RGBA") {
                        if let Some(tile_rgba) = widget.as_rgba_view() {
                            if let Some(tile) = project
                                .extract_region_tile(server_ctx.curr_region, (coord.x, coord.y))
                            {
                                let preview_size = TILEFXEDITOR.lock().unwrap().preview_size;
                                tile_rgba.set_grid(Some(preview_size / tile.buffer[0].dim().width));
                                tile_rgba
                                    .set_buffer(tile.buffer[0].scaled(preview_size, preview_size));
                            }
                        }
                    }
                }

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    let mut tiles_to_render: Vec<Vec2i> = vec![];

                    let mut prev = None;
                    if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                        prev = Some(tile.clone());
                    }

                    region.set_tile(
                        (coord.x, coord.y),
                        server_ctx.curr_layer_role,
                        server_ctx.curr_tile_id,
                    );

                    tiles_to_render.push(coord);
                    let region_to_render = Some(region.clone());

                    if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                        if prev != Some(tile.clone()) {
                            let undo = RegionUndoAtom::RegionTileEdit(
                                vec2i(coord.x, coord.y),
                                prev,
                                Some(tile.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);
                        }
                    }
                    //self.set_icon_previews(region, &palette, coord, ui);

                    server.update_region(region);

                    if !is_billboard {
                        RENDERER.lock().unwrap().set_region(region);

                        if let Some(region) = region_to_render {
                            PRERENDERTHREAD
                                .lock()
                                .unwrap()
                                .render_region(region, Some(tiles_to_render));
                        }
                    }
                }
            }
            //self.redraw_region(ui, server, ctx, server_ctx);
        }
        */
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
        let mut redraw = false;
        match event {
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "PolyView" {
                    if server_ctx.hover.1.is_some() {
                        // Selected hovered line
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let prev = region.map.clone();
                            let mut changed = false;

                            if ui.shift {
                                // Add
                                if let Some(l) = server_ctx.hover.1 {
                                    if !region.map.selected_linedefs.contains(&l) {
                                        region.map.selected_linedefs.push(l);
                                        changed = true;
                                    }
                                }
                            } else if ui.alt {
                                // Subtract
                                if let Some(l) = server_ctx.hover.1 {
                                    region
                                        .map
                                        .selected_linedefs
                                        .retain(|&selected| selected != l);
                                    changed = true;
                                }
                            } else {
                                // Replace
                                if let Some(v) = server_ctx.hover.1 {
                                    region.map.selected_linedefs = vec![v];
                                    changed = true;
                                } else {
                                    region.map.selected_linedefs.clear();
                                    changed = true;
                                }
                            }

                            if changed {
                                let undo = RegionUndoAtom::MapEdit(
                                    Box::new(prev),
                                    Box::new(region.map.clone()),
                                );

                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);

                                server.update_region(region);
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Map Selection Changed"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    } else {
                        // Line mode
                        let mut set_current_gid_pos = true;
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                let grid_pos = server_ctx.local_to_map_grid(
                                    vec2f(dim.width as f32, dim.height as f32),
                                    vec2f(coord.x as f32, coord.y as f32),
                                    &region.map,
                                    region.map.subdivisions,
                                );

                                if let Some(curr_grid_pos) = region.map.curr_grid_pos {
                                    if curr_grid_pos.x != grid_pos.x
                                        || curr_grid_pos.y != grid_pos.y
                                    {
                                        let prev = region.map.clone();

                                        let start_vertex = region
                                            .map
                                            .add_vertex_at(curr_grid_pos.x, curr_grid_pos.y);
                                        let end_vertex =
                                            region.map.add_vertex_at(grid_pos.x, grid_pos.y);

                                        // Returns id of linedef and optional id of new sector if polygon closes
                                        let ids =
                                            region.map.create_linedef(start_vertex, end_vertex);

                                        if ids.1.is_some() {
                                            // When we close a polygon delete the temporary data
                                            region.map.clear_temp();
                                            set_current_gid_pos = false;
                                        }

                                        server.update_region(region);

                                        let undo = RegionUndoAtom::MapEdit(
                                            Box::new(prev),
                                            Box::new(region.map.clone()),
                                        );

                                        UNDOMANAGER
                                            .lock()
                                            .unwrap()
                                            .add_region_undo(&region.id, undo, ctx);
                                    }
                                }

                                if set_current_gid_pos {
                                    region.map.curr_grid_pos = Some(grid_pos);
                                }
                            }
                        }
                    }

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.click_pos = vec2f(coord.x as f32, coord.y as f32);
                        self.rectangle_undo_map = region.map.clone();
                        self.rectangle_mode = false;
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            if !self.rectangle_mode {
                                region.map.curr_mouse_pos =
                                    Some(vec2f(coord.x as f32, coord.y as f32));
                            }
                            let mut hover = server_ctx.geometry_at(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                            );
                            hover.0 = None;
                            hover.2 = None;
                            server_ctx.hover = hover;

                            let cp = server_ctx.local_to_map_grid(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                                region.map.subdivisions,
                            );

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Cursor Pos Changed"),
                                TheValue::Float2(cp),
                            ));
                            server_ctx.hover_cursor = Some(cp);

                            server.update_region(region);
                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    if !self.rectangle_mode {
                        let dist = distance(self.click_pos, vec2f(coord.x as f32, coord.y as f32));
                        if dist > 10.0 {
                            self.rectangle_mode = true;
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                region.map.clear_temp();
                                server.update_region(region);
                            }
                        }
                    }

                    if self.rectangle_mode {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if let Some(render_view) = ui.get_render_view("PolyView") {
                                let dim = *render_view.dim();
                                let click_pos = server_ctx.local_to_map_grid(
                                    vec2f(dim.width as f32, dim.height as f32),
                                    self.click_pos,
                                    &region.map,
                                    region.map.subdivisions,
                                );
                                let drag_pos = server_ctx.local_to_map_grid(
                                    vec2f(dim.width as f32, dim.height as f32),
                                    vec2f(coord.x as f32, coord.y as f32),
                                    &region.map,
                                    region.map.subdivisions,
                                );

                                let top_left = Vec2f::new(
                                    click_pos.x.min(drag_pos.x),
                                    click_pos.y.min(drag_pos.y),
                                );
                                let bottom_right = Vec2f::new(
                                    click_pos.x.max(drag_pos.x),
                                    click_pos.y.max(drag_pos.y),
                                );

                                let mut selection = server_ctx.geometry_in_rectangle(
                                    top_left,
                                    bottom_right,
                                    &region.map,
                                );

                                selection.0 = vec![];
                                selection.2 = vec![];

                                region.map = self.rectangle_undo_map.clone();
                                region.map.curr_grid_pos = None;
                                region.map.curr_rectangle =
                                    Some((self.click_pos, vec2f(coord.x as f32, coord.y as f32)));

                                if ui.shift {
                                    // Add
                                    region.map.add_to_selection(
                                        selection.0,
                                        selection.1,
                                        selection.2,
                                    );
                                } else if ui.alt {
                                    // Remove
                                    region.map.remove_from_selection(
                                        selection.0,
                                        selection.1,
                                        selection.2,
                                    );
                                } else {
                                    // Replace
                                    region.map.selected_linedefs = selection.1;
                                }

                                server.update_region(region);
                            }
                        }
                    }
                }
                redraw = true;
            }
            TheEvent::RenderViewUp(id, _coord) => {
                if self.rectangle_mode && id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if region.map.curr_rectangle.is_some() {
                            region.map.clear_temp();
                            self.rectangle_mode = false;
                            server.update_region(region);

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(self.rectangle_undo_map.clone()),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                redraw = true;
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                #[allow(clippy::collapsible_if)]
                if *code == TheKeyCode::Escape {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.map.clear_temp();
                        server.update_region(region);
                        // Hover is empty, check if we need to clear selection
                        if !region.map.selected_linedefs.is_empty() {
                            let prev = region.map.clone();

                            region.map.selected_linedefs.clear();

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(prev),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                if *code == TheKeyCode::Delete {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if !region.map.selected_linedefs.is_empty() {
                            let prev = region.map.clone();
                            let lines = region.map.selected_linedefs.clone();

                            #[allow(clippy::useless_vec)]
                            region.map.delete_elements(&vec![], &lines, &vec![]);
                            region.map.selected_linedefs.clear();

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(prev),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                }
                redraw = true;
            }
            _ => {}
        }
        redraw
    }
}
