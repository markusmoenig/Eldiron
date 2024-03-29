use crate::widget::*;
use itertools::Itertools;

const SCROLLBAR_WIDTH: usize = 18;
pub struct TileSelectorWidget {
    pub rect: (usize, usize, usize, usize),
    screen_offset: (usize, usize),

    tiles: Option<Vec<TileData>>,

    pub grid_size: usize,
    pub selected: Option<TileData>,

    mouse_wheel_delta: isize,

    line_offset: isize,
    max_line_offset: usize,
    total_lines: usize,
    visible_lines: usize,

    scroll_thumb_rect: Option<(usize, usize, usize, usize)>,
    scroll_click: Option<(usize, usize)>,
    scroll_offset_start: isize,
}

impl TileSelectorWidget {
    pub fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        _asset: &Asset,
        _context: &ScreenContext,
    ) -> Self {
        Self {
            rect,
            screen_offset: (0, 0),

            tiles: None,
            grid_size: 32,

            selected: None,

            mouse_wheel_delta: 0,

            line_offset: 0,
            max_line_offset: 0,
            total_lines: 0,
            visible_lines: 0,

            scroll_thumb_rect: None,
            scroll_click: None,
            scroll_offset_start: 0,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.rect.2 = width;
        self.rect.3 = height;
        self.line_offset = 0;
    }

    pub fn draw(
        &mut self,
        frame: &mut [u8],
        stride: usize,
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) {
        context
            .draw2d
            .draw_rect(frame, &self.rect, stride, &context.color_black);

        let width = self.rect.2 - SCROLLBAR_WIDTH;

        let grid_size = self.grid_size;
        let left_offset = (width % grid_size) / 2;
        let top_offset = (self.rect.3 % grid_size) / 2;

        self.screen_offset = (left_offset, top_offset);

        let grid = (width / self.grid_size, self.rect.3 / self.grid_size);
        let max_tiles = grid.0 * grid.1;

        self.max_line_offset = 0;
        self.visible_lines = self.rect.3 / grid_size;
        self.total_lines = 0;

        if let Some(tiles) = &self.tiles {
            if tiles.len() > max_tiles {
                self.max_line_offset = (tiles.len() - max_tiles) / grid_size;
                if (tiles.len() - max_tiles) % grid_size != 0 {
                    self.max_line_offset += 1;
                }
                self.total_lines = tiles.len() / grid_size;
            }

            let mut x = self.rect.0 + left_offset;
            let mut y = self.rect.1 + top_offset;

            let offset = self.line_offset as usize * grid.0;

            for index in 0..max_tiles {
                if index + offset >= tiles.len() {
                    break;
                }

                let tile = &tiles[index + offset];

                if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                    context.draw2d.draw_animated_tile(
                        frame,
                        &(x, y),
                        map,
                        stride,
                        &(tile.x_off as usize, tile.y_off as usize),
                        anim_counter,
                        self.grid_size,
                    );

                    //let mut selected_drawn = false;
                    if let Some(selected) = &self.selected {
                        if selected.tilemap == map.settings.id
                            && selected.x_off == tile.x_off
                            && selected.y_off == tile.y_off
                        {
                            context.draw2d.draw_rect_outline(
                                frame,
                                &(x, y, grid_size, grid_size),
                                stride,
                                context.color_white,
                            );
                            //selected_drawn = true;
                        }
                    }

                    /*
                    if selected_drawn == false {
                        if tile.3 == TileUsage::EnvBlocking {
                            context.draw2d.draw_rect_outline(frame, &(x, y, grid_size, grid_size), stride, context.color_red);
                        } else
                        if tile.3 == TileUsage::Water {
                            context.draw2d.draw_rect_outline(frame, &(x, y, grid_size, grid_size), stride, context.color_blue);
                        }
                    }*/
                }

                x += self.grid_size;
                if x + self.grid_size > self.rect.0 + width {
                    x = self.rect.0 + left_offset;
                    y += self.grid_size;
                }
            }

            // Draw scrollbar
            let height = self.visible_lines * self.grid_size;

            let total_height = self.total_lines * self.grid_size;

            let mut sbr = (
                self.rect.0 + self.rect.2 - SCROLLBAR_WIDTH,
                self.rect.1,
                SCROLLBAR_WIDTH,
                self.rect.3,
            );
            context
                .draw2d
                .draw_rect(frame, &sbr, stride, &context.color_black);

            let ratio = height as f32 / total_height as f32;
            let theight = (height as f32 * ratio) as usize;

            let offset = (self.line_offset as f32 * self.grid_size as f32 * ratio) as usize;

            sbr.1 += offset;
            sbr.3 = theight;

            if offset + theight > height {
                sbr.3 -= (offset + theight) - height;
            }

            if self.scroll_click.is_some() {
                context
                    .draw2d
                    .draw_rect(frame, &sbr, stride, &[60, 60, 60, 255]);
            } else {
                context
                    .draw2d
                    .draw_rect(frame, &sbr, stride, &context.color_node_dark_gray);
            }
            self.scroll_thumb_rect = Some(sbr);
        }
    }

    pub fn mouse_down(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        self.scroll_click = None;
        if pos.0 > self.rect.0 + self.rect.2 - SCROLLBAR_WIDTH {
            if let Some(scroll_rect) = &self.scroll_thumb_rect {
                if context.contains_pos_for(pos, *scroll_rect) {
                    self.scroll_click = Some(pos);
                    self.scroll_offset_start = self.line_offset;
                }
            }
            return true;
        }

        if context.contains_pos_for(pos, self.rect) {
            let grid_size = self.grid_size;

            let width = self.rect.2 - SCROLLBAR_WIDTH;
            let screen_x = width / grid_size;

            if pos.0 >= self.rect.0 + self.screen_offset.0
                && pos.1 > self.rect.1 + self.screen_offset.1
            {
                let x = (pos.0 - self.rect.0 - self.screen_offset.0) / grid_size;
                let y = (pos.1 - self.rect.1 - self.screen_offset.1) / grid_size;
                let tile_offset = x + y * screen_x + self.line_offset as usize * screen_x;

                if let Some(tiles) = &self.tiles {
                    if tile_offset < tiles.len() {
                        let tile_ref = tiles[tile_offset].clone();
                        self.selected = Some(tile_ref);
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn _mouse_up(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        self.scroll_click = None;

        if context.contains_pos_for(pos, self.rect) {
            return true;
        }
        false
    }

    pub fn mouse_dragged(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
        _options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        if let Some(scroll_click) = &self.scroll_click {
            let mut y: isize = pos.1 as isize - scroll_click.1 as isize;
            y = (y as f32 * self.total_lines as f32 / self.visible_lines as f32) as isize;

            self.line_offset = self.scroll_offset_start + y / self.grid_size as isize;
            self.line_offset = self.line_offset.clamp(0, self.max_line_offset as isize);

            return true;
        }

        false
    }

    pub fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
    ) -> bool {
        self.mouse_wheel_delta += delta.1;
        self.line_offset -= self.mouse_wheel_delta / self.grid_size as isize;
        self.line_offset = self.line_offset.clamp(0, self.max_line_offset as isize);
        self.mouse_wheel_delta -=
            (self.mouse_wheel_delta / self.grid_size as isize) * self.grid_size as isize;
        true
    }

    /// Collects the tiles of the given type
    pub fn set_tile_type(
        &mut self,
        tile_usage: Vec<TileUsage>,
        tilemap_index: Option<usize>,
        tags: Option<String>,
        asset: &Asset,
    ) {
        let mut tiles: Vec<TileData> = vec![];
        let sorted_keys = asset.tileset.maps.keys().sorted();

        let mut tilemap_id: Option<Uuid> = None;

        if let Some(tilemap_index) = tilemap_index {
            tilemap_id = Some(asset.tileset.maps_ids[tilemap_index]);
        }

        for key in sorted_keys {
            let map = &asset.tileset.maps[key];
            let amount = map.max_tiles();
            for offset in 0..amount {
                let id = map.offset_to_id(offset);
                if let Some(tile) = map.get_tile(&id) {
                    if tile_usage.contains(&tile.usage)
                        || (tile_usage.is_empty() && tile.usage != TileUsage::Unused)
                    {
                        if tilemap_id == None || tilemap_id.unwrap() == map.settings.id {
                            if tags.is_some() {
                                if tile.tags.contains(&tags.clone().unwrap()) {
                                    tiles.push(TileData {
                                        tilemap: map.settings.id,
                                        x_off: id.0 as u16,
                                        y_off: id.1 as u16,
                                        usage: tile.usage.clone(),
                                        size: None,
                                    });
                                }
                            } else {
                                tiles.push(TileData {
                                    tilemap: map.settings.id,
                                    x_off: id.0 as u16,
                                    y_off: id.1 as u16,
                                    usage: tile.usage.clone(),
                                    size: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        self.tiles = Some(tiles);
        self.line_offset = 0;
    }
}
