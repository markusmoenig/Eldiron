use crate::widget::*;

#[derive(PartialEq)]
pub enum AtomWidgetType {
    ToolBarButton,
}

pub struct AtomWidget {
    rect                : (usize, usize, usize, usize),
    content_rect        : (usize, usize, usize, usize),
    text                : Vec<String>,
    atom_widget_type    : AtomWidgetType,
    atom_data           : AtomData,
    state               : WidgetState,
    pub clicked         : bool,
    dirty               : bool,
    buffer              : Vec<u8>,
}

impl AtomWidget {
    
    pub fn new(text: Vec<String>, atom_widget_type: AtomWidgetType, atom_data: AtomData) -> Self {

        Self {
            rect                : (0,0,0,0),
            content_rect        : (0,0,0,0),
            text,
            atom_widget_type,
            atom_data,
            state               : WidgetState::Normal,
            clicked             : false,
            dirty               : true,
            buffer              : vec![]
        }
    }

    pub fn set_rect(&mut self, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) {
        self.rect = rect;
        self.buffer = vec![0;rect.2 * rect.3 * 4];
    }

    pub fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {
            if self.atom_widget_type == AtomWidgetType::ToolBarButton {
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

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        if self.contains_pos(pos) {
            if self.atom_widget_type == AtomWidgetType::ToolBarButton {
                self.clicked = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.clicked = false;
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        if self.atom_widget_type == AtomWidgetType::ToolBarButton {
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
        }
        false
    }

    fn contains_pos(&self, pos: (usize, usize)) -> bool {
        let rect = self.rect;

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }
}