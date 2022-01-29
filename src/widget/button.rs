use crate::widget::*;

use core::cell::Cell;

pub struct ButtonWidget {
    rect            : (u32, u32, u32, u32),
    text            : Vec<String>,
    state           : Cell<u32>
}

impl Widget for ButtonWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(0)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {

        asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        //if self.state.get() == WidgetState::Normal {
        //}

        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        }

        //asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        //asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
    }

    fn mouse_down(&self, pos: (u32, u32)) -> bool {
        println!("text {:?}", pos);

        false
    }

    fn set_state(&self, state: u32) {
        self.state.set(state);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}