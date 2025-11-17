use crate::docks::tiles_editor_undo::*;
use crate::editor::TOOLLIST;
use crate::prelude::*;

pub struct TilesEditorDock {
    zoom: f32,
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

        let mut palette_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Palette", self.palette_node));
        palette_node.set_open(true);

        let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        item.set_icon_count(256);
        item.set_icons_per_row(14);

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
                self.set_tile(tile, ui, ctx, false);
            }
        }

        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
            if let Some(palette_node) = tree_layout.get_node_by_id_mut(&self.palette_node) {
                if let Some(widget) = palette_node.widgets[0].as_tree_icons() {
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
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;

        match event {
            TheEvent::Custom(id, value) => {
                if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Picked"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, false);
                    }
                } else if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Updated"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, true);
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
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Palette Item" {
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
}

impl TilesEditorDock {
    /// Switch to a different tile and update undo button states
    pub fn switch_to_tile(&mut self, tile_id: Uuid, ctx: &mut TheContext) {
        self.current_tile_id = Some(tile_id);
        self.set_undo_state_to_ui(ctx);
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
        update_only: bool,
    ) {
        // Switch to this tile's undo stack
        if !update_only {
            self.switch_to_tile(tile.id, ctx);
        }

        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let buffer = tile.textures[0].to_rgba();

                if !update_only {
                    rgba_view.set_grid(Some(1));

                    let icon_width = tile.textures[0].width;
                    let icon_height = tile.textures[0].height;

                    self.zoom = (view_width as f32 / icon_width as f32)
                        .min(view_height as f32 / icon_height as f32);
                }
                rgba_view.set_buffer(buffer);
            }
            if !update_only {
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }
}
