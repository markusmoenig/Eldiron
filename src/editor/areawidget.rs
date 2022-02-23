use server::asset::{ Asset };
//use server::asset::tileset::TileUsage;

use crate::widget::context::ScreenContext;

pub struct AreaWidget {
    pub rect                : (usize, usize, usize, usize),
    area_index              : usize,

    grid_size               : usize,

    offset                  : (usize, usize),
    screen_offset           : (usize, usize),

    pub clicked             : bool,
}

impl AreaWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self {

        Self {
            rect,
            area_index              : 0,
            grid_size               : 32,

            offset                  : (0, 0),
            screen_offset           : (0, 0),

            clicked                 : false
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[0,0,0,255]);

        let rect = self.rect;
        let grid_size = self.grid_size;

        let left_offset = (self.rect.2 % grid_size) / 2;
        let top_offset = (self.rect.3 % grid_size) / 2;

        self.screen_offset = (left_offset, top_offset);

        //let grid = (rect.2 / grid_size, rect.3 / grid_size);
        //let max_tiles = grid.0 * grid.1;

        let area = context.data.areas.get(&self.area_index).unwrap();

        let x_tiles = (rect.2 / grid_size) as isize;
        let y_tiles = (rect.3 / grid_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                if let Some(value) = area.get_value((x, y)) {
                    let pos = (rect.0 + (x as usize) * grid_size, rect.1 + (y as usize) * grid_size);

                    let map = asset.get_map_of_id(value.0);
                    context.draw2d.draw_animated_tile(frame, &pos, map,context.width,&(value.1, value.2), anim_counter, grid_size);
                }
            }
        }
    }

    pub fn mouse_down(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        true
    }

    pub fn _mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        let consumed = false;
        consumed
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn _mouse_dragged(&mut self, _pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if let Some(_curr_id) = context.curr_tile {
        }
        false
    }

    pub fn mouse_wheel(&mut self, _delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        /*
        self.line_offset_counter += delta.1;
        self.line_offset = (self.line_offset_counter / 40).clamp(0, self.max_line_offset as isize) as usize;
        if delta.1 == 0 {
            self.line_offset_counter = 0;
        }
        */
        true
    }

    /// Sets a new map index
    pub fn set_area_index(&mut self, index: usize) {
        self.area_index = index;
    }
}