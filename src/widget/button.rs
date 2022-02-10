use crate::widget::*;

use core::cell::Cell;

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
    pub clicked     : Cell<bool>,
    dirty           : bool,
    buffer          : Vec<u8>,
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
            clicked             : Cell::new(false),
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4]
        }
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {
            if self.button_type == ButtonType::ToolBar {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.toolbar_button_height);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64 / 2.0, self.content_rect.3 as f64 / 2.0), &fill_color, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);
                context.draw2d.draw_text_rect(buffer_frame, &rect, rect.2, &asset.open_sans, context.toolbar_button_text_size, &self.text[0], &context.color_white, &fill_color, draw2d::TextAlignment::Center);
            }            
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        // if self.contains_pos(pos) {
        //     //self.state.set(2);
        //     self.clicked.set(true);
        //     return true;
        // }
        false
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset) -> bool {
        // if self.state.get() == 2 {
        //     //self.state.set(1);
        //     return true;
        // }
        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset) -> bool {
        //println!("{}");
        if self.contains_pos_for(pos, self.content_rect) {
            if self.state != WidgetState::Disabled {
                if self.state != WidgetState::Hover {
                    self.state = WidgetState::Hover;
                    self.dirty = true;
                    return true;
                }
            }
        } else {
            if self.state != WidgetState::Disabled {
                if self.state == WidgetState::Hover {
                    self.state = WidgetState::Normal;
                    self.dirty = true;
                    return true;
                }
            }
        }
        false
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}