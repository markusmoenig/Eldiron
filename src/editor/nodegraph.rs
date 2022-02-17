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
    nodes           : Vec<NodeWidget>,

    offset          : (isize, isize)
}

impl NodeGraph {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext, nodes: Vec<NodeWidget>) -> Self {
        Self {
            rect,
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4],
            graph_mode          : GraphMode::Overview,
            nodes,
            offset              : (0, 0)
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

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {
            context.draw2d.draw_square_pattern(buffer_frame, &rect, rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);
            //context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);

            if self.graph_mode == GraphMode::Overview {
                for index in 0..self.nodes.len() {

                    let mut pos = (self.nodes[index].user_data.overview_position.0, self.nodes[index].user_data.overview_position.1);

                    if self.nodes[index].overview_dirty {
                        self.nodes[index].draw_overview(frame, anim_counter, asset, context);

                        let x = self.nodes[index].user_data.overview_position.0 as usize;
                        let y = self.nodes[index].user_data.overview_position.1 as usize;

                        let rect = (x, y, self.nodes[index].overview_size.0, self.nodes[index].overview_size.1);
                        context.draw2d.blend_slice(buffer_frame, &self.nodes[index].overview_buffer[..], &rect, context.width);
                    }

                    //context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(pos.0 as usize, pos.1 as usize, 250, 121), rect.2, &(249.0, 120.0), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
                    //context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(100, 100, 220, 120), rect.2, &(119.0, 120.0), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

                    //context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(100, 100, 250, 121), rect.2, &(249.0, 120.0), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_white, 1.5);

                    //context.draw2d.draw_text(buffer_frame, &(150, 120 + 4), rect.2, &asset.open_sans, context.button_text_size, &node.text[0], &context.color_white, &context.color_black);
                }
            }
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
    }

    fn get_node_overview_rect(&self, node_index: usize) -> (usize, usize, usize, usize) {
        let x = self.nodes[node_index].user_data.overview_position.0 as usize;
        let y = self.nodes[node_index].user_data.overview_position.1 as usize;

        (x, y, self.nodes[node_index].overview_size.0, self.nodes[node_index].overview_size.1)
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