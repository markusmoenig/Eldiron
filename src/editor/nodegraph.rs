use crate::widget::*;

use crate::Asset;
use crate::editor::ScreenContext;

pub struct NodeGraph {
    rect            : (usize, usize, usize, usize),
    dirty           : bool,
    buffer          : Vec<u8>,
}

impl Widget for NodeGraph {
    
    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self where Self: Sized {
        Self {
            rect,
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4]
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.buffer.resize(width * height * 4, 0);
        self.dirty = true;
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {           
            context.draw2d.draw_square_pattern(buffer_frame, &rect, rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);     
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
    }

    fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        // if self.contains_pos(pos) {
        //     //self.state.set(2);
        //     self.clicked.set(true);
        //     return true;
        // }
        false
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        // if self.state.get() == 2 {
        //     //self.state.set(1);
        //     return true;
        // }
        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}