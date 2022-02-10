use crate::widget::*;

use core::cell::Cell;

use crate::draw2d::TextAlignment;

#[derive(PartialEq)]
pub enum ButtonType {
    ToolBar,
    Normal,
}

pub struct ButtonWidget {
    rect            : (usize, usize, usize, usize),
    content_rect    : (usize, usize, usize, usize),
    text            : Vec<String>,
    button_type     : ButtonType,
    state           : WidgetState,
    pub clicked     : Cell<bool>
}

impl Widget for ButtonWidget {
    
    fn new(text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self where Self: Sized {

        let content_rect = rect.clone();

        Self {
            rect,
            content_rect,
            text,
            button_type         : ButtonType::ToolBar,
            state               : WidgetState::Normal,
            clicked             : Cell::new(false)
        }
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &ScreenContext) {

        //asset.draw_rect(frame, &self.rect, [255, 255, 255, 255]);

        //context.draw2d.draw_rect(frame, &self.rect, context.width, &[255, 255, 255, 255]);

        if self.button_type == ButtonType::ToolBar {
            self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.toolbar_button_height);
            context.draw2d.draw_rounded_rect_with_border(frame, &self.rect, context.width, &(self.content_rect.2 as f64 / 2.0, self.content_rect.3 as f64 / 2.0), &context.color_black, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);
            //context.draw2d.draw_text_rect(frame, &self.content_rect, context.width, &self.text[0], [255, 255, 255, 255], context.color_black, TextAlignment::Center);
            context.draw2d.draw_text_rect(frame, &self.content_rect, context.width, &asset.open_sans,25.0, &self.text[0], &context.color_white, &context.color_black, draw2d::TextAlignment::Center);
        }
        //pub fn draw_text_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, text: &str, color: [u8; 4], background: [u8;4], align: TextAlignment) {

        /*
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
        */   
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        // if self.contains_pos(pos) {
        //     //self.state.set(2);
        //     self.clicked.set(true);
        //     return true;
        // }
        false
    }

    fn mouse_up(&mut self, _pos: (u32, u32), _asset: &mut Asset) -> bool {
        // if self.state.get() == 2 {
        //     //self.state.set(1);
        //     return true;
        // }
        false
    }

    fn set_state(&self, state: u32) {
        // if self.state.get() == 2 && state == 1 {
        //     return;
        // }
        //self.state.set(state);
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}