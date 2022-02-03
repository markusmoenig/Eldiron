use crate::{widget::*, prelude::UI_ELEMENT_HEIGHT};

use core::cell::Cell;

pub struct MenuWidget {
    rect                    : (u32, u32, u32, u32),
    text                    : Vec<String>,
    state                   : Cell<u32>,
    pub clicked             : Cell<bool>,
    pub selected_index      : Cell<u32>
}

impl Widget for MenuWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(1),
            clicked             : Cell::new(false),
            selected_index      : Cell::new(0)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {

        asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect(frame, &self.rect, self.text[self.selected_index.get() as usize].as_str(), self.get_color_text_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else {
            asset.draw_text_rect(frame, &self.rect, self.text[self.selected_index.get() as usize].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } 

        // Is open
        if state == 2 {

            let mut r = self.rect;

            r.1 += UI_ELEMENT_HEIGHT;

            for (index, text) in self.text.iter().enumerate() {

                if index as u32 == self.selected_index.get() {
                    asset.draw_text_rect(frame, &r, text.as_str(), self.get_color_text(), self.get_color_selection(), crate::asset::TextAlignment::Center);
                } else {
                    asset.draw_text_rect(frame, &r, text.as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
                }

                //asset.draw_rect_outline(frame, &r, self.get_color_text_disabled());

                r.1 += UI_ELEMENT_HEIGHT;
            }
        }
    }

    fn mouse_down(&self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.contains_pos(pos) {
            self.state.set(2);
            self.clicked.set(true);
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

    fn mouse_dragged(&self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.state.get() == 2 {

            if pos.1 > self.rect.1 + UI_ELEMENT_HEIGHT {
                let y = pos.1 - self.rect.1 - UI_ELEMENT_HEIGHT;

                let index = y / UI_ELEMENT_HEIGHT;

                if index < self.text.len() as u32 {
                    self.selected_index.set(index);
                }
            }
            return true
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