use shared::prelude::*;

use crate::editor::TILEDRAWER;
use crate::prelude::*;

pub struct TerrainEditor {
    pub buffer: TheRGBABuffer,
    pub grid_size: i32,
}

#[allow(clippy::new_without_default)]
impl TerrainEditor {
    pub fn new() -> Self {
        Self {
            buffer: TheRGBABuffer::default(),
            grid_size: 0,
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        let terrain_editor = TheRGBALayout::new(TheId::named("TerrainMap"));
        //if let Some(rgba_view) = terrain_editor.rgba_view_mut().as_rgba_view() {}
        center.set_layout(terrain_editor);

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Terrain Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        center
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::match_single_binding)]
        match event {
            // TheEvent::TileEditorHoverChanged(id, coord) => {
            //     if id.name == "TerrainMap View" {
            //         if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
            //             if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
            //                 let b = rgba_view.buffer_mut();
            //                 b.copy_into(0, 0, &self.buffer);

            //                 for y in coord.y - self.se
            //             }
            //         }
            //         //println!("coord {}", coord);
            //     }
            // }
            _ => {}
        }

        redraw
    }

    pub fn activated(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &ServerContext,
        scroll_to: bool,
    ) {
        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            self.grid_size = region.grid_size;
            let region_width = region.width * region.grid_size;
            let region_height = region.height * region.grid_size;

            let mut buffer = TheRGBABuffer::new(TheDim::sized(region_width, region_height));
            crate::minimap::draw_minimap(region, &mut buffer, true);
            self.draw_selection(ui, ctx, server_ctx, None);

            if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
                if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                    rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                    rgba_view.set_grid(Some(1));

                    self.buffer = buffer.clone();

                    rgba_view.set_buffer(buffer);
                    ctx.ui.relayout = true;
                    ctx.ui.redraw_all = true;
                }
                if scroll_to {
                    editor.scroll_to(region.scroll_offset);
                }
            }
        }
    }

    pub fn draw_selection(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
        hover_pos: Option<Vec2i>,
    ) {
        let tiledrawer = TILEDRAWER.lock().unwrap();

        if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let b = rgba_view.buffer_mut();
                if self.buffer.len() == b.len() {
                    b.pixels_mut().copy_from_slice(self.buffer.pixels());
                }

                // Selection
                if let Some(tilearea) = &server_ctx.tile_selection {
                    tiledrawer.draw_tile_selection(
                        &tilearea.merged(),
                        b,
                        self.grid_size,
                        WHITE,
                        ctx,
                    );
                }

                // Hover
                if let Some(hover_pos) = hover_pos {
                    let x0 = hover_pos.x * self.grid_size;
                    let y0 = hover_pos.y * self.grid_size;

                    b.draw_rect_outline(
                        &TheDim::new(x0, y0, self.grid_size, self.grid_size),
                        &[128, 128, 128, 255],
                    );
                }
            }
        }
    }
}
