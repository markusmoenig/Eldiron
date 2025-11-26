use crate::docks::tiles_editor_undo::*;
use crate::editor::TOOLLIST;
use crate::prelude::*;

pub struct TilesEditorDock {
    zoom: f32,
    tile_node: Uuid,
    palette_node: Uuid,

    // Per-tile undo stacks
    tile_undos: FxHashMap<Uuid, TileEditorUndo>,
    current_tile_id: Option<Uuid>,
    max_undo: usize,
}

impl Dock for TilesEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            zoom: 5.0,
            tile_node: Uuid::new_v4(),
            palette_node: Uuid::new_v4(),
            tile_undos: FxHashMap::default(),
            current_tile_id: None,
            max_undo: 30,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut rgba_layout = TheRGBALayout::new(TheId::named("Tile Editor Dock RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_supports_external_zoom(true);
            rgba_view.set_background([116, 116, 116, 255]);
            // rgba_view.set_grid(Some(1));
            // rgba_view.set_grid_color([20, 20, 20, 255]);
            // rgba_view.set_dont_show_grid(true);
            rgba_view.set_show_transparency(true);
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        canvas.set_layout(rgba_layout);

        // Tree

        let mut palette_canvas = TheCanvas::default();
        let mut palette_tree_layout = TheTreeLayout::new(TheId::named("Tile Editor Tree"));
        palette_tree_layout.limiter_mut().set_max_width(305);
        let root = palette_tree_layout.get_root();

        // Tile
        let mut tile_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Tile", self.tile_node));
        tile_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Tile Size"));
        item.set_text("Size".into());

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Size Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Tile Frames"));
        item.set_text("Frames".into());

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Frame Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeIcons::new(TheId::named("Tile Frame Icons"));
        item.set_icon_size(40);
        item.set_icon_count(1);
        item.set_selected_index(Some(0));
        tile_node.add_widget(Box::new(item));

        root.add_child(tile_node);

        // Palette

        let mut palette_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Palette", self.palette_node));
        palette_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Palette Opacity"));
        item.set_text("Opacity".into());

        let mut edit = TheTextLineEdit::new(TheId::named("Palette Opacity Edit"));
        edit.set_value(TheValue::Float(1.0));
        edit.set_range(TheValue::RangeF32(0.0..=1.0));
        item.add_widget_column(150, Box::new(edit));
        palette_node.add_widget(Box::new(item));

        let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        item.set_icon_count(256);
        item.set_icons_per_row(14);
        item.set_selected_index(Some(0));

        palette_node.add_widget(Box::new(item));
        root.add_child(palette_node);

        palette_canvas.set_layout(palette_tree_layout);

        canvas.set_left(palette_canvas);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                self.set_tile(tile, ui, ctx, server_ctx, false);
            }
        }

        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
            if let Some(palette_node) = tree_layout.get_node_by_id_mut(&self.palette_node) {
                if let Some(widget) = palette_node.widgets[1].as_tree_icons() {
                    widget.set_palette(&project.palette);
                }
            }
        }
    }

    fn minimized(&mut self, _ui: &mut TheUI, ctx: &mut TheContext) {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
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

        match event {
            TheEvent::Custom(id, value) => {
                if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Picked"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, false);
                    }
                } else if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Updated"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, true);

                        // Update the current frame
                        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node)
                            {
                                // Update the frame icon
                                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                    if server_ctx.curr_tile_frame_index < tile.textures.len() {
                                        widget.set_icon(
                                            server_ctx.curr_tile_frame_index,
                                            tile.textures[server_ctx.curr_tile_frame_index]
                                                .to_rgba(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Tile Editor Undo Available" {
                    if let Some(atom) = TOOLLIST
                        .write()
                        .unwrap()
                        .get_current_editor_tool()
                        .get_undo_atom(project)
                    {
                        if let Some(atom) = atom.downcast_ref::<TileEditorUndoAtom>() {
                            self.add_undo(atom.clone(), ctx);
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                // The Size of the Tile has been edited
                if id.name == "Tile Size Edit" {
                    if let Some(size) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if !tile.is_empty() {
                                    if size != tile.textures[0].width as i32 {
                                        let new_tile = tile.resized(size as usize, size as usize);
                                        let atom = TileEditorUndoAtom::TileEdit(
                                            tile.id,
                                            tile.clone(),
                                            new_tile.clone(),
                                        );
                                        *tile = new_tile;
                                        self.add_undo(atom, ctx);
                                        self.set_tile(tile, ui, ctx, server_ctx, false);
                                    }
                                }
                            }
                        }
                    }
                } else
                // The frame count of the Tile has been edited
                if id.name == "Tile Frame Edit" {
                    if let Some(frames) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if frames != tile.textures.len() as i32 {
                                    let mut new_tile = tile.clone();
                                    new_tile.set_frames(frames as usize);
                                    let atom = TileEditorUndoAtom::TileEdit(
                                        tile.id,
                                        tile.clone(),
                                        new_tile.clone(),
                                    );
                                    *tile = new_tile;
                                    self.add_undo(atom, ctx);
                                    self.set_tile(tile, ui, ctx, server_ctx, false);
                                }
                            }
                        }
                    }
                } else
                // The palette opacity has been edited
                if id.name == "Palette Opacity Edit" {
                    if let Some(opacity) = value.to_f32() {
                        server_ctx.palette_opacity = opacity;
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Tile Frame Icons" {
                    // New frame index selected - update the editor display
                    self.set_frame_index(*index as usize, project, ui, ctx, server_ctx);
                } else if id.name == "Palette Item" {
                    project.palette.current_index = *index as u16;
                }
            }
            TheEvent::TileZoomBy(id, delta) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    self.zoom += *delta * 0.5;
                    self.zoom = self.zoom.clamp(1.0, 60.0);
                    if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
                        editor.set_zoom(self.zoom);
                        editor.relayout(ctx);
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any tile has changes (index >= 0, meaning not fully undone)
        self.tile_undos.values().any(|undo| undo.has_changes())
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = self.current_tile_id {
            if let Some(undo) = self.tile_undos.get_mut(&tile_id) {
                undo.undo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = self.current_tile_id {
            if let Some(undo) = self.tile_undos.get_mut(&tile_id) {
                undo.redo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(tile_id) = self.current_tile_id {
            if let Some(undo) = self.tile_undos.get(&tile_id) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No tile selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }

    fn editor_tools(&self) -> Option<Vec<Box<dyn EditorTool>>> {
        Some(vec![
            Box::new(TileDrawTool::new()),
            // Box::new(TileFillTool::new()),
            // Box::new(TilePickerTool::new()),
        ])
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        buffer.fill(BLACK);

        if let Some(tile_id) = self.current_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                let index = server_ctx.curr_tile_frame_index;

                let stride: usize = buffer.stride();

                let src_pixels = &tile.textures[index].data;
                let src_w = tile.textures[index].width as f32;
                let src_h = tile.textures[index].height as f32;

                let dim = buffer.dim();
                let dst_w = dim.width as f32;
                let dst_h = dim.height as f32;

                // Compute scale
                let scale = (dst_w / src_w).min(dst_h / src_h);

                // Scaled dimensions
                let draw_w = src_w * scale;
                let draw_h = src_h * scale;

                // Center
                let offset_x = ((dst_w - draw_w) * 0.5).round() as usize;
                let offset_y = ((dst_h - draw_h) * 0.5).round() as usize;

                let dst_rect = (
                    offset_x,
                    offset_y,
                    draw_w.round() as usize,
                    draw_h.round() as usize,
                );

                ctx.draw.blend_scale_chunk(
                    buffer.pixels_mut(),
                    &dst_rect,
                    stride,
                    src_pixels,
                    &(src_w as usize, src_h as usize),
                );

                return true;
            }
        }
        false
    }
}

impl TilesEditorDock {
    /// Switch to a different tile and update undo button states
    pub fn switch_to_tile(
        &mut self,
        tile: &rusterix::Tile,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        self.current_tile_id = Some(tile.id);

        // Verify frame index is valid for the new tile
        if server_ctx.curr_tile_frame_index >= tile.textures.len() {
            server_ctx.curr_tile_frame_index = 0;
        }

        self.set_undo_state_to_ui(ctx);
    }

    /// Set the current frame/texture index
    pub fn set_frame_index(
        &mut self,
        index: usize,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Verify the index is valid for current tile
        if let Some(tile_id) = self.current_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                if index < tile.textures.len() {
                    server_ctx.curr_tile_frame_index = index;

                    // Update the TreeIcons selection
                    if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                        if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                            if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                widget.set_selected_index(Some(index));
                            }
                        }
                    }

                    // Refresh the display with the new frame
                    self.update_editor_display(tile, ui, ctx, server_ctx);
                }
            }
        }
    }

    /// Update just the editor display (for when frame index changes)
    fn update_editor_display(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();
                    let icon_width = tile.textures[frame_index].width;
                    let icon_height = tile.textures[frame_index].height;

                    self.zoom = (view_width as f32 / icon_width as f32)
                        .min(view_height as f32 / icon_height as f32);

                    rgba_view.set_buffer(buffer);
                    editor.set_zoom(self.zoom);
                    editor.relayout(ctx);
                }
            }
        }
    }

    /// Update the frame icons in the tree (call after editing a texture)
    pub fn update_frame_icons(&self, tile: &rusterix::Tile, ui: &mut TheUI) {
        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                    // Update all frame icons
                    for (index, texture) in tile.textures.iter().enumerate() {
                        widget.set_icon(index, texture.to_rgba());
                    }
                }
            }
        }
    }

    /// Add an undo atom to the current tile's undo stack
    pub fn add_undo(&mut self, atom: TileEditorUndoAtom, ctx: &mut TheContext) {
        if let Some(tile_id) = self.current_tile_id {
            let undo = self
                .tile_undos
                .entry(tile_id)
                .or_insert_with(TileEditorUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    /// Set the tile for the editor.
    pub fn set_tile(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        update_only: bool,
    ) {
        // Switch to this tile's undo stack
        if !update_only {
            self.switch_to_tile(tile, ctx, server_ctx);

            if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                    // Set the tile size
                    if let Some(widget) = tile_node.widgets[0].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures[0].width as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[1].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures.len() as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                        widget.set_icon_count(tile.textures.len());
                        for (index, texture) in tile.textures.iter().enumerate() {
                            widget.set_icon(index, texture.to_rgba());
                        }
                    }
                }
            }
        }

        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                // Use current frame index, ensure it's valid
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();

                    if !update_only {
                        rgba_view.set_grid(Some(1));

                        let icon_width = tile.textures[frame_index].width;
                        let icon_height = tile.textures[frame_index].height;

                        self.zoom = (view_width as f32 / icon_width as f32)
                            .min(view_height as f32 / icon_height as f32);
                    }
                    rgba_view.set_buffer(buffer);
                }
            }
            if !update_only {
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }
}
