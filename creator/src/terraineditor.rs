use shared::prelude::*;

use crate::prelude::*;

pub struct TerrainEditor {
    pub buffer: TheRGBABuffer,
}

#[allow(clippy::new_without_default)]
impl TerrainEditor {
    pub fn new() -> Self {
        Self {
            buffer: TheRGBABuffer::default(),
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
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
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
    ) {
        let palette = project.palette.clone();
        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
                if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                    rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                    rgba_view.set_grid(Some(1));

                    let region_width = region.width * region.grid_size;
                    let region_height = region.height * region.grid_size;

                    let mut buffer = TheRGBABuffer::new(TheDim::sized(region_width, region_height));
                    crate::minimap::draw_minimap(region, &mut buffer, &palette);

                    self.buffer = buffer.clone();

                    rgba_view.set_buffer(buffer);
                    ctx.ui.relayout = true;
                    ctx.ui.redraw_all = true;
                }
                editor.scroll_to(region.scroll_offset);
            }
        }
    }
}
