use crate::prelude::*;

pub struct TilePickerTool {
    id: TheId,
}

impl EditorTool for TilePickerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tile Picker Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Color Picker Tool (I). Click to pick a color from the tile.".to_string()
    }

    fn icon_name(&self) -> String {
        "eyedropper".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('I')
    }

    fn handle_event(
        &mut self,
        event: EditorToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            EditorToolEvent::Click(pos) | EditorToolEvent::Drag(pos) => {
                self.pick_color(pos, ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}

impl TilePickerTool {
    fn pick_color(
        &mut self,
        pos: Vec2<i32>,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                let frame_index = server_ctx.curr_tile_frame_index;
                if frame_index < tile.textures.len() {
                    let width = tile.textures[frame_index].width as i32;
                    let height = tile.textures[frame_index].height as i32;

                    if pos.x >= 0 && pos.x < width && pos.y >= 0 && pos.y < height {
                        let index = (pos.y * width + pos.x) as usize;
                        if index < tile.textures[frame_index].data.len() {
                            let color = tile.textures[frame_index].data[index];
                            server_ctx.curr_palette_color = color;

                            // Update UI to reflect the picked color
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Palette Color Picked"),
                                TheValue::Int(color as i32),
                            ));
                        }
                    }
                }
            }
        }
    }
}
