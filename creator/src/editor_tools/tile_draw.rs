use crate::docks::tiles_editor_undo::TileEditorUndoAtom;
use crate::prelude::*;

pub struct TileDrawTool {
    id: TheId,
    before_tile: Option<rusterix::Tile>,
    changed: bool,
}

impl EditorTool for TileDrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tile Draw Tool"),
            before_tile: None,
            changed: false,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Draw Tool (D). Click and drag to draw pixels with the selected palette color.".to_string()
    }

    fn icon_name(&self) -> String {
        "draw".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('D')
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        // println!("draw {:?}", event);

        match event {
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    if let Some(tile_id) = server_ctx.curr_tile_id {
                        if let Some(tile) = project.tiles.get_mut(&tile_id) {
                            self.before_tile = Some(tile.clone());
                        }
                    }

                    self.draw_pixel(*coord, ui, ctx, project, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    self.draw_pixel(*coord, ui, ctx, project, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorUp(_) => {
                if self.changed {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tilepicker"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Editor Undo Available"),
                        TheValue::Empty,
                    ));
                    self.changed = false;
                }
            }
            _ => {}
        }

        redraw
    }

    fn get_undo_atom(&mut self, project: &Project) -> Option<Box<dyn std::any::Any>> {
        if let Some(before) = self.before_tile.take() {
            // Get the current (after) state from the project
            if let Some(tile) = project.tiles.get(&before.id) {
                if !tile.textures.is_empty() {
                    let after = tile.clone();
                    let atom = TileEditorUndoAtom::TileEdit(before.id, before, after);
                    return Some(Box::new(atom));
                }
            }
        }
        None
    }
}

impl TileDrawTool {
    fn draw_pixel(
        &mut self,
        pos: Vec2<i32>,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                let frame_index = server_ctx.curr_tile_frame_index;
                if frame_index < tile.textures.len() {
                    let width = tile.textures[frame_index].width as i32;
                    let height = tile.textures[frame_index].height as i32;

                    if pos.x >= 0 && pos.x < width && pos.y >= 0 && pos.y < height {
                        // Get the selected palette color
                        if let Some(color) = project.palette.get_current_color() {
                            // Set the pixel
                            let index = (pos.y * width + pos.x) as usize;
                            if index < tile.textures[frame_index].data.len() {
                                // Apply palette opacity to the color
                                let mut color_array = color.to_u8_array();
                                color_array[3] =
                                    (color_array[3] as f32 * server_ctx.palette_opacity) as u8;

                                tile.textures[frame_index].set_pixel(
                                    pos.x as u32,
                                    pos.y as u32,
                                    color_array,
                                );

                                tile.textures[frame_index].generate_normals(true);

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Tile Updated"),
                                    TheValue::Id(tile_id),
                                ));

                                self.changed = true;
                            }
                        }
                    }
                }
            }
        }
    }
}
