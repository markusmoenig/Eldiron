//use std::collections::HashMap;

//use server::gamedata::behavior::{GameBehaviorData, BehaviorNode, BehaviorNodeConnector};

use crate::atom:: { AtomWidget, AtomWidgetType, AtomData };
use crate::widget::*;

pub struct NodePreviewWidget {
    pub rect                    : (usize, usize, usize, usize),
    pub widgets                 : Vec<AtomWidget>,

    pub clicked                 : bool,

    pub id                      : usize,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub disabled                : bool,

    pub size                    : (usize, usize),

    pub clicked_id              : Option<(usize, usize, String)>,
}

impl NodePreviewWidget {

    pub fn new() -> Self {

        let run_button = AtomWidget::new(vec!["Run Behavior".to_string()], AtomWidgetType::LargeButton,
        AtomData::new_as_int("run".to_string(), 0));

        Self {
            rect                : (0,0,0,0),
            widgets             : vec![run_button],
            clicked             : false,

            id                  : 0,

            dirty               : true,
            buffer              : vec![],

            disabled            : false,

            size                : (250, 120),

            clicked_id          : None,
        }
    }

    /// Draw the node
    pub fn draw(&mut self, _frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (0, 0, self.size.0, self.size.1);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(0.0, 0.0, 20.0, 0.0), &context.color_gray, 1.5);
            context.draw2d.draw_rect(buffer_frame, &(2, 0, rect.2 - 4, 2), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(rect.2-2, 0, 2, rect.3 - 1), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(1, 1, 1, 1), stride, &[65, 65, 65, 255]);

            self.widgets[0].set_rect((20, 4, 140, 32), asset, context);
            self.widgets[0].draw(buffer_frame, stride, anim_counter, asset, context)
        }
        self.dirty = false;
    }

    /// Check if one of the atom widgets was clicked
    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_down(pos, asset, context) {
                self.dirty = true;
                self.clicked = true;
                self.clicked_id = atom_widget.behavior_id.clone();
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.clicked = false;
        self.clicked_id = None;
        false
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}