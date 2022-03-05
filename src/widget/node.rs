use server::gamedata::behavior::{GameBehaviorData, BehaviorNode, BehaviorNodeType};

use crate::atom:: { AtomData, AtomWidget, AtomWidgetType };
use crate::widget::*;

#[derive(Serialize, Deserialize)]
pub struct NodeUserData {
    pub position                : (isize, isize)
}

pub struct NodeWidget {
    pub text                    : Vec<String>,
    pub widgets                 : Vec<AtomWidget>,

    pub clicked                 : bool,

    pub id                      : usize,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub user_data               : NodeUserData,

    pub disabled                : bool,

    pub size                    : (usize, usize),

    pub clicked_id              : Option<(usize, usize, String)>
}

impl NodeWidget {

    pub fn new(text: Vec<String>, user_data: NodeUserData) -> Self {

        Self {
            text,
            widgets             : vec![],
            clicked             : false,

            id                  : 0,

            dirty               : true,
            buffer              : vec![],

            user_data,

            disabled            : false,

            size                : (250, 120),

            clicked_id          : None,
        }
    }

    pub fn new_from_behavior_data(behavior: &GameBehaviorData, behavior_node: &BehaviorNode, _asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets = vec![];

        if behavior_node.behavior_type == BehaviorNodeType::BehaviorTree {
            let mut tree1 = AtomWidget::new(vec!["Always".to_string(), "On Startup".to_string(), "On Demand".to_string()], AtomWidgetType::NodeSliderButton,
            AtomData::new_as_int("execute".to_string(), 0));
            tree1.atom_data.text = "Execute".to_string();
            let id = (behavior.id, behavior_node.id, "execute".to_string());
            tree1.behavior_id = Some(id.clone());
            tree1.curr_index = context.data.get_behavior_id_value(id).0 as usize;
            widgets.push(tree1);
        }

        Self {
            text                : vec![behavior_node.name.clone()],
            widgets,

            clicked             : false,

            id                  : behavior_node.id,

            dirty               : true,
            buffer              : vec![],

            user_data           : NodeUserData { position: behavior_node.position.clone() },

            disabled            : false,

            size                : (180, 300),

            clicked_id          : None,
        }
    }


    /// Draw the node
    pub fn draw(&mut self, _frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool) {

        let title_size = 30_usize;
        let mut height = title_size + 15;
        for atom_widget in &mut self.widgets {
            height += atom_widget.get_height(context);
            height += context.node_button_header_text_size as usize;
            height += 15;
        }

        self.size.1 = height;

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.size.0, self.size.1);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let title_color = &context.color_yellow;
            let back_color : &[u8;4] = &context.color_black;

            let rounding = &(20.0, 20.0, 20.0, 20.0);

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), title_color, rounding, &context.color_gray, 1.5);

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(rect.0, rect.1 + title_size, rect.2, rect.3 - title_size), rect.2, &((rect.2 - 1) as f64, (rect.3 - title_size - 1) as f64), back_color, rounding, &context.color_gray, 0.0);

            if selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], rounding, &context.color_light_white, 1.5);
            } else {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], rounding, &context.color_gray, 1.5);
            }

            context.draw2d.draw_text(buffer_frame, &(20, 5), rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, title_color);

            let mut y = 38_usize;

            for atom_widget in &mut self.widgets {

                context.draw2d.draw_text(buffer_frame, &(30, y), rect.2, &asset.open_sans, context.node_button_header_text_size, &atom_widget.atom_data.text, &context.color_light_white, &context.color_black);

                y += 20;
                atom_widget.set_rect((15, y, self.size.0 - 30, context.node_button_height), asset, context);
                atom_widget.draw(buffer_frame, self.size.0, anim_counter, asset, context);
            }
        }
        self.dirty = false;
    }

    /// Draw an overview node
    pub fn draw_overview(&mut self, _frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool, preview_buffer: &[u8]) {

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.size.0, self.size.1);

        if self.dirty {

            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, self.size.1, self.size.1), rect.2, &((self.size.1 - 1) as f64, (self.size.1 - 1) as f64), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

            context.draw2d.draw_text(buffer_frame, &(135, 85), rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, &context.color_black);

            if selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_light_white, 1.5);
            }

            context.draw2d.blend_slice(buffer_frame, preview_buffer, &(10, 10, 100, 100), rect.2);
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

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if self.clicked == true {

            if let Some(id) = self.clicked_id.clone() {
                for index in 0..self.widgets.len() {
                    if let Some(widget_id) = self.widgets[index].behavior_id.clone() {
                        if id == widget_id {
                            context.data.set_behavior_id_value(id.clone(), self.widgets[index].atom_data.data);
                        }
                    }
                }
            }

            self.clicked = false;
        }
        self.clicked_id = None;
        false
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}