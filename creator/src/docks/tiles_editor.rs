use crate::prelude::*;

pub struct TilesEditorDock {
    zoom: f32,
    palette_node: Uuid,
}

impl Dock for TilesEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            zoom: 5.0,
            palette_node: Uuid::new_v4(),
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
                self.set_tile(tile, ui, ctx);
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
                        self.set_tile(tile, ui, ctx);
                    }
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
            TheEvent::TileEditorClicked(id, pos) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    println!("{}", pos);
                }
            }
            _ => {}
        }

        redraw
    }
}

impl TilesEditorDock {
    /// Set the tile for the editor.
    pub fn set_tile(&mut self, tile: &rusterix::Tile, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            editor.set_zoom(self.zoom);

            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.set_grid(Some(1));

                let buffer = tile.textures[0].to_rgba();
                let icon_width = tile.textures[0].width;
                let icon_height = tile.textures[0].height;

                self.zoom = (view_width as f32 / icon_width as f32)
                    .min(view_height as f32 / icon_height as f32);

                rgba_view.set_buffer(buffer);
            }
            editor.set_zoom(self.zoom);
            editor.relayout(ctx);
        }
    }
}
