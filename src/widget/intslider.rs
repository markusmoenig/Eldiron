use crate::widget::*;

use core::cell::Cell;

pub struct IntSliderWidget {
    rect            : (u32, u32, u32, u32),
    text            : Vec<String>,
    state           : Cell<u32>,
    pub value       : i32,
    pub range       : (i32, i32)
}

impl Widget for IntSliderWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(1),
            value               : 0,
            range               : (1, 4)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset, _context: &ScreenContext) {

        asset.draw_rect(frame, &self.rect, self.get_color_selection());

        let width = (self.rect.2 as i32 / self.range.1) * self.value;

        asset.draw_rect(frame, &(self.rect.0, self.rect.1, width as u32, self.rect.3), self.get_color_selection_blue());

        let text = format!("{}: {}", self.text[0], self.value);
        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect_blend(frame, &self.rect, &text.to_string(), self.get_color_text_disabled(), crate::asset::TextAlignment::Center);
        } else
        if state == 1 {
            asset.draw_text_rect_blend(frame, &self.rect, &text.to_string(), self.get_color_text(), crate::asset::TextAlignment::Center);
        } 
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            //self.state.set(2);

            let step = self.rect.2 as i32 / self.range.1;
            let v = self.range.0 + (pos.0 as i32 - self.rect.0 as i32 ) / step;

            //println!("value {}", v);

            self.value = v;

            return true;
        }
        false
    }

    fn mouse_dragged(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            //self.state.set(2);

            let step = self.rect.2 as i32 / self.range.1;
            let v = self.range.0 + (pos.0 as i32 - self.rect.0 as i32 ) / step;

            self.value = v;

            return true;
        }
        false
    }

    fn mouse_up(&mut self, _pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.state.get() == 2 {
            self.state.set(1);
            return true;
        }
        false
    }

    fn set_state(&self, state: u32) {
        if self.state.get() == 2 && state == 1 {
            return;
        }
        self.state.set(state);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}