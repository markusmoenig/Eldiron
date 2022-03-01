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

// #[derive(Serialize, Deserialize, PartialEq)]
// pub enum AtomDataType {
//     Int,
//     Float,
// }

#[derive(Serialize, Deserialize)]
pub struct AtomData {
    //atom_type                 : AtomDataType,
    pub text                    : String,
    pub id                      : String,
    pub data                    : (f64, f64, f64, f64)
}

impl AtomData {

    pub fn new_as_int(id: String, value: isize) -> Self {

        Self {
            //atom_type           : AtomDataType::Int,
            text                : "".to_string(),
            id                  : id,
            data                : (value as f64,0.0,0.0,0.0)
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum AtomWidgetType {
    ToolBarButton,
    ToolBarSliderButton,
    ToolBarSwitchButton,
    CheckButton,
    Button,
    GroupedList,
    NodeSliderButton,
}

pub struct AtomWidget {
    rect                        : (usize, usize, usize, usize),
    content_rect                : (usize, usize, usize, usize),
    pub text                    : Vec<String>,
    atom_widget_type            : AtomWidgetType,
    pub atom_data               : AtomData,
    state                       : WidgetState,
    pub clicked                 : bool,
    pub dirty                   : bool,
    buffer                      : Vec<u8>,

    pub disabled                : bool,
    pub selected                : bool,
    has_hover                   : bool,

    // For toolbar switches
    pub right_selected          : bool,
    right_has_hover             : bool,

    // For index based buttons
    pub curr_index              : usize,

    // For GroupedLists
    groups                      : Vec<GroupedList>,
    pub curr_group_index        : usize,
    pub curr_item_index         : usize,

    // Id for behavior data (BehaviorId, NodeId, AtomId)
    pub  behavior_id            : Option<(usize, usize, String)>
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
            buffer              : vec![],

            disabled            : false,
            selected            : false,
            has_hover           : false,

            right_selected      : false,
            right_has_hover     : false,

            curr_index          : 0,

            groups              : vec![],
            curr_group_index    : 0,
            curr_item_index     : 0,

            behavior_id         : None,
        }
    }

    pub fn set_rect(&mut self, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) {
        self.rect = rect;
        self.buffer = vec![0;rect.2 * rect.3 * 4];
    }

    pub fn draw(&mut self, frame: &mut [u8], stride: usize, _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {
            if self.atom_widget_type == AtomWidgetType::ToolBarButton {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.toolbar_button_height);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64, self.content_rect.3 as f64), &fill_color, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);
                context.draw2d.draw_text_rect(buffer_frame, &rect, rect.2, &asset.open_sans, context.toolbar_button_text_size, &self.text[0], &context.color_white, &fill_color, draw2d::TextAlignment::Center);
            }  else
            if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.toolbar_button_height);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64, self.content_rect.3 as f64), &fill_color, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);
                context.draw2d.draw_text_rect(buffer_frame, &rect, rect.2, &asset.open_sans, context.toolbar_button_text_size, &self.text[self.curr_index], &context.color_white, &fill_color, draw2d::TextAlignment::Center);
            }  else
            if self.atom_widget_type == AtomWidgetType::NodeSliderButton {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.node_button_height) / 2, self.rect.2 - 2, context.node_button_height - 1);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64, self.content_rect.3 as f64), &fill_color, &context.node_button_rounding, &context.color_light_gray, 1.5);
                context.draw2d.draw_text_rect(buffer_frame, &rect, rect.2, &asset.open_sans, context.node_button_text_size, &self.text[self.curr_index], &context.color_white, &fill_color, draw2d::TextAlignment::Center);
            }  else
            if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.toolbar_button_height);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);

                let div = self.content_rect.2 - 35;
                let mut left_rect = rect.clone();

                left_rect.2 = div;

                // Draw Right part
                let mut fill_color = &context.color_black;
                if self.right_has_hover  { fill_color = &context.color_light_gray } if self.right_selected { fill_color = &context.color_gray };

                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64, self.content_rect.3 as f64), &fill_color, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);

                let mut y_pos = rect.3 / 2 - 7;
                for y in 0_usize..3_usize {
                    for x in 0_usize..3_usize {
                        let color = if y == 1 && x == 1 { &context.color_white } else { &context.color_light_gray };
                        context.draw2d.draw_circle(buffer_frame, &(rect.2 - 20 - x * 5, y_pos, 6, 6), rect.2, color, 2.0);
                    }
                    y_pos += 5;
                }

                // Draw left part

                fill_color = &context.color_black;
                if self.has_hover  { fill_color = &context.color_light_gray } if self.selected { fill_color = &context.color_gray };

                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &left_rect, rect.2, &((div - 1) as f64, self.content_rect.3 as f64), &fill_color, &context.toolbar_button_rounding, &context.color_light_gray, 1.5);
                left_rect.0 += 5;
                context.draw2d.draw_text_rect(buffer_frame, &left_rect, rect.2, &asset.open_sans, context.toolbar_button_text_size, &self.text[self.curr_index], &context.color_white, &fill_color, draw2d::TextAlignment::Center);

                y_pos = rect.3 / 2 - 7;
                for _ in 0_usize..3_usize {
                    for x in 0_usize..3_usize {
                        context.draw2d.draw_circle(buffer_frame, &(10 + x * 5, y_pos, 6, 6), rect.2, &context.color_white, 2.0);
                    }
                    y_pos += 5;
                }
            }  else
            if self.atom_widget_type == AtomWidgetType::CheckButton || self.atom_widget_type == AtomWidgetType::Button {
                self.content_rect = (self.rect.0 + 1, self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2, self.rect.2 - 2, context.button_height);

                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(self.content_rect.2 as f64, self.content_rect.3 as f64), &fill_color, &context.button_rounding, &context.color_light_gray, 1.5);
                context.draw2d.draw_text_rect(buffer_frame, &rect, rect.2, &asset.open_sans, context.button_text_size, &self.text[0], &context.color_white, &fill_color, draw2d::TextAlignment::Center);
            } else
            if self.atom_widget_type == AtomWidgetType::GroupedList {

                self.content_rect = (self.rect.0, self.rect.1, self.rect.2, self.rect.3);

                let mut y = 2;
                //for (g_index, group) in self.groups.iter().enumerate() {
                for g_index in 0..self.groups.len() {

                    //for (i_index, item) in group.items.iter().enumerate() {
                    for i_index in 0..self.groups[g_index].items.len() {

                        let r = (rect.0, y, rect.2, 32);

                        let mut rounding = context.button_rounding;

                        let color: &[u8;4];

                        if g_index == self.curr_group_index && i_index == self.curr_item_index {
                            color = &self.groups[g_index].selected_color;
                        } else {
                            color = &self.groups[g_index].color;
                        }

                        if i_index == 0 {
                            rounding.0 = 0.0;
                            rounding.2 = 0.0;
                        } else
                        if i_index == &self.groups[g_index].items.len() - 1 {
                            rounding.1 = 0.0;
                            rounding.3 = 0.0;
                        } else {
                            rounding = (0.0, 0.0, 0.0, 0.0);
                        }

                        context.draw2d.draw_rounded_rect(buffer_frame, &r, rect.2, &(self.rect.2 as f64, 32.0), color, &rounding);
                        context.draw2d.draw_text(buffer_frame, &(r.0 + 25, r.1 + 4), rect.2, &asset.open_sans, context.button_text_size, &self.groups[g_index].items[i_index].text, &context.color_white, color);

                        self.groups[g_index].items[i_index].rect = r;
                        self.groups[g_index].items[i_index].rect.1 += self.rect.1;

                        y += 33;
                    }
                }
            }
        }
        self.dirty = false;
        //context.draw2d.copy_slice(frame, buffer_frame, &self.rect, context.width);
        context.draw2d.blend_slice(frame, buffer_frame, &self.rect, stride);
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        if self.contains_pos(pos) {
            if self.atom_widget_type == AtomWidgetType::ToolBarButton ||  self.atom_widget_type == AtomWidgetType::Button {
                self.clicked = true;
                return true;
            } else
            if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton {
                self.clicked = true;
                self.curr_index += 1;
                self.curr_index %= self.text.len();
                self.dirty = true;
                return true;
            } else
            if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {

                if self.contains_pos_for(pos, self.content_rect) {
                    let mut rect = self.content_rect.clone();
                    let div = (rect.2 / 4) * 3;
                    rect.2 = div;

                    if self.contains_pos_for(pos, rect) {
                        self.selected = true;
                        self.right_selected = false;
                        self.has_hover = false;
                    } else {
                        self.selected = false;
                        self.right_selected = true;
                        self.right_has_hover = false;
                    }
                }
                self.dirty = true;
                self.clicked = true;
                return true;
            } else
            if self.atom_widget_type == AtomWidgetType::GroupedList {
                for g_index in 0..self.groups.len() {
                    for i_index in 0..self.groups[g_index].items.len() {
                        if self.contains_pos_for(pos, self.groups[g_index].items[i_index].rect) {
                            self.curr_group_index = g_index;
                            self.curr_item_index = i_index;
                            self.dirty = true;
                            self.clicked = true;
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.clicked = false;
        false
    }

    pub fn mouse_dragged(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        if self.atom_widget_type == AtomWidgetType::ToolBarButton || self.atom_widget_type == AtomWidgetType::Button {
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
        } else
        if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {
            if self.contains_pos_for(pos, self.content_rect) {
                let mut rect = self.content_rect.clone();
                let div = (rect.2 / 4) * 3;
                rect.2 = div;

                if self.contains_pos_for(pos, rect) {
                    if self.selected == false {
                        self.has_hover = true;
                    }
                    self.right_has_hover = false;
                } else {
                    if self.right_selected == false {
                        self.right_has_hover = true;
                    }
                    self.has_hover = false;
                }

                self.dirty = true;
                return true;
            } else {
                if self.has_hover { self.has_hover = false; self.dirty = true; return true; }
                if self.right_has_hover { self.right_has_hover = false; self.dirty = true; return true; }
            }
        }
        false
    }

    pub fn add_group_list(&mut self, color: [u8;4], selected_color: [u8;4], items: Vec<String>) {
        let mut g_items : Vec<GroupItem> = vec![];
        for t in &items {
            let item = GroupItem {rect: (0,0,0,0), text: t.to_string()};
            g_items.push(item);
        }
        let group = GroupedList { color: color, selected_color, items: g_items };
        self.groups.push(group);
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