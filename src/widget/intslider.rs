use crate::widget::*;

use core::cell::Cell;

pub struct IntSliderWidget {
    rect            : (u32, u32, u32, u32),
    text            : Vec<String>,
    state           : Cell<u32>,
    pub value       : Cell<i32>,
}

impl Widget for IntSliderWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(1),
            value               : Cell::new(0)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {

        asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        let text = format!("{}: {}", self.text[0], self.value.get());

        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect(frame, &self.rect, &text.to_string(), self.get_color_text_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else 
        if state == 1 {
            asset.draw_text_rect(frame, &self.rect, &text.to_string(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else
        if state == 2 {
            asset.draw_text_rect(frame, &self.rect, &text.to_string(), self.get_color_text(), self.get_color_selection_blue(), crate::asset::TextAlignment::Center);
        }        
    }

    fn mouse_down(&self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            self.state.set(2);
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

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}