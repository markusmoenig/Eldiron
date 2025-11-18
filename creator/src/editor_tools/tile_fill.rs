use crate::prelude::*;
use std::collections::VecDeque;

pub struct TileFillTool {
    id: TheId,
}

impl EditorTool for TileFillTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tile Fill Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Fill Tool (F). Click to flood fill an area with the selected palette color.".to_string()
    }

    fn icon_name(&self) -> String {
        "fill".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('F')
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
            EditorToolEvent::Click(pos) => {
                self.flood_fill(pos, ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}

impl TileFillTool {
    fn flood_fill(
        &mut self,
        start_pos: Vec2<i32>,
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

                    if start_pos.x < 0
                        || start_pos.x >= width
                        || start_pos.y < 0
                        || start_pos.y >= height
                    {
                        return;
                    }

                    let target_color = tile.textures[frame_index].data
                        [(start_pos.y * width + start_pos.x) as usize];
                    let replacement_color = server_ctx.curr_palette_color;

                    // Don't fill if the colors are the same
                    if target_color == replacement_color {
                        return;
                    }

                    // Flood fill using BFS
                    let mut queue = VecDeque::new();
                    let mut visited = vec![false; (width * height) as usize];

                    queue.push_back(start_pos);
                    visited[(start_pos.y * width + start_pos.x) as usize] = true;

                    while let Some(pos) = queue.pop_front() {
                        let index = (pos.y * width + pos.x) as usize;
                        tile.textures[frame_index].data[index] = replacement_color;

                        // Check neighbors (4-way connectivity)
                        let neighbors = [
                            Vec2::new(pos.x - 1, pos.y),
                            Vec2::new(pos.x + 1, pos.y),
                            Vec2::new(pos.x, pos.y - 1),
                            Vec2::new(pos.x, pos.y + 1),
                        ];

                        for neighbor in neighbors {
                            if neighbor.x >= 0
                                && neighbor.x < width
                                && neighbor.y >= 0
                                && neighbor.y < height
                            {
                                let neighbor_index = (neighbor.y * width + neighbor.x) as usize;
                                if !visited[neighbor_index]
                                    && tile.textures[frame_index].data[neighbor_index]
                                        == target_color
                                {
                                    visited[neighbor_index] = true;
                                    queue.push_back(neighbor);
                                }
                            }
                        }
                    }

                    // Update the tile editor view
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tile Editor"),
                        TheValue::Empty,
                    ));
                }
            }
        }
    }
}
