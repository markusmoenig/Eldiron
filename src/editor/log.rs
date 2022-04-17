use crate::Asset;
use crate::editor::ScreenContext;

pub struct LogWidget {
    pub rect                    : (isize, isize, usize, usize),

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub size                    : (usize, usize),

    pub drag_size               : Option<(usize, usize)>,

    pub drawn_lines             : usize,
}

impl LogWidget {

    pub fn new(_context: &ScreenContext) -> Self {

        Self {
            rect                : (950, 450, 280, 200),

            dirty               : true,
            buffer              : vec![],

            size                : (280, 200),

            drag_size           : None,

            drawn_lines         : 0,
        }
    }

    pub fn draw(&mut self, _frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        if self.buffer.len() != self.size.0 * self.size.1 * 4 {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        if context.data.messages.len() != self.drawn_lines {
            self.dirty = true;
        }

        let rect = (0, 0, self.size.0, self.size.1);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            let b = context.color_black;

            context.draw2d.draw_rounded_rect(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3 - 2) as f64), &[b[0], b[1], b[2], 150], &(20.0, 20.0, 20.0, 20.0));

            let text_size = 20_usize;
            let max_lines = (self.size.1 - 10 ) / (text_size as usize);

            let available_messages = context.data.messages.len();

            let w = context.color_white;

            for l in 0..max_lines {

                if l >= context.data.messages.len() {
                    break;
                }

                context.draw2d.draw_text_rect(buffer_frame, &(15, self.size.1 - 10 - (l+1) * (text_size as usize), self.size.0 - 30, text_size), rect.2, &asset.open_sans, text_size as f32, context.data.messages[available_messages - 1 - l].0.as_str(), &[w[0], w[1], w[2], 150], &[b[0], b[1], b[2], 150], crate::draw2d::TextAlignment::Left);
            }

            self.drawn_lines = context.data.messages.len();
        }
        self.dirty = false;
    }

    pub fn _mouse_wheel(&mut self, _delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        // self.area_scroll_offset.0 -= delta.0 / self.tile_size as isize;
        // self.area_scroll_offset.1 += delta.1 / self.tile_size as isize;
        // self.dirty = true;
        true
    }
}