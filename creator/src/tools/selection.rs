use crate::prelude::*;
use shared::server::prelude::MapToolType;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct SelectionTool {
    id: TheId,
    click_pos: Vec2f,
    rectangle_undo_map: Map,
}

impl Tool for SelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Select Tool"),
            click_pos: Vec2f::zero(),
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
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut clear_selection_button =
                        TheTraybarButton::new(TheId::named("Editor Clear Selection"));
                    clear_selection_button.set_text(str!("Clear"));
                    clear_selection_button
                        .set_status_text("Clears the current selection. Shortcut: 'Escape'.");
                    layout.add_widget(Box::new(clear_selection_button));

                    let mut delete_selection_button =
                        TheTraybarButton::new(TheId::named("Editor Delete Selection"));
                    delete_selection_button.set_text(str!("Delete"));
                    delete_selection_button
                        .set_status_text("Deletes the current selection. Shortcut: 'Delete'.");
                    layout.add_widget(Box::new(delete_selection_button));
                    layout.set_reverse_index(Some(1));

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
        let mut clear_selection = || {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                // Hover is empty, check if we need to clear selection
                if !region.map.selected_vertices.is_empty()
                    || !region.map.selected_linedefs.is_empty()
                    || !region.map.selected_sectors.is_empty()
                {
                    let prev = region.map.clone();

                    region.map.selected_vertices.clear();
                    region.map.selected_linedefs.clear();
                    region.map.selected_sectors.clear();

                    let undo =
                        RegionUndoAtom::MapEdit(Box::new(prev), Box::new(region.map.clone()));

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
        };

        match event {
            TheEvent::RenderViewClicked(id, coord) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "PolyView" {
                    if !server_ctx.hover_is_empty() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let prev = region.map.clone();
                            let arrays = server_ctx.hover_to_arrays();
                            if ui.shift {
                                // Add
                                region.map.add_to_selection(arrays.0, arrays.1, arrays.2);
                            } else if ui.alt {
                                // Remove
                                region
                                    .map
                                    .remove_from_selection(arrays.0, arrays.1, arrays.2);
                            } else {
                                // Replace
                                region.map.selected_vertices = arrays.0;
                                region.map.selected_linedefs = arrays.1;
                                region.map.selected_sectors = arrays.2;
                            }

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

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.click_pos = vec2f(coord.x as f32, coord.y as f32);
                        self.rectangle_undo_map = region.map.clone();
                    }
                }
                true
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            server_ctx.hover = server_ctx.geometry_at(
                                vec2f(dim.width as f32, dim.height as f32),
                                vec2f(coord.x as f32, coord.y as f32),
                                &region.map,
                            );

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
                        }
                    }
                }
                true
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
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

                            let selection = server_ctx.geometry_in_rectangle(
                                top_left,
                                bottom_right,
                                &region.map,
                            );

                            region.map = self.rectangle_undo_map.clone();
                            region.map.curr_rectangle =
                                Some((self.click_pos, vec2f(coord.x as f32, coord.y as f32)));

                            if ui.shift {
                                // Add
                                region
                                    .map
                                    .add_to_selection(selection.0, selection.1, selection.2);
                            } else if ui.alt {
                                // Remove
                                region.map.remove_from_selection(
                                    selection.0,
                                    selection.1,
                                    selection.2,
                                );
                            } else {
                                // Replace
                                region.map.selected_vertices = selection.0;
                                region.map.selected_linedefs = selection.1;
                                region.map.selected_sectors = selection.2;
                            }

                            server.update_region(region);
                        }
                    }
                }
                true
            }
            TheEvent::RenderViewUp(id, _coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if region.map.curr_rectangle.is_some() {
                            region.map.curr_rectangle = None;
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
                true
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                #[allow(clippy::collapsible_if)]
                if *code == TheKeyCode::Escape {
                    clear_selection();
                }
                if *code == TheKeyCode::Delete {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if !region.map.selected_vertices.is_empty()
                            || !region.map.selected_linedefs.is_empty()
                            || !region.map.selected_sectors.is_empty()
                        {
                            let prev = region.map.clone();

                            let vertices = region.map.selected_vertices.clone();
                            let linedefs = region.map.selected_linedefs.clone();
                            let sectors = region.map.selected_sectors.clone();

                            region.map.delete_elements(&vertices, &linedefs, &sectors);
                            region.map.selected_vertices.clear();
                            region.map.selected_linedefs.clear();
                            region.map.selected_sectors.clear();

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
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Editor Clear Selection" {
                    clear_selection();
                } else if id.name == "Editor Delete Selection" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if !region.map.selected_vertices.is_empty()
                            || !region.map.selected_linedefs.is_empty()
                            || !region.map.selected_sectors.is_empty()
                        {
                            let prev = region.map.clone();

                            let vertices = region.map.selected_vertices.clone();
                            let linedefs = region.map.selected_linedefs.clone();
                            let sectors = region.map.selected_sectors.clone();

                            region.map.delete_elements(&vertices, &linedefs, &sectors);
                            region.map.selected_vertices.clear();
                            region.map.selected_linedefs.clear();
                            region.map.selected_sectors.clear();

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
                true
            }
            /*
            TheEvent::Cut | TheEvent::Copy => {
                if self.tile_selection.tiles.is_empty() {
                    return false;
                }

                let tiles = self.tile_selection.tiles.clone();

                // Cut / Copy terrain
                let is_cut = matches!(*event, TheEvent::Cut);

                let (sel_min, _, _, _) = self.tile_selection.tile_dimensions().unwrap();

                // The new region we copy into
                let mut copied = Region::default();
                self.copied_area.clear();

                let mut geo_obj_to_remove = vec![];
                let mut tiles_to_remove = vec![];
                let mut heightmap_material_mask_to_remove = vec![];

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    // Copy the geometry objects
                    for (id, geo_obj) in &region.geometry {
                        let p = geo_obj.get_position();
                        let tp = Vec2i::from(p);

                        // Inside the selection
                        if tiles.contains(&(tp.x, tp.y)) {
                            if is_cut {
                                geo_obj_to_remove.push(*id);
                            }

                            let toffset = Vec2f::from(p) - Vec2f::from(sel_min);
                            let mut c = geo_obj.clone();
                            c.id = Uuid::new_v4();
                            c.set_position(toffset);
                            c.update_area();

                            let pos = tp - sel_min;
                            self.copied_area.insert((pos.x, pos.y));

                            // Insert into new region
                            copied.geometry.insert(c.id, c);
                        };
                    }

                    // Copy the tiles
                    for (tile_pos, tile) in &region.tiles {
                        if tiles.contains(tile_pos) {
                            let p = vec2i(tile_pos.0, tile_pos.1);
                            let pos = p - sel_min;

                            tiles_to_remove.push(*tile_pos);

                            self.copied_area.insert((pos.x, pos.y));
                            copied.tiles.insert((pos.x, pos.y), tile.clone());
                        }
                    }

                    // Copy heightmap content
                    for (tile_pos, tile) in &region.heightmap.material_mask {
                        if tiles.contains(tile_pos) {
                            let p = vec2i(tile_pos.0, tile_pos.1);
                            let pos = p - sel_min;

                            heightmap_material_mask_to_remove.push(*tile_pos);
                            self.copied_area.insert((pos.x, pos.y));
                            copied
                                .heightmap
                                .material_mask
                                .insert((pos.x, pos.y), tile.clone());
                        }
                    }

                    // When cutting remove from old region
                    if is_cut {
                        let prev = region.clone();

                        for id in geo_obj_to_remove {
                            region.geometry.remove(&id);
                        }

                        for t in tiles_to_remove {
                            region.tiles.remove(&t);
                        }

                        for t in heightmap_material_mask_to_remove {
                            region.heightmap.material_mask.remove(&t);
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

                        PRERENDERTHREAD
                            .lock()
                            .unwrap()
                            .render_region(region.clone(), Some(tiles_vector));
                    }

                    self.copied_region = Some(copied);

                    true
                } else {
                    false
                }
            }
            TheEvent::TileEditoHoverChanged(id, pos) => {
                if id.name == "Region Editor View" && self.copied_region.is_some() {
                    let mut sel = self.tile_selection.clone();

                    // Remap the copied area to the new pos for selection preview
                    sel.tiles.clear();
                    for t in &self.copied_area {
                        sel.tiles.insert((pos.x + t.0, pos.y + t.1));
                    }
                    server_ctx.tile_selection = Some(sel);

                    return true;
                }
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
                    self.copied_region = None;
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

                    self.copied_region = None;

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
            }*/
            _ => false,
        }
    }
}
