use crate::widget::*;

#[derive(Serialize, Deserialize)]
pub struct NodeUserData {
    pub position                : (isize, isize)
}

#[derive(PartialEq, Debug)]
pub enum NodeWidgetType {
    Overview,
    BehaviorTree,
}

pub struct NodeWidget {
    //rect                        : (usize, usize, usize, usize),
    //content_rect                : (usize, usize, usize, usize),
    pub text                    : Vec<String>,
    node_widget_type            : NodeWidgetType,
    data                        : Vec<AtomData>,
    state                       : WidgetState,
    pub clicked                 : bool,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub user_data               : NodeUserData,

    pub disabled                : bool,
    has_hover                   : bool,

    pub overview_size           : (usize, usize),
    pub size                    : (usize, usize),
}

impl NodeWidget {

    pub fn new(text: Vec<String>, node_widget_type: NodeWidgetType, data: Vec<AtomData>, user_data: NodeUserData) -> Self {

        Self {
            //rect                : (0,0,0,0),
            //content_rect        : (0,0,0,0),
            text,
            node_widget_type,
            data,
            state               : WidgetState::Normal,
            clicked             : false,

            dirty               : true,
            buffer              : vec![],

            user_data,

            disabled            : false,
            has_hover           : false,

            overview_size       : (250, 120),
            size                : (0, 0)
        }
    }


    /// Draw the node
    pub fn draw(&mut self, _frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool) {

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.overview_size.0 * self.overview_size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.overview_size.0, self.overview_size.1);

        if self.dirty {

            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, self.overview_size.1, self.overview_size.1), rect.2, &((self.overview_size.1 - 1) as f64, (self.overview_size.1 - 1) as f64), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

            context.draw2d.draw_text(buffer_frame, &(135, 85), rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, &context.color_black);

            if selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_light_white, 1.5);
            }
        }
        self.dirty = false;
    }

    /// Draw an overview node
    pub fn draw_overview(&mut self, _frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, selected: bool, preview_buffer: &[u8]) {

        if self.buffer.is_empty() {
            self.buffer = vec![0;self.overview_size.0 * self.overview_size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.overview_size.0, self.overview_size.1);

        if self.dirty {

            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, self.overview_size.1, self.overview_size.1), rect.2, &((self.overview_size.1 - 1) as f64, (self.overview_size.1 - 1) as f64), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

            context.draw2d.draw_text(buffer_frame, &(135, 85), rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, &context.color_black);

            if selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_light_white, 1.5);
            }

            context.draw2d.blend_slice(buffer_frame, preview_buffer, &(10, 10, 100, 100), rect.2);
        }
        self.dirty = false;
    }

    pub fn _mouse_down(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn _mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.clicked = false;
        false
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}