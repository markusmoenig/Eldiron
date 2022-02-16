use crate::widget::node::NodeWidget;

use crate::Asset;
use crate::editor::ScreenContext;

#[derive(PartialEq)]
pub enum GraphMode {
    Overview,
    Detail
}

pub struct NodeGraph {
    rect            : (usize, usize, usize, usize),
    dirty           : bool,
    buffer          : Vec<u8>,
    graph_mode      : GraphMode,
    nodes           : Vec<NodeWidget>
}

impl NodeGraph {
    
    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext, nodes: Vec<NodeWidget>) -> Self {
        Self {
            rect,
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4],
            graph_mode          : GraphMode::Overview,
            nodes
        }
    }

    pub fn set_mode(&mut self, mode: GraphMode, rect: (usize, usize, usize, usize), context: &ScreenContext) {
        self.graph_mode = mode;
        self.rect = rect;
        self.resize(rect.2, rect.3, context)
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.buffer.resize(width * height * 4, 0);
        self.dirty = true;
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {           
            context.draw2d.draw_square_pattern(buffer_frame, &rect, rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);
            //context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);     

            if self.graph_mode == GraphMode::Overview {
                for n in &self.nodes {

                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, 251, 121), rect.2, &(250.0, 120.0), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_light_gray, 1.5);
                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, 220, 120), rect.2, &(119.0, 120.0), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_light_gray, 1.5);
                }
            }
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        // if self.contains_pos(pos) {
        //     //self.state.set(2);
        //     self.clicked.set(true);
        //     return true;
        // }
        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        // if self.state.get() == 2 {
        //     //self.state.set(1);
        //     return true;
        // }
        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}