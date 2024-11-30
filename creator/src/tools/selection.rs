use crate::prelude::*;
use ToolEvent::*;

use crate::editor::UNDOMANAGER;

pub struct SelectionTool {
    id: TheId,

    tile_selection: TileSelection,
    //copied_area: FxHashSet<(i32, i32)>,
    //copied_region: Option<Region>,
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

                    let mut clear_area_button =
                        TheTraybarButton::new(TheId::named("Editor Clear Selection"));
                    clear_area_button.set_text(str!("Clear"));
                    //clear_area_button.limiter_mut().set_max_width(140);
                    clear_area_button
                        .set_status_text("Clears the current selection. Shortcut: 'Escape'.");

                    layout.add_widget(Box::new(clear_area_button));
                    // layout.set_reverse_index(Some(1));
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
                ui.set_widget_context_menu("Region Editor View", None);
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
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
        match event {
            TheEvent::RenderViewClicked(id, _coord) => {
                #[allow(clippy::collapsible_if)]
                if id.name == "PolyView" {
                    if !server_ctx.hover_is_empty() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let prev = region.map.clone();
                            if ui.shift {
                                // Add
                                if let Some(v) = server_ctx.hover.0 {
                                    if !region.map.selected_vertices.contains(&v) {
                                        region.map.selected_vertices.push(v);
                                    }
                                }
                                if let Some(l) = server_ctx.hover.1 {
                                    if !region.map.selected_linedefs.contains(&l) {
                                        region.map.selected_linedefs.push(l);
                                    }
                                }
                                if let Some(s) = server_ctx.hover.2 {
                                    if !region.map.selected_sectors.contains(&s) {
                                        region.map.selected_sectors.push(s);
                                    }
                                }
                            } else if ui.alt {
                                // Subtract
                                if let Some(v) = server_ctx.hover.0 {
                                    region
                                        .map
                                        .selected_vertices
                                        .retain(|&selected| selected != v);
                                }
                                if let Some(l) = server_ctx.hover.1 {
                                    region
                                        .map
                                        .selected_linedefs
                                        .retain(|&selected| selected != l);
                                }
                                if let Some(s) = server_ctx.hover.2 {
                                    region
                                        .map
                                        .selected_sectors
                                        .retain(|&selected| selected != s);
                                }
                            } else {
                                // Replace
                                if let Some(v) = server_ctx.hover.0 {
                                    region.map.selected_vertices = vec![v];
                                } else {
                                    region.map.selected_vertices.clear();
                                }
                                if let Some(l) = server_ctx.hover.1 {
                                    region.map.selected_linedefs = vec![l];
                                } else {
                                    region.map.selected_linedefs.clear();
                                }
                                if let Some(s) = server_ctx.hover.2 {
                                    region.map.selected_sectors = vec![s];
                                } else {
                                    region.map.selected_sectors.clear();
                                }
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
                        }
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
                        }
                    }
                }
                true
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(code)) => {
                #[allow(clippy::collapsible_if)]
                if *code == TheKeyCode::Escape {
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

                            let undo = RegionUndoAtom::MapEdit(
                                Box::new(prev),
                                Box::new(region.map.clone()),
                            );

                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);

                            server.update_region(region);
                        }
                    }
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
