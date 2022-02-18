use crate::widget::node::NodeWidget;

use crate::Asset;
use crate::editor::ScreenContext;

#[derive(PartialEq)]
pub enum GraphMode {
    Overview,
    Detail
}

#[derive(PartialEq)]
pub enum GraphType {
    Tiles,
}

pub struct NodeGraph {
    rect            : (usize, usize, usize, usize),
    dirty           : bool,
    buffer          : Vec<u8>,
    graph_mode      : GraphMode,
    graph_type      : GraphType,
    nodes           : Vec<NodeWidget>,

    offset          : (isize, isize),
    drag_index      : Option<usize>,
    drag_offset     : (isize, isize),
    drag_node_pos   : (isize, isize)
}

impl NodeGraph {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext, graph_type: GraphType, nodes: Vec<NodeWidget>) -> Self {
        Self {
            rect,
            dirty               : true,
            buffer              : vec![0;rect.2 * rect.3 * 4],
            graph_mode          : GraphMode::Overview,
            graph_type,
            nodes,
            offset              : (0, 0),
            drag_index          : None,
            drag_offset         : (0, 0),
            drag_node_pos       : (0, 0)
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
        let save_rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        if self.dirty {
            context.draw2d.draw_square_pattern(&mut self.buffer[..], &save_rect, save_rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

            if self.graph_mode == GraphMode::Overview {
                for index in 0..self.nodes.len() {

                    let mut selected = false;

                    if self.graph_type == GraphType::Tiles {
                        if index == context.curr_tileset_index {
                            selected = true;
                        }
                    }

                    if self.nodes[index].overview_dirty {
                        self.nodes[index].draw_overview(frame, anim_counter, asset, context, selected);
                    }
                    let rect= self.get_node_overview_rect(index, true);
                    context.draw2d.blend_slice_safe(&mut self.buffer[..], &self.nodes[index].overview_buffer[..], &rect, context.width, &save_rect);
                }
            }
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, context.width);
    }

    fn get_node_overview_rect(&self, node_index: usize, relative: bool) -> (isize, isize, usize, usize) {
        let mut x = (self.nodes[node_index].user_data.overview_position.0 + self.offset.0);
        let mut y = (self.nodes[node_index].user_data.overview_position.1 + self.offset.1);

        if relative == false {
            x += self.rect.0 as isize;
            y += self.rect.1 as isize;
        }

        (x, y, self.nodes[node_index].overview_size.0, self.nodes[node_index].overview_size.1)
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if self.graph_mode == GraphMode::Overview {
            for index in 0..self.nodes.len() {
                let rect= self.get_node_overview_rect(index, false);

                if context.contains_pos_for_isize(pos, rect) {
                    self.drag_index = Some(index);
                    self.drag_offset = (pos.0 as isize, pos.1 as isize);
                    self.drag_node_pos= (self.nodes[index].user_data.overview_position.0 as isize, self.nodes[index].user_data.overview_position.1 as isize);

                    if self.graph_type == GraphType::Tiles {
                        if context.curr_tileset_index != index {

                            self.nodes[context.curr_tileset_index].overview_dirty = true;
                            context.curr_tileset_index = index;
                            self.nodes[index].overview_dirty = true;
                            self.dirty = true;
                        }
                    }

                    return true;
                }
            }
        }
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
        if self.drag_index != None {
            self.drag_index = None;
            context.target_fps = context.default_fps;
        }
        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if let Some(index) = self.drag_index {
            let dx = pos.0 as isize - self.drag_offset.0;
            let dy = pos.1 as isize - self.drag_offset.1;

            self.nodes[index].user_data.overview_position.0 = self.drag_node_pos.0 + dx;
            self.nodes[index].user_data.overview_position.1 = self.drag_node_pos.1 + dy;
            self.dirty = true;

            context.target_fps = 60;

            //println!("here 11 {} {}", self.drag_node_pos.0 + dx, self.drag_node_pos.1 + dy);

            return true;
        }
        false
    }

    // pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
    //     false
    // }
}