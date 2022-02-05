use crate::widget::*;

use core::cell::Cell;

pub struct IntSliderWidget {
    rect            : (u32, u32, u32, u32),
    text            : Vec<String>,
    state           : Cell<u32>,
    pub value       : Cell<i32>,
    range           : Cell<(i32, i32)>
}

impl Widget for IntSliderWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(1),
            value               : Cell::new(0),
            range               : Cell::new((1, 4))
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {

        asset.draw_rect(frame, &self.rect, self.get_color_selection());

        let width = (self.rect.2 as i32 / self.range.get().1) * self.value.get();

        asset.draw_rect(frame, &(self.rect.0, self.rect.1, width as u32, self.rect.3), self.get_color_selection_blue());

        let text = format!("{}: {}", self.text[0], self.value.get());
        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect_blend(frame, &self.rect, &text.to_string(), self.get_color_text_disabled(), crate::asset::TextAlignment::Center);
        } else
        if state == 1 {
            asset.draw_text_rect_blend(frame, &self.rect, &text.to_string(), self.get_color_text(), crate::asset::TextAlignment::Center);
        } 
    }

    fn mouse_down(&self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            //self.state.set(2);

            let step = self.rect.2 as i32 / self.range.get().1;
            let v = self.range.get().0 + (pos.0 as i32 - self.rect.0 as i32 ) / step;

            //println!("value {}", v);

            self.value.set(v);

            return true;
        }
        false
    }

    fn mouse_dragged(&self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            //self.state.set(2);

            let step = self.rect.2 as i32 / self.range.get().1;
            let v = self.range.get().0 + (pos.0 as i32 - self.rect.0 as i32 ) / step;

            println!("value {}", v);

            self.value.set(v);

            return true;
        }
        false
    }

    fn mouse_up(&self, _pos: (u32, u32), _asset: &mut Asset) -> bool {
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

    fn set_range(&self, range: (f32, f32)) {
        self.range.set((range.0 as i32, range.1 as i32));
    }

    fn set_value(&self, value: f32) {
        self.value.set(value as i32);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}