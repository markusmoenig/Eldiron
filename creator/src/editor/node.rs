use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum NodeSubType {
    None,
    Audio,
    Image,
    Tilemap,
}

pub struct NodeUserData {
    pub position                : (isize, isize)
}

pub struct NodeConnector {
    pub rect                    : (usize, usize, usize, usize)
}

pub struct NodeWidget {
    pub name                    : String,
    pub widgets                 : Vec<AtomWidget>,
    pub menu                    : Option<AtomWidget>,

    pub visible                 : bool,

    pub clicked                 : bool,

    pub id                      : Uuid,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub user_data               : NodeUserData,

    pub is_corner_node          : bool,

    pub size                    : (usize, usize),

    pub clicked_id              : Option<(Uuid, Uuid, String)>,

    pub node_connector          : HashMap<BehaviorNodeConnector, NodeConnector>,

    pub graph_offset            : (isize, isize),

    pub color                   : [u8;4],

    pub sub_type                : NodeSubType,
}

impl NodeWidget {

    pub fn new(name: String, user_data: NodeUserData) -> Self {

        Self {
            name,
            widgets             : vec![],
            menu                : None,
            clicked             : false,

            visible             : true,

            id                  : Uuid::new_v4(),

            dirty               : true,
            buffer              : vec![],

            user_data,

            is_corner_node      : false,

            size                : (260, 120),

            clicked_id          : None,

            node_connector      : HashMap::new(),

            graph_offset        : (0,0),

            color               : [0, 0, 0, 255],

            sub_type            : NodeSubType::None,
        }
    }

    pub fn new_from_behavior_data(_behavior: &GameBehaviorData, behavior_node: &BehaviorNode) -> Self {

        Self {
            name                : behavior_node.name.clone(),
            widgets             : vec![],
            menu                : None,

            visible             : true,

            clicked             : false,

            id                  : behavior_node.id,

            dirty               : true,
            buffer              : vec![],

            user_data           : NodeUserData { position: behavior_node.position.clone() },

            is_corner_node      : false,

            size                : (190, 300),

            clicked_id          : None,

            node_connector      : HashMap::new(),

            graph_offset        : (0,0),

            color               : [0, 0, 0, 255],

            sub_type            : NodeSubType::None,
        }
    }

    /// Draw the node
    pub fn draw(&mut self, _frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool) {

        let title_size = 30_usize;
        let mut height =  if self.is_corner_node == false { title_size + 25 } else { 22 };
        for atom_widget in &mut self.widgets {
            height += atom_widget.get_height(context);
            if self.is_corner_node == false {
                height += context.node_button_header_text_size as usize;
                height += 16;
            } else {
                height += 6;
            }
        }

        if self.widgets.is_empty() == true {
            height -= 2;
        }

        self.size.1 = height;

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (5, 5, self.size.0 - 10, self.size.1 - 10);
        let t_center_x = self.size.0 / 2 - 6;

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            if self.is_corner_node {
                // Corner Node

                let rounding = &(15.0, 0.0, 0.0, 0.0);

                context.draw2d.draw_rounded_rect(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3 - 2) as f64), &self.color, rounding);

                // Draw atoms

