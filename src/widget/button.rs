use crate::widget::*;

use core::cell::Cell;

pub struct ButtonWidget {
    rect            : (u32, u32, u32, u32),
    text            : Vec<String>,
    state           : Cell<u32>,
    pub clicked     : Cell<bool>
}

impl Widget for ButtonWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(0),
            clicked             : Cell::new(false)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {

        asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else 
        if state == 1 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else
        if state == 2 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_selection_blue(), crate::asset::TextAlignment::Center);
        }        
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            self.state.set(2);
            self.clicked.set(true);
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