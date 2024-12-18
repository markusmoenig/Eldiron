use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{PRERENDERTHREAD, TERRAINEDITOR, UNDOMANAGER};

pub struct TerrainSelectionTool {
    id: TheId,

    hover_pos: Option<Vec2i>,
    grid_size: i32,

    tile_selection: TileSelection,

    copied_area: FxHashSet<(i32, i32)>,
    copied_region: Option<Region>,
}

impl Tool for TerrainSelectionTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Terrain Select Tool"),

            hover_pos: None,
            grid_size: 0,

            tile_selection: TileSelection::default(),

            copied_area: FxHashSet::default(),
            copied_region: None,
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
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    self.grid_size = region.grid_size;
                }

                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Top);
                }

                if let Some(layout) = ui.get_hlayout("Terrain Tool Params") {
                    layout.clear();

                    let mut clear_area_button =
                        TheTraybarButton::new(TheId::named("Editor Clear Selection"));
                    clear_area_button.set_text(str!("Clear"));
                    //clear_area_button.limiter_mut().set_max_width(140);
                    clear_area_button
                        .set_status_text("Clears the current selection. Shortcut: 'Escape'.");

                    layout.add_widget(Box::new(clear_area_button));
                    layout.set_reverse_index(Some(1));
                }

                server_ctx.tile_selection = Some(self.tile_selection.clone());

                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_mode(TheSharedVLayoutMode::Shared);
                }
                // server_ctx.tile_selection = None;
                if let Some(layout) = ui.get_hlayout("Terrain Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                return true;
            }
            _ => {}
        };

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
                let p = (coord.x / self.grid_size, coord.y / self.grid_size);

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
                TERRAINEDITOR
                    .lock()
                    .unwrap()
                    .draw_selection(ui, ctx, server_ctx, None);
            }
            if let TileDrag(coord, _) = tool_event {
                let p = (coord.x / self.grid_size, coord.y / self.grid_size);
                if let Some(tile_selection) = &mut server_ctx.tile_selection {
                    tile_selection.grow_rect_by(p);
                    TERRAINEDITOR
                        .lock()
                        .unwrap()
                        .draw_selection(ui, ctx, server_ctx, None);
                }
            }
            if let TileUp = tool_event {
                if let Some(tile_selection) = &mut server_ctx.tile_selection {
                    self.tile_selection.tiles = tile_selection.merged();
                    TERRAINEDITOR
                        .lock()
                        .unwrap()
                        .draw_selection(ui, ctx, server_ctx, None);
                }
            }
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
            TheEvent::TileEditorHoverChanged(id, mut pos) => {
                if id.name == "TerrainMap View" {
                    pos = vec2i(pos.x / self.grid_size, pos.y / self.grid_size);

                    // Set hover position and repaint
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        self.grid_size = region.grid_size;
                        self.hover_pos = Some(pos);
                    }
                    TERRAINEDITOR.lock().unwrap().draw_selection(
                        ui,
                        ctx,
                        server_ctx,
                        self.hover_pos,
                    );

                    if self.copied_region.is_some() {
                        let mut sel = self.tile_selection.clone();

                        // Remap the copied area to the new pos for selection preview
                        sel.tiles.clear();
                        for t in &self.copied_area {
                            sel.tiles.insert((pos.x + t.0, pos.y + t.1));
                        }
                        server_ctx.tile_selection = Some(sel);

                        return true;
                    }
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
            _ => false,
        }
    }
}
