use crate::prelude::*;

pub struct TileMapWidget {
    rect                    : (usize, usize, usize, usize),
    tilemap_id              : Uuid,
    scale                   : f32,

    screen_offset           : (usize, usize),

    line_offset             : isize,
    max_line_offset         : usize,

    image_offset            : (isize, isize),

    mouse_wheel_delta       : (isize, isize),

    is_image                : bool,
}

impl EditorContent for TileMapWidget {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _behavior_type: BehaviorType, _asset: &Asset, _context: &ScreenContext) -> Self {

        Self {
            rect,
            tilemap_id              : Uuid::new_v4(),
            scale                   : 2.0,

            screen_offset           : (0, 0),

            line_offset             : 0,
            max_line_offset         : 0,

            image_offset            : (0, 0),

            mouse_wheel_delta       : (0, 0),

            is_image                : false,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &mut ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>) {

        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        if self.is_image {
            if asset.tileset.images.is_empty() { return }

            let image = &asset.tileset.images[&self.tilemap_id];
            let source = &image.pixels[..];

            //pub fn blend_slice_safe(&self, dest: &mut [u8], source: &[u8], rect: &(isize, isize, usize, usize), dest_stride: usize, safe_rect: &(usize, usize, usize, usize)) {

            //context.draw2d.blend_slice_safe(frame, &image.pixels[..], &(self.rect.0 as isize + self.image_offset.0, self.rect.1 as isize - self.image_offset.1, image.width, image.height), context.width, &(self.rect.0, self.rect.1, self.rect.2, self.rect.3));

            let rect = (self.rect.0 as isize + self.image_offset.0, self.rect.1 as isize - self.image_offset.1, image.width, image.height);
            let safe_rect = (self.rect.0, self.rect.1, self.rect.2, self.rect.3);

            let dest_stride_isize = context.width as isize;
            for y in 0..rect.3 as isize {
                let d = rect.0 * 4 + (y + rect.1) * dest_stride_isize * 4;
                let s = y * (rect.2 as isize) * 4;

                // TODO: Make this faster

                if (y + rect.1 as isize) >= safe_rect.1 as isize && (y + rect.1 as isize) < (safe_rect.1 + safe_rect.3) as isize {
                    for x in 0..rect.2 as isize {

                        if (x + rect.0 as isize) >= safe_rect.0 as isize && (x + rect.0 as isize) < (safe_rect.0 + safe_rect.2) as isize {
                            let dd = (d + x * 4) as usize;
                            let ss = (s + x * 4) as usize;

                            //let background = &[frame[dd], frame[dd+1], frame[dd+2], frame[dd+3]];
                            let color = &[source[ss], source[ss+1], source[ss+2], source[ss+3]];
                            //frame[dd..dd + 4].copy_from_slice(&self.mix_color(&background, &color, (color[3] as f64) / 255.0));
                            frame[dd..dd + 4].copy_from_slice(color);
                        }
                    }
                }
            }

        } else {
            if asset.tileset.maps.is_empty() { return }

            let scale = self.scale;
            if let Some(map) = &asset.tileset.maps.get_mut(&self.tilemap_id) {
                let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

                let x_tiles = map.width / map.settings.grid_size;
                let y_tiles = map.height / map.settings.grid_size;

                let total_tiles = (x_tiles * y_tiles) as usize;

                let screen_x = self.rect.2 / scaled_grid_size;
                let screen_y = self.rect.3 / scaled_grid_size;

                let left_offset = (self.rect.2 % scaled_grid_size) / 2;
                let top_offset = (self.rect.3 % scaled_grid_size) / 2;

                self.screen_offset = (left_offset, top_offset);

                let tiles_per_page = screen_x * screen_y;

                self.max_line_offset = 0;

                if total_tiles > tiles_per_page {
                    self.max_line_offset = (total_tiles - tiles_per_page) / screen_x;
                    if (total_tiles - tiles_per_page) % screen_x != 0 {
                        self.max_line_offset += 1;
                    }
                }

                let mut x_off = 0_usize;
                let mut y_off = 0_usize;

                let offset = self.line_offset as usize * screen_x;

                // Draw the tiles
                for tile in 0..tiles_per_page {

                    if tile + offset >= total_tiles {
                        break;
                    }

                    let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as usize;
                    let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as usize;

                    let x = (tile+offset) % x_tiles as usize;
                    let y = (tile+offset) / x_tiles as usize;

                    let pp = &(x_step + self.rect.0 + left_offset, y_step + self.rect.1 + top_offset);

                    if let Some(tile) = map.get_tile(&(x, y)) {

                        if tile.anim_tiles.len() > 0 {
                            let index = anim_counter % tile.anim_tiles.len() as usize;
                            context.draw2d.draw_tile(frame, pp, map, context.width, &tile.anim_tiles[index], scale);
                        } else
                        if tile.usage == TileUsage::Unused {
                            context.draw2d.draw_tile_mixed(frame, pp, map, context.width, &(x, y), [128, 128, 128, 255], scale);
                        } else {
                            context.draw2d.draw_tile(frame, pp, map, context.width, &(x, y), scale);
                        }
                    } else {
                        context.draw2d.draw_tile(frame, pp, map, context.width, &(x, y), scale);
                    }

                    if let Some(selection) = context.curr_tile {
                        if x == selection.0 && y == selection.1 {
                            context.draw2d.draw_rect_outline(frame, &(pp.0, pp.1, scaled_grid_size, scaled_grid_size), context.width, context.color_white);
                        } else {
                            if let Some(selection_end) = context.selection_end {
                                if  y > selection.1 || y == selection.1 && x >= selection.0 { // >=
                                    if  y < selection_end.1 || y == selection_end.1 && x <= selection_end.0 { // <=
                                        context.draw2d.draw_rect_outline(frame, &(pp.0, pp.1, scaled_grid_size, scaled_grid_size), context.width, context.color_white);
                                    }
                                }
                            }
                        }
                    }

                    x_off += 1;

                    if x_off >= screen_x {
                        x_off = 0;
                        y_off += 1;
                        if y_off >= screen_y {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if let Some(tile) = self.screen_to_map(asset, pos) {
            context.curr_tile = Some(tile);
            context.selection_end = None;

            if let Some(options) = options {
                options.adjust_tile_usage(asset, context);

                if context.curr_tile.is_some() {
                    options.set_state(WidgetState::Normal);
                } else {
                    options.set_state(WidgetState::Disabled);
                }
            }
            true
        } else {
            false
        }
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        let consumed = false;
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        if let Some(curr_id) = context.curr_tile {
            if let Some(end_pos) = self.screen_to_map(asset, pos) {
                if end_pos.0 > curr_id.0 || end_pos.1 > curr_id.1 {
                    context.selection_end = Some(end_pos);
                    return true;
                }
            }
        }
        false
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        if self.is_image {
            let grid_size = 32_isize;
            self.mouse_wheel_delta.0 += delta.0;
            self.mouse_wheel_delta.1 += delta.1;

            self.image_offset.0 -= self.mouse_wheel_delta.0 / grid_size as isize;
            self.image_offset.1 -= self.mouse_wheel_delta.1 / grid_size as isize;
            //self.image_offset.1 = self.image_offset.1.clamp(0, self.max_line_offset as isize);

            self.mouse_wheel_delta.0 -= (self.mouse_wheel_delta.0 / grid_size) * grid_size;
            self.mouse_wheel_delta.1 -= (self.mouse_wheel_delta.1 / grid_size) * grid_size;
        } else {
            let grid_size = 32_isize;
            self.mouse_wheel_delta.1 += delta.1;
            self.line_offset -= self.mouse_wheel_delta.1 / grid_size as isize;
            self.line_offset = self.line_offset.clamp(0, self.max_line_offset as isize);
            self.mouse_wheel_delta.1 -= (self.mouse_wheel_delta.1 / grid_size) * grid_size;
        }
        true
    }

    /// Sets a new map index
    fn set_tilemap_id(&mut self, id: Uuid, asset: &mut Asset) {
        self.tilemap_id = id;
        self.line_offset = 0;
        self.is_image = asset.tileset.images_ids.contains(&id);
    }

    // Returns true if we show an image
    fn is_image(&mut self) -> bool {
        self.is_image
    }

    /// Converts a screen position to a map grid position
    fn screen_to_map(&self, asset: &Asset, screen_pos: (usize, usize)) -> Option<(usize, usize)> {
        let scale = self.scale;

        if let Some(map) = asset.get_map_of_id(self.tilemap_id) {
            let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

            let x_tiles = map.width / map.settings.grid_size;
            let y_tiles = map.height / map.settings.grid_size;

            let total_tiles = (x_tiles * y_tiles) as usize;

            let screen_x = self.rect.2 / scaled_grid_size;

            if screen_pos.0 > self.rect.0 + self.screen_offset.0 && screen_pos.1 > self.rect.1 + self.screen_offset.0 {

                let x = (screen_pos.0 - self.rect.0 - self.screen_offset.0) / scaled_grid_size;
                let y = (screen_pos.1 - self.rect.1 - self.screen_offset.1) / scaled_grid_size + self.line_offset as usize;

                let tile_offset = x + y * screen_x;

                if tile_offset < total_tiles {
                    return Some(((tile_offset % x_tiles), (tile_offset / x_tiles)));
                }
            }
        }
        None
    }
}