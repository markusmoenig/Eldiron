use crate::widget::*;
use crate::prelude::*;
use core::cell::Cell;

pub struct OptionsGridWidget {
    rect                : (u32, u32, u32, u32),
    text                : Vec<String>,
    state               : Cell<u32>,
    pub clicked         : Cell<bool>,
    spacing             : u32,
    pub selected_index  : u32
}

impl Widget for OptionsGridWidget {
    
    fn new(text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            text,
            state               : Cell::new(1),
            clicked             : Cell::new(false),
            spacing             : 8,
            selected_index      : 0
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {

        let mut x = self.rect.0;
        let mut y = self.rect.1;

        let index = self.selected_index;

        for (i, text) in self.text.iter().enumerate() {

            if self.state.get() == 0 {
                asset.draw_text_rect(frame, &(x, y, 120, UI_ELEMENT_HEIGHT), text, self.get_color_text_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
            } else {
                if index == i as u32 {
                    asset.draw_text_rect(frame, &(x, y, 120, UI_ELEMENT_HEIGHT), text, self.get_color_text(), self.get_color_selection_blue(), crate::asset::TextAlignment::Center);
                } else {
                    asset.draw_text_rect(frame, &(x, y, 120, UI_ELEMENT_HEIGHT), text, self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
                }
            }

            x += 120 + self.spacing;

            if x >= self.rect.2 {
                x = self.rect.0;
                y += UI_ELEMENT_HEIGHT + self.spacing;
            }

            //if index == 0 {
            //} else 
            //if state == 1 {
            //    asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
            //}            
        }

        /* 
        asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        //if self.state.get() == WidgetState::Normal {
        //}

        let state = self.state.get();

        if state == 0 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else 
        if state == 1 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        } else
        if state == 2 {
            asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_selection_blue(), crate::asset::TextAlignment::Center);
        }        

        //asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_text(), self.get_color_background(), crate::asset::TextAlignment::Center);
        //asset.draw_text_rect(frame, &self.rect, self.text[0].as_str(), self.get_color_disabled(), self.get_color_background(), crate::asset::TextAlignment::Center);
        */
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {

        if self.state.get() == 0 {
            return false;
        }

        if self.contains_pos(pos) {

            let mut x = self.rect.0;
            let mut y = self.rect.1;
        
            for (i, _text) in self.text.iter().enumerate() {

                if self.contains_pos_for(pos, (x, y, 120, UI_ELEMENT_HEIGHT)) {
                    self.selected_index = i as u32;
                    self.clicked.set(true);
                    return true;
                }
    
                x += 120 + self.spacing;
    
                if x > self.rect.2 {
                    x = self.rect.0;
                    y += UI_ELEMENT_HEIGHT + self.spacing;
                }         
            }
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