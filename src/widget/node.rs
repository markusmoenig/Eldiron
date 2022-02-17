use crate::widget::*;

pub struct GroupedList {
    color                       : [u8;4],
    selected_color              : [u8;4],
    items                       : Vec<GroupItem>
}
struct GroupItem {
    rect                        : (usize, usize, usize, usize),
    text                        : String
}

#[derive(Serialize, Deserialize)]
pub struct NodeUserData {
    pub overview_position       : (isize, isize),
    pub position                : (isize, isize)
}

#[derive(PartialEq, Debug)]
pub enum NodeWidgetType {
    Tile
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

    pub overview_dirty          : bool,
    pub overview_buffer         : Vec<u8>,

    pub user_data               : NodeUserData,

    pub disabled                : bool,
    pub selected                : bool,
    has_hover                   : bool,

    pub overview_selected       : bool,

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

            overview_dirty      : true,
            overview_buffer     : vec![],
            overview_selected   : false,

            user_data,

            disabled            : false,
            selected            : false,
            has_hover           : false,

            overview_size       : (250, 120),
            size                : (0, 0)
        }
    }

    /*
    pub fn set_rect(&mut self, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) {
        self.rect = rect;
        self.buffer = vec![0;rect.2 * rect.3 * 4];
    }*/

    pub fn draw(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        /*
        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
        */
    }

    pub fn draw_overview(&mut self, frame: &mut [u8], _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        if self.overview_buffer.is_empty() {
            self.overview_buffer = vec![0;self.overview_size.0 * self.overview_size.1 * 4];
        }

        let rect = (0_usize, 0_usize, self.overview_size.0, self.overview_size.1);
        let buffer_frame = &mut self.overview_buffer[..];

        if self.overview_dirty {
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);
            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &(0, 0, self.overview_size.1, self.overview_size.1), rect.2, &((self.overview_size.1 - 1) as f64, (self.overview_size.1 - 1) as f64), &[0,0,0,255], &(20.0, 20.0, 20.0, 20.0), &context.color_gray, 1.5);

            context.draw2d.draw_text(buffer_frame, &(135, 85), rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, &context.color_black);

            if self.overview_selected {
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &((rect.2 - 1) as f64, (rect.3 - 1) as f64), &[0,0,0,0], &(20.0, 20.0, 20.0, 20.0), &context.color_light_white, 1.5);
            }
        }
        self.overview_dirty = false;
        //context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.clicked = false;
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}