                let mut y = 18_usize;
                for atom_widget in &mut self.widgets {
                    let height = atom_widget.get_height(context);
                    atom_widget.set_rect((18, y, self.size.0 - 30, height), asset, context);
                    atom_widget.draw(buffer_frame, stride, anim_counter, asset, context);
                    y += height + 6;
                }
            } else {
                // Normal Node

                let title_color = &self.color;
                let back_color : &[u8;4] = &context.color_black;

                let rounding = &(20.0, 20.0, 20.0, 20.0);

                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), title_color, rounding, &context.color_gray, 1.5);

                if self.widgets.is_empty() == false {
                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(rect.0, rect.1 + title_size, rect.2, rect.3 - title_size), stride, &((rect.2 - 1) as f64, (rect.3 - title_size ) as f64 - 2.5), back_color, rounding, &context.color_gray, 0.0);
                }

                if selected {
                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3) as f64 - 2.5), &[0,0,0,0], rounding, &context.color_light_white, 1.5);
                } else {
                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3) as f64 - 2.5), &[0,0,0,0], rounding, &context.color_gray, 1.5);
                }

                context.draw2d.draw_text(buffer_frame, &(25, 10), stride, &asset.get_editor_font("OpenSans"), context.button_text_size, &self.name, &context.color_white, title_color);

                // Draw menu

                if let Some(menu) = &mut self.menu {
                    menu.set_rect((self.size.0 - 37, 12, 20, 20), asset, context);
                    menu.draw(buffer_frame, stride, anim_counter, asset, context);
                }

                // Draw atoms

                let mut y = 46_usize;
                for atom_widget in &mut self.widgets {
                    context.draw2d.draw_text(buffer_frame, &(30, y - 4), stride, &asset.get_editor_font("OpenSans"), context.node_button_header_text_size, &atom_widget.atom_data.text, &[180, 180, 180, 255], &context.color_black);

                    let height = atom_widget.get_height(context);

                    y += context.node_button_header_text_size as usize;
                    atom_widget.set_rect((18, y, self.size.0 - 35, height), asset, context);
                    atom_widget.draw(buffer_frame, stride, anim_counter, asset, context);

                    y += height + 10;
                }

                // Draw terminals

                let mut top_is_connected = false;
                let mut left_is_connected = false;
                if let Some(behavior) = context.data.behaviors.get(&context.data.behaviors_ids[context.curr_behavior_index]) {
                    for (_source_node_id , _source_connector, dest_node_id, dest_connector) in &behavior.data.connections {
                        if *dest_node_id == self.id {
                            if *dest_connector == BehaviorNodeConnector::Top {
                                top_is_connected = true;
                            }
                            if *dest_connector == BehaviorNodeConnector::Left {
                                left_is_connected = true;
                            }
                        }
                    }
                }

                if let Some(top) = self.node_connector.get_mut(&BehaviorNodeConnector::Top) {
                    top.rect = (t_center_x, 0, 12, 12);
                    if left_is_connected == false {
                        context.draw2d.draw_circle(buffer_frame, &top.rect, stride, &context.node_connector_color, 6.0);
                    }
                }
                if let Some(right) = self.node_connector.get_mut(&BehaviorNodeConnector::Right) {
                    right.rect = (rect.2 - 3, rect.3 / 2 + 5, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &right.rect, stride, &context.node_connector_color, 6.0);
                }
                if let Some(left) = self.node_connector.get_mut(&BehaviorNodeConnector::Left) {
                    left.rect = (0, rect.3 / 2 + 5, 12, 12);
                    if top_is_connected == false {
                        context.draw2d.draw_circle(buffer_frame, &left.rect, stride, &context.node_connector_color, 6.0);
                    }
                }

                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Bottom) {
                    bottom.rect = (t_center_x, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0);
                }
                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Bottom1) {
                    bottom.rect = (t_center_x - 60, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0);
                }
                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Bottom2) {
                    bottom.rect = (t_center_x - 30, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0);
                }
                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Bottom3) {
                    bottom.rect = (t_center_x + 30, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0);
                }
                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Bottom4) {
                    bottom.rect = (t_center_x + 60, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0);
                }

                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Success) {
                    bottom.rect = (t_center_x - 30, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.color_green, 6.0);
                    //context.draw2d._draw_circle_with_border(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0, &context.color_green, 2.0);
                }
                if let Some(bottom) = self.node_connector.get_mut(&BehaviorNodeConnector::Fail) {
                    bottom.rect = (t_center_x + 30, rect.3 - 2, 12, 12);
                    context.draw2d.draw_circle(buffer_frame, &bottom.rect, stride, &context.color_red, 6.0);
                    //context.draw2d._draw_circle_with_border(buffer_frame, &bottom.rect, stride, &context.node_connector_color, 6.0, &context.color_red, 2.0);
                }
            }
        }
        self.dirty = false;
    }

    /// Draw an overview node
    pub fn draw_overview(&mut self, _frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool, preview_buffer: &[u8], preview_clicked: bool) {

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.size.0, self.size.1);

        if self.dirty {

            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, self.size.1, self.size.1), rect.2, &((self.size.1 - 1) as f64, (self.size.1 - 1) as f64), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

            context.draw2d.draw_text_rect(buffer_frame, &(133, 80, self.size.0 - 146, 40), rect.2, &asset.get_editor_font("OpenSans"), context.button_text_size, &self.name, &context.color_white, &context.color_black, crate::draw2d::TextAlignment::Left);

            // Draw atoms

            let mut y = 12_usize;
            for atom_widget in &mut self.widgets {

                context.draw2d.draw_text(buffer_frame, &(145, y - 4), rect.2, &asset.get_editor_font("OpenSans"), context.node_button_header_text_size, &atom_widget.atom_data.text, &[180, 180, 180, 255], &context.color_black);

                let height = atom_widget.get_height(context) + 4;
                y += context.node_button_header_text_size as usize;
                atom_widget.set_rect((133, y, self.size.0 - 146, height), asset, context);
                atom_widget.draw(buffer_frame, rect.2, anim_counter, asset, context);

                y += height + 5;
            }

            if selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_light_white, 1.5);
            }

            // Draw menu

            if let Some(menu) = &mut self.menu {
                menu.set_rect((self.size.0 - 37, 12, 20, 20), asset, context);
                menu.draw(buffer_frame, rect.2, anim_counter, asset, context);
            }

            context.draw2d.blend_slice(buffer_frame, preview_buffer, &(10, 10, 100, 100), rect.2);

            if preview_clicked {
                context.draw2d.blend_rounded_rect(buffer_frame, &(0, 0, self.size.1, self.size.1), rect.2, &((self.size.1 - 1) as f64, (self.size.1 - 1) as f64), &context.color_light_gray, &(20.0, 20.0, 20.0, 20.0));
            }
        }
        self.dirty = false;
    }

    /// Check if one of the atom widgets was clicked
    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if let Some(menu) = &mut self.menu {
            if menu.mouse_down(pos, asset, context) {
                self.dirty = true;
                self.clicked = true;
                self.clicked_id = menu.behavior_id.clone();
                return true;
            }
        }

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

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        let mut consumed = false;

        if let Some(menu) = &mut self.menu {
            if menu.mouse_up(pos, asset, context) {
                self.dirty = true;
                self.clicked = true;
                return true;
            }
        }

        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_up(pos, asset, context) {
                self.dirty = true;
                self.clicked = true;
            }
        }

        if self.clicked == true {

            // Save the data
            if let Some(id) = self.clicked_id.clone() {
                for index in 0..self.widgets.len() {
                    if let Some(widget_id) = self.widgets[index].behavior_id.clone() {
                        if id == widget_id {
                            context.data.set_behavior_id_value(id.clone(), self.widgets[index].atom_data.value.clone(), context.curr_graph_type);
                        }
                    }
                }
            }

            consumed = true;
            self.clicked = false;
        }
        self.clicked_id = None;
        consumed
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}