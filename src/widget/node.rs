use crate::widget::*;

pub struct GroupedList {
    color               : [u8;4],
    selected_color      : [u8;4],
    items               : Vec<GroupItem>
}
struct GroupItem {
    rect                : (usize, usize, usize, usize),
    text                : String
}

#[derive(PartialEq, Debug)]
pub enum NodeWidgetType {
    Tile
}

pub struct NodeWidget {
    rect                        : (usize, usize, usize, usize),
    content_rect                : (usize, usize, usize, usize),
    pub text                    : Vec<String>,
    node_widget_type            : NodeWidgetType,
    data                        : Vec<AtomData>,
    state                       : WidgetState,
    pub clicked                 : bool,
    dirty                       : bool,
    buffer                      : Vec<u8>,

    pub disabled                : bool,
    pub selected                : bool,
    has_hover                   : bool,
}

impl NodeWidget {
    
    pub fn new(text: Vec<String>, node_widget_type: NodeWidgetType, data: Vec<AtomData>) -> Self {

        Self {
            rect                : (0,0,0,0),
            content_rect        : (0,0,0,0),
            text,
            node_widget_type,
            data,
            state               : WidgetState::Normal,
            clicked             : false,
            dirty               : true,
            buffer              : vec![],

            disabled            : false,
            selected            : false,
            has_hover           : false,
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
        }
        self.dirty = false;
        context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
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