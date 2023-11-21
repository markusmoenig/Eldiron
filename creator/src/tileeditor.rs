use crate::prelude::*;

pub struct TileEditor {
    tiledrawer: TileDrawer,
    curr_tile_uuid: Uuid,
}

#[allow(clippy::new_without_default)]
impl TileEditor {
    pub fn new() -> Self {
        Self {
            tiledrawer: TileDrawer::new(),
            curr_tile_uuid: Uuid::new_v4(),
        }
    }

    pub fn init_ui(&mut self, ui: &mut TheUI, _ctx: &mut TheContext, _project: &mut Project) {
        let mut center = TheCanvas::new();
        let mut region_editor = TheRGBALayout::new(TheId::named("Region Editor"));
        if let Some(rgba_view) = region_editor.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            rgba_view.set_selection_color([255, 255, 255, 10]);
        }
        center.set_layout(region_editor);
        ui.canvas.set_center(center);
    }

    #[allow(clippy::suspicious_else_formatting)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::TileEditorClicked(id, coord) => {
                if let Some(coord) = coord.to_vec2i() {
                    if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None)
                    {
                        if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                            if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                self.tiledrawer.draw_single_tile(coord, rgba_view.buffer_mut(), 24, self.curr_tile_uuid, ctx)
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, _state) => {
                if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            if let Some(rgba_layout) =
                                ui.canvas.get_layout(Some(&"Region Editor".into()), None)
                            {
                                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                    if let Some(rgba_view) =
                                        rgba_layout.rgba_view_mut().as_rgba_view()
                                    {
                                        rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                                        let width = r.width * r.grid_size;
                                        let height = r.height * r.grid_size;
                                        let mut buffer =
                                            TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                        rgba_view.set_buffer(buffer);
                                        rgba_view.set_grid(Some(r.grid_size));
                                        ctx.ui.relayout = true;
                                    }
                                }
                            }
                            redraw = true;
                        }
                    }
                } else if id.name == "Tilemap Tile" {
                    self.curr_tile_uuid = id.uuid;
                } else if id.name == "Tilemap Editor Add Selection" {
                    self.tiledrawer.tiles = project.extract_tiles();
                }
            }
            _ => {}
        }
        redraw
    }
}
