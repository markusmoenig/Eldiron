use crate::prelude::*;

const SCROLLBAR_WIDTH: usize = 18;

pub struct GroupedList {
    color: [u8; 4],
    selected_color: [u8; 4],
    pub items: Vec<GroupItem>,
}

pub struct GroupItem {
    pub rect: (usize, usize, usize, usize),
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct AtomData {
    pub text: String,
    pub id: String,
    pub data: (f32, f32, f32, f32, String),
    pub value: Value,
}

impl AtomData {
    pub fn new(id: &str, value: Value) -> Self {
        Self {
            text: "".to_string(),
            id: id.to_string(),
            data: (0.0, 0.0, 0.0, 0.0, "".to_string()),
            value: value,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum AtomWidgetType {
    ToolBarButton,
    ToolBarSliderButton,
    ToolBarMenuButton,
    ToolBarSwitchButton,
    ToolBarCheckButton,

    NodeSliderButton,
    NodeMenuButton,
    NodeIntSlider,
    NodeNumberButton,
    NodeSize2DButton,
    NodeExpressionButton,
    NodeExpressionValueButton,
    NodeScriptButton,
    NodeTextButton,
    NodeMenu,
    NodePositionButton,
    NodeCharTileButton,
    NodeIconTileButton,
    NodeEnvTileButton,
    NodeEffectTileButton,
    NodeGridSizeButton,
    NodeScreenButton,
    NodePropertyLog,
    NodeItemSettingsButton,
    NodeSpellSettingsButton,
    NodeTimeButton,
    NodeCharacterSettingsButton,
    NodeRevealScriptsButton,

    LargeButton,
    CheckButton,
    Button,
    TileScaleButton,
    GroupedList,
    MenuButton,
    TagsButton,
    SliderButton,
    SmallMenuButton,
    LayerNumberRow,
    IconRow,
    CheckedIcon,
    EnabledIcon,
}

pub struct AtomWidget {
    pub rect: (usize, usize, usize, usize),
    pub content_rect: (usize, usize, usize, usize),
    pub text: Vec<String>,
    pub atom_widget_type: AtomWidgetType,
    pub atom_data: AtomData,
    pub state: WidgetState,
    pub clicked: bool,
    pub dirty: bool,
    pub dirty_scroll: bool,
    buffer: Vec<u8>,

    pub selected: bool,
    has_hover: bool,
    pub checked: bool,

    pub no_border: bool,

    // For toolbar switches
    pub right_selected: bool,
    right_has_hover: bool,

    // For index based buttons
    pub curr_index: usize,

    // For GroupedLists
    pub groups: Vec<GroupedList>,
    pub curr_group_index: usize,
    pub curr_item_index: usize,
    pub centered_text: bool,

    // For Menus
    pub new_selection: Option<usize>,

    // Id for behavior data (BehaviorId, NodeId, AtomId)
    pub behavior_id: Option<(Uuid, Uuid, String)>,

    // Drag
    pub drag_enabled: bool,
    pub drag_context: Option<ScreenDragContext>,

    // For embedded atoms (in a node), provide the offset to the absolute position
    pub emb_offset: (isize, isize),

    pub custom_color: Option<[u8; 4]>,

    pub status_help_text: Option<String>,

    pub status_help_vector: Option<Vec<(String, String)>>,

    pub button_mask: Option<Vec<Option<TileData>>>,

    pub debug_value: Option<f32>,

    grouped_list_height: isize,
    scroll_offset: isize,
    scroll_distance: isize,
    scroll_thumb_rect: Option<(usize, usize, usize, usize)>,
    scroll_click: Option<(usize, usize)>,
    scroll_offset_start: isize,
}

impl AtomWidget {
    pub fn new(text: Vec<String>, atom_widget_type: AtomWidgetType, atom_data: AtomData) -> Self {
        Self {
            rect: (0, 0, 0, 0),
            content_rect: (0, 0, 0, 0),
            text,
            atom_widget_type,
            atom_data,
            state: WidgetState::Normal,
            clicked: false,
            dirty: true,
            dirty_scroll: false,
            buffer: vec![],

            selected: false,
            has_hover: false,
            checked: false,

            no_border: false,

            right_selected: false,
            right_has_hover: false,

            curr_index: 0,

            groups: vec![],
            curr_group_index: 0,
            curr_item_index: 0,
            centered_text: false,

            new_selection: None,

            behavior_id: None,

            drag_enabled: false,
            drag_context: None,

            emb_offset: (0, 0),
            custom_color: None,

            status_help_text: None,

            status_help_vector: None,

            button_mask: None,

            debug_value: None,

            scroll_offset: 0,
            grouped_list_height: 0,
            scroll_distance: 0,
            scroll_thumb_rect: None,
            scroll_click: None,
            scroll_offset_start: 0,
        }
    }

    pub fn set_rect(&mut self, rect: (usize, usize, usize, usize)) {
        self.rect = rect;
        let mut height = rect.3;
        if self.atom_widget_type == AtomWidgetType::GroupedList {
            self.scroll_distance = 0;
            let h = self.grouped_list_height();
            if h > height {
                self.scroll_distance = h as isize - height as isize;
            }
            height = height.max(h);
            self.grouped_list_height = height as isize;
        }
        if self.buffer.len() != rect.2 * height * 4 {
            self.buffer = vec![0; rect.2 * height * 4];
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut [u8],
        stride: usize,
        _anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) {
        let rect = (0_usize, 0_usize, self.rect.2, self.rect.3);
        let buffer_frame = &mut self.buffer[..];

        if self.dirty && self.dirty_scroll == false {
            // Toolbar

            if self.atom_widget_type == AtomWidgetType::ToolBarButton
                || self.atom_widget_type == AtomWidgetType::ToolBarCheckButton
            {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                    self.rect.2 - 2,
                    context.toolbar_button_height,
                );

                let mut border_color = context.color_light_gray;
                if let Some(custom_color) = self.custom_color {
                    border_color = custom_color;
                }

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &[0, 0, 0, 0]); //context.color_toolbar);

                let fill_color;
                if self.atom_widget_type == AtomWidgetType::ToolBarButton {
                    fill_color = if self.state != WidgetState::Clicked {
                        &[0, 0, 0, 0] /*context.color_toolbar*/
                    } else {
                        &context.color_light_gray
                    };
                } else {
                    if self.state == WidgetState::Hover {
                        fill_color = &context.color_light_gray;
                    } else {
                        fill_color = if self.checked == false {
                            &[0, 0, 0, 0] /*context.color_toolbar*/
                        } else {
                            &context.color_gray
                        };
                    }
                }

                if self.no_border == false {
                    context.draw2d.draw_rounded_rect_with_border(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                        &fill_color,
                        &context.toolbar_button_rounding,
                        &border_color,
                        1.5,
                    );
                } else {
                    context.draw2d.draw_rounded_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                        &fill_color,
                        &context.toolbar_button_rounding,
                    );
                }
                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.toolbar_button_text_size,
                    &self.text[0],
                    &if self.state == WidgetState::Disabled {
                        context.color_gray
                    } else {
                        context.color_white
                    },
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                    self.rect.2 - 2,
                    context.toolbar_button_height,
                );

                let mut border_color = context.color_light_gray;
                if let Some(custom_color) = self.custom_color {
                    border_color = custom_color;
                }

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &context.color_toolbar);
                let fill_color = &context.color_toolbar; //if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };

                if self.no_border == false {
                    context.draw2d.draw_rounded_rect_with_border(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                        &fill_color,
                        &context.toolbar_button_rounding,
                        &border_color,
                        1.5,
                    );
                } else {
                    context.draw2d.draw_rounded_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                        &fill_color,
                        &context.toolbar_button_rounding,
                    );
                }

                if self.text.len() > 0 && self.curr_index < self.text.len() {
                    let text_color = if self.state == WidgetState::Disabled {
                        &context.color_gray
                    } else {
                        &context.color_white
                    };
                    context.draw2d.draw_text_rect(
                        buffer_frame,
                        &(rect.0 + 30, rect.1, rect.2 - 60, rect.3),
                        rect.2,
                        &asset.get_editor_font("OpenSans"),
                        context.toolbar_button_text_size,
                        &self.text[self.curr_index],
                        text_color,
                        &fill_color,
                        draw2d::TextAlignment::Center,
                    );
                }

                // Right Arrow

                let arrow_y = rect.3 / 2 - 9;

                let left_color = if self.has_hover && self.text.len() > 1 {
                    &context.color_light_gray
                } else {
                    &context.color_gray
                };
                let right_color = if self.right_has_hover && self.text.len() > 1 {
                    &context.color_light_gray
                } else {
                    &context.color_gray
                };
                context.draw2d.blend_mask(
                    buffer_frame,
                    &(rect.2 + 14, arrow_y, 12, 14),
                    rect.2,
                    &context.left_arrow_mask[..],
                    &(12, 18),
                    &left_color,
                );
                context.draw2d.blend_mask(
                    buffer_frame,
                    &(rect.2 - 26, arrow_y, 12, 14),
                    rect.2,
                    &context.right_arrow_mask[..],
                    &(12, 18),
                    &right_color,
                );
            } else if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                    self.rect.2 - 2,
                    context.toolbar_button_height,
                );

                let mut border_color = context.color_light_gray;
                if let Some(custom_color) = self.custom_color {
                    border_color = custom_color;
                }

                //context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);

                let div = self.content_rect.2 - 35;
                let mut left_rect = rect.clone();

                left_rect.2 = div;

                // Draw Right part
                let mut fill_color = context.color_black;
                if self.right_has_hover {
                    fill_color = context.color_light_gray
                }
                if self.right_selected {
                    fill_color = context.color_gray
                };

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.toolbar_button_rounding,
                    &border_color,
                    1.5,
                );

                let mut y_pos = rect.3 / 2 - 7;
                for y in 0_usize..3_usize {
                    for x in 0_usize..3_usize {
                        let color = if y == 1 && x == 1 {
                            &context.color_white
                        } else {
                            &context.color_light_gray
                        };
                        context.draw2d.draw_circle(
                            buffer_frame,
                            &(rect.2 - 20 - x * 5, y_pos, 6, 6),
                            rect.2,
                            color,
                            2.0,
                        );
                    }
                    y_pos += 5;
                }

                // Draw left part

                fill_color = context.color_black;
                if self.has_hover {
                    fill_color = context.color_light_gray
                }
                if self.selected {
                    fill_color = context.color_gray
                };

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &left_rect,
                    rect.2,
                    &((div - 1) as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.toolbar_button_rounding,
                    &border_color,
                    1.5,
                );
                left_rect.0 += 5;
                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &left_rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.toolbar_button_text_size,
                    &self.text[self.curr_index],
                    &context.color_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );

                y_pos = rect.3 / 2 - 7;
                for _ in 0_usize..3_usize {
                    for x in 0_usize..3_usize {
                        context.draw2d.draw_circle(
                            buffer_frame,
                            &(10 + x * 5, y_pos, 6, 6),
                            rect.2,
                            &context.color_white,
                            2.0,
                        );
                    }
                    y_pos += 5;
                }
            } else if self.atom_widget_type == AtomWidgetType::ToolBarMenuButton {
                if self.state != WidgetState::Clicked {
                    self.content_rect = (
                        self.rect.0 + 1,
                        self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                        self.rect.2 - 2,
                        context.toolbar_button_height,
                    );

                    context
                        .draw2d
                        .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                    let fill_color = &context.color_black;
                    context.draw2d.draw_rounded_rect_with_border(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                        &fill_color,
                        &context.toolbar_button_rounding,
                        &context.color_light_gray,
                        1.5,
                    );
                    if self.text.len() > 0 {
                        context.draw2d.draw_text_rect(
                            buffer_frame,
                            &rect,
                            rect.2,
                            &asset.get_editor_font("OpenSans"),
                            context.toolbar_button_text_size,
                            &self.text[self.curr_index],
                            &context.color_white,
                            &fill_color,
                            draw2d::TextAlignment::Center,
                        );
                    }

                    // Triangle
                    let color = if self.state == WidgetState::Hover && self.text.len() > 1 {
                        &context.color_light_gray
                    } else {
                        &context.color_gray
                    };

                    context.draw2d.blend_mask(
                        buffer_frame,
                        &(rect.2 - 25, 20, rect.2, rect.3),
                        rect.2,
                        &context.menu_triangle_mask[..],
                        &(10, 10),
                        &color,
                    );
                }
            } else
            // Node
            if self.atom_widget_type == AtomWidgetType::NodeSliderButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.node_button_height) / 2,
                    self.rect.2 - 2,
                    context.node_button_height - 1,
                );

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal {
                    &context.color_black
                } else {
                    &context.color_light_gray
                };
                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.node_button_rounding,
                    &context.color_light_gray,
                    1.5,
                );
                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &self.text[self.curr_index],
                    &context.color_light_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeMenuButton
                || self.atom_widget_type == AtomWidgetType::SmallMenuButton
            {
                if self.state != WidgetState::Clicked {
                    self.content_rect = (
                        self.rect.0 + 1,
                        self.rect.1 + (self.rect.3 - context.node_button_height) / 2,
                        self.rect.2 - 2,
                        context.node_button_height,
                    );

                    let fill_color = if self.atom_widget_type == AtomWidgetType::SmallMenuButton {
                        context.color_black
                    } else {
                        context.color_node_light_gray
                    };
                    let border_color = if self.atom_widget_type == AtomWidgetType::SmallMenuButton {
                        context.color_light_gray
                    } else {
                        context.color_node_light_gray
                    };

                    //context.draw2d.draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                    context.draw2d.draw_rounded_rect_with_border_2(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &fill_color,
                        &context.node_button_rounding,
                        &border_color,
                        1.5,
                    );

                    if self.text.len() > 0 && self.curr_index < self.text.len() {
                        context.draw2d.draw_text_rect(
                            buffer_frame,
                            &(rect.0, rect.1, rect.2, rect.3 - 3),
                            rect.2,
                            &asset.get_editor_font("OpenSans"),
                            context.node_button_text_size,
                            &self.text[self.curr_index],
                            &context.color_light_white,
                            &fill_color,
                            draw2d::TextAlignment::Center,
                        );
                    }

                    // Triangle
                    let mut color = if self.state == WidgetState::Hover && self.text.len() > 1 {
                        &context.color_light_gray
                    } else {
                        &context.color_gray
                    };

                    if self.atom_widget_type == AtomWidgetType::NodeMenuButton {
                        color = if self.state == WidgetState::Hover && self.text.len() > 1 {
                            &context.color_light_white
                        } else {
                            &context.color_node_picker
                        };
                    }

                    context.draw2d.blend_mask(
                        buffer_frame,
                        &(rect.2 - 25, 9, rect.2, rect.3),
                        rect.2,
                        &context.menu_triangle_mask[..],
                        &(10, 10),
                        &color,
                    );
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeIntSlider {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let fill_color = if self.atom_widget_type == AtomWidgetType::SmallMenuButton {
                    context.color_black
                } else {
                    context.color_node_dark_gray
                };
                let border_color = if self.atom_widget_type == AtomWidgetType::SmallMenuButton {
                    context.color_light_gray
                } else {
                    context.color_node_dark_gray
                };

                let v = self.atom_data.data.0.round();

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                    &fill_color,
                    &context.node_button_rounding,
                    &border_color,
                    1.5,
                );

                let min = self.atom_data.data.1;

                if v > min {
                    let max = self.atom_data.data.2;
                    let pp = self.content_rect.2 as f32 / (max - min);

                    let mut r = rect.clone();
                    let left_off = ((v - 1.0) * pp).round() as usize;

                    if left_off < r.2 {
                        r.2 = left_off;
                        let mut round = context.node_button_rounding.clone();
                        if v < max {
                            round.0 = 0.0;
                            round.1 = 0.0;
                        } else {
                            r.2 = rect.2;
                        }

                        context.draw2d.draw_rounded_rect_with_border(
                            buffer_frame,
                            &r,
                            rect.2,
                            &(r.2 as f32, r.3 as f32 - 1.0),
                            &context.color_node_light_gray,
                            &round,
                            &&context.color_node_light_gray,
                            1.5,
                        );
                    }
                }

                context.draw2d.blend_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &format!("{}", v),
                    &context.color_white,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeNumberButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let fill_color = if self.state == WidgetState::Clicked {
                    context.color_node_dark_gray
                } else {
                    context.color_gray
                };

                let mut v = 0.0_f32;
                if let Some(value) = self.atom_data.value.to_float() {
                    v = value;
                }

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                    &fill_color,
                    &context.node_button_rounding,
                    &fill_color,
                    1.5,
                );

                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &format!("{}", v),
                    &context.color_light_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeTimeButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let fill_color = if self.state == WidgetState::Clicked {
                    context.color_node_dark_gray
                } else {
                    context.color_gray
                };

                let mut v = "".to_string();
                if let Some(value) = self.atom_data.value.to_date() {
                    v = value.to_time24();
                }

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                    &fill_color,
                    &context.node_button_rounding,
                    &fill_color,
                    1.5,
                );

                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &format!("{}", v),
                    &context.color_light_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeSize2DButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let fill_color = if self.state == WidgetState::Clicked {
                    context.color_light_orange
                } else {
                    context.color_orange
                };

                let v1 = self.atom_data.data.0.round();
                let v2 = self.atom_data.data.0.round();

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                    &fill_color,
                    &context.node_button_rounding,
                    &fill_color,
                    1.5,
                );

                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &format!("{} x {}", v1, v2),
                    &context.color_light_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeExpressionButton
                || self.atom_widget_type == AtomWidgetType::NodeExpressionValueButton
                || self.atom_widget_type == AtomWidgetType::NodeTextButton
                || self.atom_widget_type == AtomWidgetType::NodeGridSizeButton
                || self.atom_widget_type == AtomWidgetType::NodeScriptButton
                || self.atom_widget_type == AtomWidgetType::NodeScreenButton
                || self.atom_widget_type == AtomWidgetType::NodeItemSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeSpellSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeCharacterSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeRevealScriptsButton
            {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let mut fill_color = if self.state == WidgetState::Clicked {
                    &context.color_node_dark_gray
                } else {
                    &context.color_node_light_gray
                };

                if self.debug_value.is_some() {
                    fill_color = &context.color_red;
                }

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                    fill_color,
                    &context.node_button_rounding,
                    fill_color,
                    1.5,
                );

                let t = self.atom_data.value.to_string_value();
                let chars = t.chars();
                let text: String = chars.into_iter().take(20).collect();

                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &(rect.0 + 5, rect.1, rect.2 - 10, rect.3),
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &text,
                    &context.color_light_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeMenu {
                self.content_rect = self.rect.clone();

                let fill_color = if self.state == WidgetState::Clicked {
                    context.color_white
                } else {
                    [210, 210, 210, 255]
                };

                context
                    .draw2d
                    .draw_rect(buffer_frame, &(0, 4, 15, 2), rect.2, &fill_color);
                context
                    .draw2d
                    .draw_rect(buffer_frame, &(0, 10, 15, 2), rect.2, &fill_color);
                context
                    .draw2d
                    .draw_rect(buffer_frame, &(0, 16, 15, 2), rect.2, &fill_color);
                //context.draw2d.blend_mask(buffer_frame, &(0, 0, rect.2, rect.3), rect.2, &context.menu_mask[..], &(20, 20), &context.color_white);
            } else if self.atom_widget_type == AtomWidgetType::NodePositionButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height * 2,
                );

                let border_color = if context.active_position_id == self.behavior_id {
                    context.color_red
                } else {
                    context.color_node_light_gray
                };

                context.draw2d.draw_rounded_rect_with_border_2(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &context.color_black,
                    &context.node_button_rounding,
                    &border_color,
                    1.5,
                );

                match &self.atom_data.value {
                    Value::Position(pos) => {
                        if let Some(region) = context.data.regions.get(&pos.region) {
                            let mut mask_buffer = vec![0.0; rect.2 * rect.3];
                            context.draw2d.create_rounded_rect_mask(
                                &mut mask_buffer[..],
                                &(1, 1, rect.2 - 3, rect.3 - 3),
                                rect.2,
                                &context.node_button_rounding,
                            );

                            context.draw2d.mask = Some(mask_buffer.clone());
                            context.draw2d.mask_size = (rect.2, rect.3);

                            context.draw2d.draw_region_centered_with_behavior(
                                buffer_frame,
                                region,
                                &(0, 0, rect.2, rect.3),
                                &(pos.x as isize, pos.y as isize),
                                &(0, 0),
                                rect.2,
                                14,
                                0,
                                asset,
                                context,
                            );

                            context.draw2d.mask = None;
                        }
                    }
                    _ => {}
                }

                if self.clicked {
                    context.draw2d.blend_rounded_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                        &context.color_light_gray,
                        &context.node_button_rounding,
                    );
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeCharTileButton
                || self.atom_widget_type == AtomWidgetType::NodeEnvTileButton
                || self.atom_widget_type == AtomWidgetType::NodeIconTileButton
                || self.atom_widget_type == AtomWidgetType::NodeEffectTileButton
            {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + ((self.rect.3 - context.node_button_height) / 2),
                    self.rect.2 - 2,
                    context.node_button_height,
                );

                let fill_color = if self.state == WidgetState::Clicked {
                    context.color_node_dark_gray
                } else {
                    context.color_black
                };
                let border_color = if self.state == WidgetState::Clicked {
                    context.color_node_dark_gray
                } else {
                    context.color_node_light_gray
                };

                context.draw2d.draw_rounded_rect_with_border_2(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &fill_color,
                    &context.node_button_rounding,
                    &border_color,
                    1.5,
                );

                //context.draw2d.draw_text(buffer_frame, &(25, 1), rect.2, &asset.open_sans, context.node_button_text_size, &"Default Tile:".to_string(), &context.color_white, &fill_color);

                if let Some(id) = self.atom_data.value.to_tile_id() {
                    if let Some(map) = asset.get_map_of_id(id.tilemap) {
                        context.draw2d.draw_animated_tile(
                            buffer_frame,
                            &(rect.2 / 2 - 9, 2),
                            map,
                            rect.2,
                            &(id.x_off as usize, id.y_off as usize),
                            0,
                            18,
                        );
                    }
                }
            } else if self.atom_widget_type == AtomWidgetType::NodePropertyLog {
                let b = &context.color_black;
                context.draw2d.draw_rect(buffer_frame, &rect, rect.2, b);

                if let Some(icon) = context.icons.get(&"attrs".to_string()) {
                    context.draw2d.scale_chunk(
                        buffer_frame,
                        &(rect.0, rect.1, 18, 18),
                        rect.2,
                        &icon.0[..],
                        &(icon.1 as usize, icon.2 as usize),
                        if self.curr_index == 0 { 0.8 } else { 0.5 },
                    );
                }

                if let Some(icon) = context.icons.get(&"helmet".to_string()) {
                    context.draw2d.scale_chunk(
                        buffer_frame,
                        &(rect.0 + 24, rect.1, 18, 18),
                        rect.2,
                        &icon.0[..],
                        &(icon.1 as usize, icon.2 as usize),
                        if self.curr_index == 1 { 0.8 } else { 0.5 },
                    );
                }

                if let Some(icon) = context.icons.get(&"bag".to_string()) {
                    context.draw2d.scale_chunk(
                        buffer_frame,
                        &(rect.0 + 24 * 2, rect.1, 18, 18),
                        rect.2,
                        &icon.0[..],
                        &(icon.1 as usize, icon.2 as usize),
                        if self.curr_index == 2 { 0.8 } else { 0.5 },
                    );
                }

                if let Some(icon) = context.icons.get(&"log".to_string()) {
                    context.draw2d.scale_chunk(
                        buffer_frame,
                        &(rect.0 + 24 * 3, rect.1, 18, 18),
                        rect.2,
                        &icon.0[..],
                        &(icon.1 as usize, icon.2 as usize),
                        if self.curr_index == 3 { 0.8 } else { 0.5 },
                    );
                }

                if let Some(icon) = context.icons.get(&"logo".to_string()) {
                    let size = 128;
                    context.draw2d.scale_chunk(
                        buffer_frame,
                        &((rect.2 - size) / 2, (rect.3 - 128) / 2 + 10, size, size),
                        rect.2,
                        &icon.0[..],
                        &(icon.1 as usize, icon.2 as usize),
                        0.1,
                    );
                }

                //context.draw2d.draw_rect(buffer_frame, &(rect.0, rect.1 + 20, rect.2, 1), rect.2, &context.color_gray);

                if context.is_debugging {
                    self.content_rect = (self.rect.0, self.rect.1, self.rect.2, self.rect.3);
                    let r = rect;

                    let text_size = 12_usize;
                    let max_lines = (self.content_rect.3 - 3 * text_size) / (text_size as usize);

                    let font = &asset.get_editor_font("OpenSans");
                    let w = &context.color_light_white;
                    let mut color = w;

                    let y_start = r.1 + 2 * text_size;

                    if self.curr_index == 0 {
                        // Abilities

                        let mut index = 0;
                        for (name, value) in context.debug_sheet.abilities.iter() {
                            if index >= max_lines {
                                break;
                            }

                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &(r.0, y_start + index * (text_size as usize), r.2, text_size),
                                rect.2,
                                font,
                                text_size as f32,
                                name.as_str(),
                                color,
                                b,
                                crate::draw2d::TextAlignment::Left,
                            );

                            let str: String = value.to_string();

                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &(r.0, y_start + index * (text_size as usize), r.2, text_size),
                                rect.2,
                                font,
                                text_size as f32,
                                str.as_str(),
                                color,
                                b,
                                crate::draw2d::TextAlignment::Right,
                            );

                            index += 1;
                        }
                    } else if self.curr_index == 2 {
                        // Inventory
                        for (index, item) in context.debug_sheet.inventory.items.iter().enumerate()
                        {
                            if index >= max_lines {
                                break;
                            }

                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &(r.0, y_start + index * (text_size as usize), r.2, text_size),
                                rect.2,
                                font,
                                text_size as f32,
                                item.name.as_str(),
                                color,
                                b,
                                crate::draw2d::TextAlignment::Left,
                            );

                            let str = item.amount.to_string();

                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &(r.0, y_start + index * (text_size as usize), r.2, text_size),
                                rect.2,
                                font,
                                text_size as f32,
                                str.as_str(),
                                color,
                                b,
                                crate::draw2d::TextAlignment::Right,
                            );
                        }
                    } else if self.curr_index == 3 {
                        // Messages
                        let available_messages = context.debug_messages.len();

                        for l in 0..max_lines {
                            if l >= available_messages {
                                break;
                            }

                            match &context.debug_messages[available_messages - 1 - l].message_type {
                                MessageType::Debug => color = &context.color_light_green,
                                MessageType::Error => color = &context.color_light_orange,
                                _ => {}
                            }

                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &(
                                    r.0,
                                    r.3 - 10 - (l + 1) * (text_size as usize),
                                    r.2,
                                    text_size,
                                ),
                                rect.2,
                                font,
                                text_size as f32,
                                context.debug_messages[available_messages - 1 - l]
                                    .message
                                    .as_str(),
                                color,
                                b,
                                crate::draw2d::TextAlignment::Left,
                            );
                        }
                    }
                }
            }

            // Large
            if self.atom_widget_type == AtomWidgetType::LargeButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.large_button_height) / 2,
                    self.rect.2 - 2,
                    context.large_button_height,
                );

                context
                    .draw2d
                    .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                let fill_color = if self.state == WidgetState::Normal {
                    &context.color_black
                } else {
                    &context.color_light_gray
                };
                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.large_button_rounding,
                    &context.color_light_gray,
                    1.3,
                );
                context.draw2d.draw_text_rect(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &asset.get_editor_font("OpenSans"),
                    context.large_button_text_size,
                    &self.text[0],
                    &context.color_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
            } else
            // Normal
            if self.atom_widget_type == AtomWidgetType::Button
                || self.atom_widget_type == AtomWidgetType::TagsButton
                || self.atom_widget_type == AtomWidgetType::CheckButton
            {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let fill_color;
                if self.atom_widget_type != AtomWidgetType::CheckButton {
                    fill_color = if self.state != WidgetState::Clicked {
                        &context.color_black
                    } else {
                        &context.color_light_gray
                    };
                } else {
                    if self.state == WidgetState::Hover {
                        fill_color = &context.color_light_gray;
                    } else {
                        fill_color = if self.checked == false {
                            &context.color_black
                        } else {
                            &context.color_gray
                        };
                    }
                }

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.button_rounding,
                    &if self.state == WidgetState::Disabled {
                        context.color_gray
                    } else {
                        context.color_light_gray
                    },
                    1.5,
                );

                if self.text[0].is_empty() == false {
                    context.draw2d.draw_text_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &asset.get_editor_font("OpenSans"),
                        context.button_text_size,
                        &self.text[0],
                        &if self.state == WidgetState::Disabled {
                            context.color_gray
                        } else {
                            context.color_white
                        },
                        &fill_color,
                        draw2d::TextAlignment::Center,
                    );
                } else if self.atom_widget_type == AtomWidgetType::TagsButton {
                    context.draw2d.draw_text_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &asset.get_editor_font("OpenSans"),
                        context.button_text_size,
                        &"Enter Tags".to_string(),
                        &context.color_gray,
                        &fill_color,
                        draw2d::TextAlignment::Center,
                    );
                }
            } else
            // Normal
            if self.atom_widget_type == AtomWidgetType::TileScaleButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.toolbar_button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let fill_color;
                if self.atom_widget_type != AtomWidgetType::CheckButton {
                    fill_color = if self.state != WidgetState::Clicked {
                        &context.color_black
                    } else {
                        &context.color_light_gray
                    };
                } else {
                    if self.state == WidgetState::Hover {
                        fill_color = &context.color_light_gray;
                    } else {
                        fill_color = if self.checked == false {
                            &context.color_black
                        } else {
                            &context.color_gray
                        };
                    }
                }

                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.button_rounding,
                    &if self.state == WidgetState::Disabled {
                        context.color_gray
                    } else {
                        context.color_light_gray
                    },
                    1.5,
                );

                let text = self.atom_data.value.to_integer().unwrap().to_string();

                if text.is_empty() == false {
                    context.draw2d.draw_text_rect(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &asset.get_editor_font("OpenSans"),
                        context.button_text_size,
                        &text,
                        &if self.state == WidgetState::Disabled {
                            context.color_gray
                        } else {
                            context.color_white
                        },
                        &fill_color,
                        draw2d::TextAlignment::Center,
                    );
                }
            } else if self.atom_widget_type == AtomWidgetType::MenuButton {
                if self.state != WidgetState::Clicked {
                    self.content_rect = (
                        self.rect.0 + 1,
                        self.rect.1 + (self.rect.3 - context.node_button_height) / 2,
                        self.rect.2 - 2,
                        context.node_button_height,
                    );

                    let fill_color = context.color_black;
                    let border_color = if self.state != WidgetState::Disabled {
                        context.color_light_gray
                    } else {
                        context.color_node_light_gray
                    };

                    context
                        .draw2d
                        .draw_rect(buffer_frame, &rect, rect.2, &context.color_black);
                    context.draw2d.draw_rounded_rect_with_border(
                        buffer_frame,
                        &rect,
                        rect.2,
                        &(self.content_rect.2 as f32, self.content_rect.3 as f32 - 1.0),
                        &fill_color,
                        &context.node_button_rounding,
                        &border_color,
                        1.5,
                    );

                    if self.text.len() > 0 {
                        context.draw2d.draw_text_rect(
                            buffer_frame,
                            &rect,
                            rect.2,
                            &asset.get_editor_font("OpenSans"),
                            context.button_text_size,
                            &self.text[self.curr_index],
                            &context.color_white,
                            &fill_color,
                            draw2d::TextAlignment::Center,
                        );
                    }

                    // Triangle
                    let color = if self.state == WidgetState::Hover && self.text.len() > 1 {
                        &context.color_light_gray
                    } else {
                        &context.color_gray
                    };

                    context.draw2d.blend_mask(
                        buffer_frame,
                        &(rect.2 - 25, 17, rect.2, rect.3),
                        rect.2,
                        &context.menu_triangle_mask[..],
                        &(10, 10),
                        &color,
                    );
                }
            } else if self.atom_widget_type == AtomWidgetType::GroupedList {
                self.content_rect = (self.rect.0, self.rect.1, self.rect.2, self.rect.3);

                let mut width = rect.2;
                if self.grouped_list_height > rect.3 as isize {
                    width -= SCROLLBAR_WIDTH;
                }

                let mut y = 2;
                for g_index in 0..self.groups.len() {
                    for i_index in 0..self.groups[g_index].items.len() {
                        let r = (rect.0, y, width, 32);

                        let mut rounding = context.button_rounding;

                        let color: [u8; 4];
                        let mut text_color: [u8; 4] = context.color_white;

                        if self.state == WidgetState::Disabled {
                            color = context.draw2d.mix_color(
                                &self.groups[g_index].color,
                                &[128, 128, 128, 255],
                                0.5,
                            );
                            text_color = context.color_light_gray;
                        } else if g_index == self.curr_group_index
                            && i_index == self.curr_item_index
                        {
                            color = self.groups[g_index].selected_color;
                        } else {
                            color = self.groups[g_index].color;
                        }

                        if self.groups[g_index].items.len() > 1 {
                            if i_index == 0 {
                                rounding.0 = 0.0;
                                rounding.2 = 0.0;
                            } else if i_index == &self.groups[g_index].items.len() - 1 {
                                rounding.1 = 0.0;
                                rounding.3 = 0.0;
                            } else {
                                rounding = (0.0, 0.0, 0.0, 0.0);
                            }
                        }

                        context.draw2d.draw_rounded_rect(
                            buffer_frame,
                            &r,
                            rect.2,
                            &(width as f32, 32.0),
                            &color,
                            &rounding,
                        );
                        if self.centered_text == false {
                            context.draw2d.draw_text(
                                buffer_frame,
                                &(r.0 + 15, r.1 + 4),
                                rect.2,
                                &asset.get_editor_font("OpenSans"),
                                context.button_text_size,
                                &self.groups[g_index].items[i_index].text,
                                &text_color,
                                &color,
                            );
                        } else {
                            context.draw2d.draw_text_rect(
                                buffer_frame,
                                &r,
                                rect.2,
                                &asset.get_editor_font("OpenSans"),
                                context.button_text_size,
                                &self.groups[g_index].items[i_index].text,
                                &text_color,
                                &color,
                                draw2d::TextAlignment::Center,
                            );
                        }

                        self.groups[g_index].items[i_index].rect = r;
                        self.groups[g_index].items[i_index].rect.1 += self.rect.1;

                        y += 33;
                    }
                }
            } else if self.atom_widget_type == AtomWidgetType::LayerNumberRow {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let cell_size = rect.3;
                let mut spacing = rect.2 - self.text.len() * cell_size;
                spacing /= self.text.len() - 1;

                let mut x = rect.0;
                for index in 0..self.text.len() {
                    let r = (x, rect.1, cell_size, rect.3);

                    let border_color = if index != self.curr_index {
                        &context.color_light_gray
                    } else {
                        &context.color_white
                    };

                    let fill_color = if index != 3 {
                        &context.color_black
                    } else {
                        &context.color_orange
                    };

                    // if index == 3 {
                    //     fill_color = if index != self.curr_index { &context.color_orange } else { &context.color_light_orange };
                    // }

                    let mut drawn = false;

                    if let Some(mask) = &self.button_mask {
                        if let Some(t) = &mask[index] {
                            context.draw2d.draw_rounded_rect_with_border(
                                buffer_frame,
                                &r,
                                rect.2,
                                &((cell_size - 2) as f32, (cell_size - 2) as f32),
                                &context.color_black,
                                &(0.0, 0.0, 0.0, 0.0),
                                &border_color,
                                1.5,
                            );

                            drawn = true;

                            if let Some(map) = asset.get_map_of_id(t.tilemap) {
                                context.draw2d.draw_animated_tile(
                                    buffer_frame,
                                    &(r.0 + 2, r.1 + 2),
                                    map,
                                    rect.2,
                                    &(t.x_off as usize, t.y_off as usize),
                                    0,
                                    cell_size - 4,
                                );
                            }
                        }
                    }

                    if drawn == false {
                        context.draw2d.draw_rounded_rect_with_border(
                            buffer_frame,
                            &r,
                            rect.2,
                            &((cell_size - 2) as f32, (cell_size - 2) as f32),
                            &fill_color,
                            &(0.0, 0.0, 0.0, 0.0),
                            border_color,
                            1.5,
                        );

                        context.draw2d.draw_text_rect(
                            buffer_frame,
                            &r,
                            rect.2,
                            &asset.get_editor_font("OpenSans"),
                            context.button_text_size,
                            &self.text[index],
                            &context.color_white,
                            &fill_color,
                            draw2d::TextAlignment::Center,
                        );
                    }

                    x += cell_size + spacing;
                }
            } else if self.atom_widget_type == AtomWidgetType::SliderButton {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let mut border_color = context.color_light_gray;
                if let Some(custom_color) = self.custom_color {
                    border_color = custom_color;
                }

                let fill_color = &context.color_black; //if self.state == WidgetState::Normal { &context.color_black } else { &context.color_light_gray };
                context.draw2d.draw_rounded_rect_with_border(
                    buffer_frame,
                    &rect,
                    rect.2,
                    &(self.content_rect.2 as f32, self.content_rect.3 as f32),
                    &fill_color,
                    &context.button_rounding,
                    &border_color,
                    1.5,
                );

                if self.text.is_empty() == false {
                    context.draw2d.draw_text_rect(
                        buffer_frame,
                        &(rect.0 + 30, rect.1, rect.2 - 60, rect.3),
                        rect.2,
                        &asset.get_editor_font("OpenSans"),
                        context.button_text_size,
                        &self.text[self.curr_index],
                        &context.color_white,
                        &fill_color,
                        draw2d::TextAlignment::Center,
                    );
                }

                // Right Arrow

                let arrow_y = rect.3 / 2 - 6;

                let left_color = if self.has_hover && self.text.len() > 1 {
                    &context.color_light_gray
                } else {
                    &context.color_gray
                };
                let right_color = if self.right_has_hover && self.text.len() > 1 {
                    &context.color_light_gray
                } else {
                    &context.color_gray
                };
                context.draw2d.blend_mask(
                    buffer_frame,
                    &(rect.2 + 14, arrow_y, 12, 14),
                    rect.2,
                    &context.left_arrow_mask_small[..],
                    &(8, 12),
                    &left_color,
                );
                context.draw2d.blend_mask(
                    buffer_frame,
                    &(rect.2 - 22, arrow_y, 12, 14),
                    rect.2,
                    &context.right_arrow_mask_small[..],
                    &(8, 12),
                    &right_color,
                );
            } else if self.atom_widget_type == AtomWidgetType::IconRow {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let cell_size = rect.3;
                let mut spacing = rect.2 - self.text.len() * cell_size;
                if self.text.len() > 1 {
                    spacing /= self.text.len() - 1;
                } else {
                    spacing = 0;
                }

                let mut x = rect.0;
                for index in 0..self.text.len() {
                    let r = (x, rect.1, cell_size, cell_size);

                    if self.curr_index == index {
                        if let Some(icon) = context.icons.get(&self.text[index]) {
                            context.draw2d.scale_chunk(
                                buffer_frame,
                                &r,
                                rect.2,
                                &icon.0[..],
                                &(icon.1 as usize, icon.2 as usize),
                                1.0,
                            );
                        }
                    } else {
                        if let Some(icon) =
                            context.icons.get(&(self.text[index].clone() + "_inactive"))
                        {
                            context.draw2d.scale_chunk(
                                buffer_frame,
                                &r,
                                rect.2,
                                &icon.0[..],
                                &(icon.1 as usize, icon.2 as usize),
                                1.0,
                            );
                        } else {
                            if let Some(icon) = context.icons.get(&self.text[index]) {
                                context.draw2d.scale_chunk(
                                    buffer_frame,
                                    &r,
                                    rect.2,
                                    &icon.0[..],
                                    &(icon.1 as usize, icon.2 as usize),
                                    0.3,
                                );
                            }
                        }
                    }

                    x += cell_size + spacing;
                }
            } else if self.atom_widget_type == AtomWidgetType::CheckedIcon
                || self.atom_widget_type == AtomWidgetType::EnabledIcon
            {
                self.content_rect = (
                    self.rect.0 + 1,
                    self.rect.1 + (self.rect.3 - context.button_height) / 2,
                    self.rect.2 - 2,
                    context.button_height,
                );

                let cell_size = rect.3;

                let x = rect.0;
                let r = (x, rect.1, cell_size, cell_size);

                if self.checked {
                    if let Some(icon) = context.icons.get(&self.text[0]) {
                        context.draw2d.scale_chunk(
                            buffer_frame,
                            &r,
                            rect.2,
                            &icon.0[..],
                            &(icon.1 as usize, icon.2 as usize),
                            0.9,
                        );
                    }
                } else {
                    if let Some(icon) = context.icons.get(&(self.text[0].clone() + "_inactive")) {
                        context.draw2d.scale_chunk(
                            buffer_frame,
                            &r,
                            rect.2,
                            &icon.0[..],
                            &(icon.1 as usize, icon.2 as usize),
                            1.0,
                        );
                    } else {
                        if let Some(icon) = context.icons.get(&self.text[0]) {
                            context.draw2d.scale_chunk(
                                buffer_frame,
                                &r,
                                rect.2,
                                &icon.0[..],
                                &(icon.1 as usize, icon.2 as usize),
                                0.3,
                            );
                        }
                    }
                }
            }
        }
        self.dirty = false;
        self.dirty_scroll = false;
        self.scroll_thumb_rect = None;

        if self.atom_widget_type != AtomWidgetType::GroupedList {
            context
                .draw2d
                .blend_slice(frame, buffer_frame, &self.rect, stride);
        } else {
            context.draw2d.blend_slice_offset(
                frame,
                buffer_frame,
                &self.rect,
                self.scroll_offset as usize,
                stride,
            );

            if self.grouped_list_height > rect.3 as isize {
                // Draw scrollbar
                let height = rect.3;

                let total_height = self.grouped_list_height as usize;

                let mut sbr = (
                    self.rect.0 + self.rect.2 - SCROLLBAR_WIDTH,
                    self.rect.1,
                    SCROLLBAR_WIDTH,
                    rect.3,
                );
                context
                    .draw2d
                    .draw_rect(frame, &sbr, stride, &context.color_black);

                let ratio = height as f32 / total_height as f32;
                let theight = (height as f32 * ratio) as usize;

                let offset = (self.scroll_offset as f32 * ratio) as usize;

                sbr.1 += offset;
                sbr.3 = theight;

                if offset + theight > height {
                    sbr.3 -= (offset + theight) - height;
                }

                if self.scroll_click.is_some() {
                    context
                        .draw2d
                        .draw_rect(frame, &sbr, stride, &[60, 60, 60, 255]);
                } else {
                    context
                        .draw2d
                        .draw_rect(frame, &sbr, stride, &context.color_node_dark_gray);
                }
                self.scroll_thumb_rect = Some(sbr);
            }
        }
    }

    // Draw overlay widgets which gets rendered on the whole screen, like open menus etc
    pub fn draw_overlay(
        &mut self,
        frame: &mut [u8],
        _rect: &(usize, usize, usize, usize),
        _anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) {
        //println!("{:?} {:?}",  self.atom_widget_type, self.state );
        if self.atom_widget_type == AtomWidgetType::ToolBarMenuButton
            && self.state == WidgetState::Clicked
        {
            // Draw Open Menu
            self.content_rect = (
                self.rect.0 + self.emb_offset.0 as usize,
                self.rect.1
                    + self.emb_offset.1 as usize
                    + (self.rect.3 - context.toolbar_button_height) / 2,
                self.rect.2,
                context.toolbar_button_height * self.text.len(),
            );

            context.draw2d.draw_rounded_rect_with_border(
                frame,
                &self.content_rect,
                context.width,
                &(
                    self.content_rect.2 as f32 - 1.0,
                    self.content_rect.3 as f32 - 1.0,
                ),
                &context.color_black,
                &context.toolbar_button_rounding,
                &context.color_light_gray,
                1.5,
            );

            let mut r = self.content_rect.clone();
            r.3 = context.toolbar_button_height;
            for (index, text) in self.text.iter().enumerate() {
                let mut fill_color = &context.color_black;

                if let Some(selection) = self.new_selection {
                    if index == selection {
                        fill_color = &context.color_gray;
                        let mut rounding = (0.0, 0.0, 0.0, 0.0);

                        if index == 0 {
                            rounding.1 = context.toolbar_button_rounding.1;
                            rounding.3 = context.toolbar_button_rounding.3;
                        } else if index == self.text.len() - 1 {
                            rounding.0 = context.toolbar_button_rounding.0;
                            rounding.2 = context.toolbar_button_rounding.2;
                        }

                        context.draw2d.draw_rounded_rect_with_border(
                            frame,
                            &r,
                            context.width,
                            &(r.2 as f32 - 1.0, r.3 as f32 - 1.0),
                            &fill_color,
                            &rounding,
                            &context.color_light_gray,
                            1.5,
                        );
                    }
                }

                context.draw2d.draw_text_rect(
                    frame,
                    &r,
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    context.toolbar_button_text_size,
                    &text,
                    &context.color_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
                r.1 += context.toolbar_button_height;
            }
        } else if (self.atom_widget_type == AtomWidgetType::NodeMenuButton
            || self.atom_widget_type == AtomWidgetType::SmallMenuButton
            || self.atom_widget_type == AtomWidgetType::MenuButton)
            && self.state == WidgetState::Clicked
        {
            // Draw Open Menu
            self.content_rect = (
                self.rect.0 + self.emb_offset.0 as usize,
                self.rect.1
                    + self.emb_offset.1 as usize
                    + (self.rect.3 - context.node_button_height) / 2,
                self.rect.2,
                context.node_button_height * self.text.len(),
            );

            if self.content_rect.0 + self.content_rect.2 >= context.width {
                self.content_rect.0 -= (self.content_rect.0 + self.content_rect.2) - context.width;
            }

            if self.content_rect.1 + self.content_rect.3 >= context.height {
                self.content_rect.1 -= (self.content_rect.1 + self.content_rect.3) - context.height;
            }

            context.draw2d.draw_rounded_rect_with_border(
                frame,
                &self.content_rect,
                context.width,
                &(
                    self.content_rect.2 as f32 - 1.0,
                    self.content_rect.3 as f32 - 1.0,
                ),
                &context.color_black,
                &context.node_button_rounding,
                &context.color_light_gray,
                1.5,
            );

            let mut r = self.content_rect.clone();
            r.3 = context.node_button_height;
            for (index, text) in self.text.iter().enumerate() {
                let mut fill_color = &context.color_black;

                if let Some(selection) = self.new_selection {
                    if index == selection {
                        fill_color = &context.color_gray;
                        let mut rounding = (0.0, 0.0, 0.0, 0.0);

                        if index == 0 {
                            rounding.1 = context.node_button_rounding.1;
                            rounding.3 = context.node_button_rounding.3;
                        } else if index == self.text.len() - 1 {
                            rounding.0 = context.node_button_rounding.0;
                            rounding.2 = context.node_button_rounding.2;
                        }

                        context.draw2d.draw_rounded_rect_with_border(
                            frame,
                            &r,
                            context.width,
                            &(r.2 as f32 - 1.0, r.3 as f32 - 1.0),
                            &fill_color,
                            &rounding,
                            &context.color_light_gray,
                            1.5,
                        );
                    }
                }

                context.draw2d.draw_text_rect(
                    frame,
                    &r,
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &text,
                    &context.color_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
                r.1 += context.node_button_height;
            }
        } else if (self.atom_widget_type == AtomWidgetType::NodeMenu)
            && self.state == WidgetState::Clicked
        {
            // Draw Open Menu
            self.content_rect = (
                self.rect.0 + self.emb_offset.0 as usize,
                self.rect.1 + self.emb_offset.1 as usize + self.rect.3 + 2,
                140,
                context.node_button_height * self.text.len(),
            );

            if self.content_rect.0 + self.content_rect.2 >= context.width {
                self.content_rect.0 -= (self.content_rect.0 + self.content_rect.2) - context.width;
            }

            if self.content_rect.1 + self.content_rect.3 >= context.height {
                self.content_rect.1 -= (self.content_rect.1 + self.content_rect.3) - context.height;
            }

            context.draw2d.draw_rounded_rect_with_border(
                frame,
                &self.content_rect,
                context.width,
                &(
                    self.content_rect.2 as f32 - 1.0,
                    self.content_rect.3 as f32 - 1.0,
                ),
                &&context.color_black,
                &context.node_button_rounding,
                &context.color_light_gray,
                1.5,
            );

            let mut r = self.content_rect.clone();
            r.3 = context.node_button_height;
            for (index, text) in self.text.iter().enumerate() {
                let mut fill_color = &context.color_black;

                if let Some(selection) = self.new_selection {
                    if index == selection {
                        fill_color = &context.color_gray;
                        let mut rounding = (0.0, 0.0, 0.0, 0.0);

                        if index == 0 {
                            rounding.1 = context.node_button_rounding.1;
                            rounding.3 = context.node_button_rounding.3;
                        } else if index == self.text.len() - 1 {
                            rounding.0 = context.node_button_rounding.0;
                            rounding.2 = context.node_button_rounding.2;
                        }

                        context.draw2d.draw_rounded_rect_with_border(
                            frame,
                            &r,
                            context.width,
                            &(r.2 as f32 - 1.0, r.3 as f32 - 1.0),
                            &fill_color,
                            &rounding,
                            &context.color_light_gray,
                            1.5,
                        );
                    }
                }

                context.draw2d.draw_text_rect(
                    frame,
                    &r,
                    context.width,
                    &asset.get_editor_font("OpenSans"),
                    context.node_button_text_size,
                    &text,
                    &context.color_white,
                    &fill_color,
                    draw2d::TextAlignment::Center,
                );
                r.1 += context.node_button_height;
            }
        }
    }

    pub fn mouse_down(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        if self.state == WidgetState::Disabled {
            return false;
        }

        self.scroll_click = None;
        if let Some(scroll_rect) = &self.scroll_thumb_rect {
            if context.contains_pos_for(pos, *scroll_rect) {
                self.scroll_click = Some(pos);
                self.scroll_offset_start = self.scroll_offset;
                return true;
            }
        }

        if self.contains_pos(pos) {
            if self.atom_widget_type == AtomWidgetType::ToolBarButton
                || self.atom_widget_type == AtomWidgetType::Button
                || self.atom_widget_type == AtomWidgetType::TagsButton
                || self.atom_widget_type == AtomWidgetType::LargeButton
                || self.atom_widget_type == AtomWidgetType::NodeNumberButton
                || self.atom_widget_type == AtomWidgetType::NodeTimeButton
                || self.atom_widget_type == AtomWidgetType::NodeSize2DButton
                || self.atom_widget_type == AtomWidgetType::NodeExpressionButton
                || self.atom_widget_type == AtomWidgetType::NodeExpressionValueButton
                || self.atom_widget_type == AtomWidgetType::NodeScriptButton
                || self.atom_widget_type == AtomWidgetType::NodeTextButton
                || self.atom_widget_type == AtomWidgetType::NodeCharTileButton
                || self.atom_widget_type == AtomWidgetType::NodeEnvTileButton
                || self.atom_widget_type == AtomWidgetType::NodeIconTileButton
                || self.atom_widget_type == AtomWidgetType::NodeGridSizeButton
                || self.atom_widget_type == AtomWidgetType::NodeScreenButton
                || self.atom_widget_type == AtomWidgetType::NodeItemSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeSpellSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeEffectTileButton
                || self.atom_widget_type == AtomWidgetType::NodeCharacterSettingsButton
                || self.atom_widget_type == AtomWidgetType::NodeRevealScriptsButton
            {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                self.dirty = true;
                return true;
            } else if self.atom_widget_type == AtomWidgetType::TileScaleButton {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                self.dirty = true;
                self.tile_scale();
                return true;
            } else if self.atom_widget_type == AtomWidgetType::ToolBarCheckButton
                || self.atom_widget_type == AtomWidgetType::CheckButton
                || self.atom_widget_type == AtomWidgetType::CheckedIcon
            {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                self.dirty = true;
                self.checked = !self.checked;
                return true;
            } else if self.atom_widget_type == AtomWidgetType::EnabledIcon {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                self.dirty = true;
                self.checked = true;
                return true;
            } else if self.atom_widget_type == AtomWidgetType::ToolBarMenuButton
                || self.atom_widget_type == AtomWidgetType::NodeMenuButton
                || self.atom_widget_type == AtomWidgetType::SmallMenuButton
                || self.atom_widget_type == AtomWidgetType::MenuButton
                || self.atom_widget_type == AtomWidgetType::NodeMenu
            {
                if self.text.len() > 1 {
                    self.clicked = true;
                    self.state = WidgetState::Clicked;
                    self.dirty = true;
                    self.new_selection = None; //Some(0);
                    return true;
                }
            } else if self.atom_widget_type == AtomWidgetType::LayerNumberRow
                || self.atom_widget_type == AtomWidgetType::IconRow
            {
                let rect = self.rect;

                let cell_size = rect.3;
                let mut spacing = rect.2 - self.text.len() * cell_size;
                spacing /= self.text.len() - 1;

                let mut x = rect.0;
                for index in 0..self.text.len() {
                    let r = (x, rect.1, cell_size, rect.3);

                    if context.contains_pos_for(pos, r) {
                        if index != self.curr_index {
                            self.clicked = true;
                            self.state = WidgetState::Clicked;
                            self.dirty = true;
                            self.curr_index = index;
                            return true;
                        }
                    }
                    x += cell_size + spacing;
                }
            } else if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton
                || self.atom_widget_type == AtomWidgetType::NodeSliderButton
                || self.atom_widget_type == AtomWidgetType::SliderButton
            {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                return true;
            } else if self.atom_widget_type == AtomWidgetType::NodeIntSlider {
                let min = self.atom_data.data.1;
                let max = self.atom_data.data.2;
                //let step = self.atom_data.data.3;

                if pos.0 >= self.content_rect.0 {
                    let offset = (pos.0 - self.content_rect.0) as f32;

                    let pp = (max - min) / self.content_rect.2 as f32;
                    let v = (min + offset * pp).round().clamp(min, max);

                    self.atom_data.data.0 = v;

                    self.dirty = true;
                    self.state = WidgetState::Clicked;
                    self.new_selection = Some(v as usize);
                }

                return true;
            } else if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {
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
            } else if self.atom_widget_type == AtomWidgetType::GroupedList {
                let mut p = pos.clone();
                p.1 += self.scroll_offset as usize;
                for g_index in 0..self.groups.len() {
                    for i_index in 0..self.groups[g_index].items.len() {
                        if self.contains_pos_for(p, self.groups[g_index].items[i_index].rect) {
                            self.curr_group_index = g_index;
                            self.curr_item_index = i_index;
                            self.dirty = true;
                            self.clicked = true;

                            let mouse_offset = (
                                pos.0 - self.groups[g_index].items[i_index].rect.0,
                                p.1 - self.groups[g_index].items[i_index].rect.1,
                            );

                            if self.drag_enabled {
                                self.drag_context = Some(ScreenDragContext {
                                    text: self.groups[g_index].items[i_index].text.clone(),
                                    color: self.groups[g_index].color.clone(),
                                    offset: (mouse_offset.0 as isize, mouse_offset.1 as isize),
                                    buffer: None,
                                });
                            }

                            return true;
                        }
                    }
                }
            } else if self.atom_widget_type == AtomWidgetType::NodePositionButton {
                self.clicked = true;
                self.state = WidgetState::Clicked;
                self.dirty = true;
                /*
                if context.active_position_id == self.behavior_id {
                    context.active_position_id = None;
                } else {
                    context.active_position_id = self.behavior_id.clone();
                    if self.atom_data.data.0 >= 0.0 {
                        context.jump_to_position = Some((self.atom_data.data.0 as usize, self.atom_data.data.1 as isize, self.atom_data.data.2 as isize));
                    }
                }*/
                return true;
            } else if self.atom_widget_type == AtomWidgetType::NodePropertyLog {
                let x = pos.0 - self.rect.0;
                let y = pos.1 - self.rect.1;
                if y <= 20 {
                    self.dirty = true;
                    if x < 24 {
                        self.curr_index = 0;
                    } else if x < 24 * 2 {
                        self.curr_index = 1;
                    } else if x < 24 * 3 {
                        self.curr_index = 2;
                    } else if x < 24 * 4 {
                        self.curr_index = 3;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn mouse_up(
        &mut self,
        _pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        self.drag_context = None;
        self.scroll_thumb_rect = None;

        if self.scroll_click.is_some() {
            self.scroll_click = None;
            return true;
        }

        if self.clicked || self.state == WidgetState::Clicked {
            self.clicked = false;
            self.dirty = true;

            if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton
                || self.atom_widget_type == AtomWidgetType::SliderButton
            {
                if self.right_has_hover {
                    if self.text.len() > 1 {
                        self.curr_index += 1;
                        self.curr_index %= self.text.len();
                        self.new_selection = Some(self.curr_index);
                    }
                } else if self.has_hover {
                    if self.text.len() > 1 {
                        if self.curr_index > 0 {
                            self.curr_index -= 1;
                        } else {
                            self.curr_index = self.text.len() - 1;
                        }
                        self.new_selection = Some(self.curr_index);
                    }
                }
                self.atom_data.data.0 = self.curr_index as f32;
            } else if self.atom_widget_type == AtomWidgetType::NodeScreenButton {
                context.switch_editor_state = Some(crate::editor::EditorState::ScreenDetail);
                context.code_editor_mode = CodeEditorMode::Rhai;
                if let Some(id) = &self.behavior_id {
                    context.code_editor_node_behavior_id = id.clone();
                    //     context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeNumberButton {
                context.dialog_state = DialogState::Opening;
                context.dialog_height = 0;
                context.target_fps = 60;
                context.dialog_entry = DialogEntry::NodeNumber;
                context.dialog_node_behavior_id = self.behavior_id.clone().unwrap();
                context.dialog_value = self.atom_data.value.clone();
            } else if self.atom_widget_type == AtomWidgetType::NodeTimeButton {
                context.dialog_state = DialogState::Opening;
                context.dialog_height = 0;
                context.target_fps = 60;
                context.dialog_entry = DialogEntry::NodeTime;
                context.dialog_node_behavior_id = self.behavior_id.clone().unwrap();
                context.dialog_value = self.atom_data.value.clone();
            } else if self.atom_widget_type == AtomWidgetType::NodeSize2DButton {
                context.dialog_state = DialogState::Opening;
                context.dialog_height = 0;
                context.target_fps = 60;
                context.dialog_entry = DialogEntry::NodeSize2D;
                context.dialog_node_behavior_id = self.behavior_id.clone().unwrap();
                context.dialog_node_behavior_value = self.atom_data.data.clone();
            } else if self.atom_widget_type == AtomWidgetType::NodeExpressionButton {
                context.code_editor_mode = CodeEditorMode::Rhai;
                if let Some(id) = &self.behavior_id {
                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeExpressionValueButton {
                context.code_editor_mode = CodeEditorMode::Rhai;
                if let Some(id) = &self.behavior_id {
                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeScriptButton {
                context.code_editor_mode = CodeEditorMode::Rhai;
                if let Some(id) = &self.behavior_id {
                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeTextButton {
                context.code_editor_mode = CodeEditorMode::Text;
                if let Some(id) = &self.behavior_id {
                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeGridSizeButton {
                context.dialog_entry = DialogEntry::NodeGridSize;
                context.open_dialog(
                    self.behavior_id.clone().unwrap(),
                    self.atom_data.value.clone(),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeCharTileButton {
                context.dialog_entry = DialogEntry::NodeTile;
                context.dialog_tile_usage = vec![TileUsage::Character, TileUsage::UtilityChar];
                context.open_dialog(
                    self.behavior_id.clone().unwrap(),
                    self.atom_data.value.clone(),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeIconTileButton {
                context.dialog_entry = DialogEntry::NodeTile;
                context.dialog_tile_usage = vec![TileUsage::Icon];
                context.open_dialog(
                    self.behavior_id.clone().unwrap(),
                    self.atom_data.value.clone(),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeEffectTileButton {
                context.dialog_entry = DialogEntry::NodeTile;
                context.dialog_tile_usage = vec![TileUsage::Effect];
                context.open_dialog(
                    self.behavior_id.clone().unwrap(),
                    self.atom_data.value.clone(),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeEnvTileButton {
                context.dialog_state = DialogState::Opening;
                context.dialog_height = 0;
                context.target_fps = 60;
                context.dialog_entry = DialogEntry::NodeTile;
                context.dialog_node_behavior_id = self.behavior_id.clone().unwrap();
                context.dialog_node_behavior_value = self.atom_data.data.clone();
                context.dialog_tile_usage = vec![
                    TileUsage::Environment,
                    TileUsage::EnvRoad,
                    TileUsage::EnvBlocking,
                    TileUsage::Water,
                ];
            } else if self.atom_widget_type == AtomWidgetType::TagsButton {
                context.dialog_entry = DialogEntry::Tags;
                if self.behavior_id.is_none() {
                    self.behavior_id = Some(context.create_property_id("tags"));
                }
                context.open_dialog(
                    self.behavior_id.clone().unwrap(),
                    Value::String(self.text[0].clone()),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodePositionButton {
                context.open_position_dialog(
                    self.behavior_id.clone().unwrap(),
                    self.atom_data.value.clone(),
                );
            } else if self.atom_widget_type == AtomWidgetType::NodeItemSettingsButton {
                context.code_editor_mode = CodeEditorMode::Settings;
                if let Some(id) = &self.behavior_id {
                    let mut sink = PropertySink::new();
                    sink.load_from_string(self.atom_data.value.to_string_value());
                    update_item_sink(&mut sink);
                    let str = sink.to_string(generate_item_sink_descriptions());

                    self.atom_data.value = Value::String(str);

                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeSpellSettingsButton {
                context.code_editor_mode = CodeEditorMode::Settings;
                if let Some(id) = &self.behavior_id {
                    let mut sink = PropertySink::new();
                    sink.load_from_string(self.atom_data.value.to_string_value());
                    update_spell_sink(&mut sink);
                    let str = sink.to_string(generate_spell_sink_descriptions());

                    self.atom_data.value = Value::String(str);

                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeCharacterSettingsButton {
                context.code_editor_mode = CodeEditorMode::Settings;
                if let Some(id) = &self.behavior_id {
                    let mut sink = PropertySink::new();
                    sink.load_from_string(self.atom_data.value.to_string_value());
                    update_behavior_sink(&mut sink);
                    let str = sink.to_string(generate_behavior_sink_descriptions());

                    self.atom_data.value = Value::String(str);

                    context.open_code_editor(id.clone(), self.atom_data.value.clone(), true);
                }
            } else if self.atom_widget_type == AtomWidgetType::NodeRevealScriptsButton {
                _ = open::that(context.curr_project_path.join("game").join("scripts"));
            }

            if self.state == WidgetState::Clicked {
                self.state = WidgetState::Normal;
            }

            if let Some(selection) = self.new_selection {
                self.curr_index = selection;
                self.atom_data.data.0 = self.curr_index as f32;
                self.atom_data.value = Value::Integer(selection as i32);
            }

            return true;
        }

        false
    }

    pub fn mouse_dragged(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        if let Some(scroll_click) = &self.scroll_click {
            let mut y: isize = pos.1 as isize - scroll_click.1 as isize;
            y = (y as f32 * self.grouped_list_height as f32 / self.rect.3 as f32) as isize;

            self.scroll_offset = self.scroll_offset_start + y;
            self.scroll_offset = self.scroll_offset.clamp(0, self.scroll_distance);

            return true;
        }

        if self.atom_widget_type == AtomWidgetType::ToolBarMenuButton
            && self.state == WidgetState::Clicked
        {
            self.new_selection = None;

            let mut r = self.content_rect.clone();
            r.0 -= self.emb_offset.0 as usize;
            r.3 = context.toolbar_button_height;
            for index in 0..self.text.len() {
                if context.contains_pos_for(pos, r) {
                    self.new_selection = Some(index);
                    return true;
                }
                r.1 += context.toolbar_button_height;
            }
            return true;
        } else if (self.atom_widget_type == AtomWidgetType::NodeMenuButton
            || self.atom_widget_type == AtomWidgetType::SmallMenuButton
            || self.atom_widget_type == AtomWidgetType::MenuButton
            || self.atom_widget_type == AtomWidgetType::NodeMenu)
            && self.state == WidgetState::Clicked
        {
            self.new_selection = None;

            let mut r = self.content_rect.clone();
            r.3 = context.node_button_height;
            for index in 0..self.text.len() {
                if context.contains_pos_for(pos, r) {
                    self.new_selection = Some(index);
                    return true;
                }
                r.1 += context.node_button_height;
            }
            return true;
        } else if self.atom_widget_type == AtomWidgetType::NodeIntSlider
            && self.state == WidgetState::Clicked
        {
            let min = self.atom_data.data.1;
            let max = self.atom_data.data.2;
            //let step = self.atom_data.data.3;

            if pos.0 >= self.content_rect.0 {
                let offset = (pos.0 - self.content_rect.0) as f32;

                let pp = (max - min) / self.content_rect.2 as f32;
                let v = (min + offset * pp).round().clamp(min, max);
                self.atom_data.data.0 = v;

                self.new_selection = Some(v as usize);
                self.dirty = true;
            } else {
                self.atom_data.data.0 = min;

                self.new_selection = Some(min as usize);
                self.dirty = true;
            }
            return true;
        }
        false
    }

    pub fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        if context.contains_pos_for(pos, self.rect) {
            if let Some(status_help_vector) = &self.status_help_vector {
                let mut help_index = 0;

                let rect = self.rect;

                let cell_size = rect.3;
                let mut spacing = rect.2 - self.text.len() * cell_size;
                spacing /= self.text.len() - 1;

                let mut x = rect.0;
                for index in 0..self.text.len() {
                    let r = (x, rect.1, cell_size, rect.3);

                    if context.contains_pos_for(pos, r) {
                        help_index = index;
                    }
                    x += cell_size + spacing;
                }

                context.status_help_text = Some(status_help_vector[help_index].1.clone());
            } else {
                context.status_help_text = self.status_help_text.clone();
            }
        }

        if self.atom_widget_type == AtomWidgetType::ToolBarButton
            || self.atom_widget_type == AtomWidgetType::ToolBarCheckButton
            || self.atom_widget_type == AtomWidgetType::CheckButton
            || self.atom_widget_type == AtomWidgetType::Button
            || self.atom_widget_type == AtomWidgetType::TagsButton
            || self.atom_widget_type == AtomWidgetType::LargeButton
            || self.atom_widget_type == AtomWidgetType::ToolBarMenuButton
            || self.atom_widget_type == AtomWidgetType::TileScaleButton
        {
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
        } else if self.atom_widget_type == AtomWidgetType::ToolBarSliderButton
            || self.atom_widget_type == AtomWidgetType::SliderButton
        {
            if self.contains_pos_for(pos, self.content_rect) {
                if self.state != WidgetState::Disabled {
                    if self.has_hover == false {
                        if pos.0 < self.rect.0 + 36 {
                            self.has_hover = true;
                            self.dirty = true;
                            return true;
                        }
                    } else {
                        if pos.0 > self.rect.0 + 36 {
                            self.has_hover = false;
                            self.dirty = true;
                            return true;
                        }
                    }

                    if self.right_has_hover == false {
                        if pos.0 > self.rect.0 + self.rect.2 - 36 {
                            self.right_has_hover = true;
                            self.dirty = true;
                            return true;
                        }
                    } else {
                        if pos.0 < self.rect.0 + self.rect.2 - 36 {
                            self.right_has_hover = false;
                            self.dirty = true;
                            return true;
                        }
                    }
                }
            } else {
                if self.state != WidgetState::Disabled {
                    if self.has_hover == true {
                        self.has_hover = false;
                        self.dirty = true;
                        return true;
                    }
                    if self.right_has_hover == true {
                        self.right_has_hover = false;
                        self.dirty = true;
                        return true;
                    }
                }
            }
        } else if self.atom_widget_type == AtomWidgetType::ToolBarSwitchButton {
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
                if self.has_hover {
                    self.has_hover = false;
                    self.dirty = true;
                    return true;
                }
                if self.right_has_hover {
                    self.right_has_hover = false;
                    self.dirty = true;
                    return true;
                }
            }
        }
        false
    }

    pub fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
    ) -> bool {
        self.scroll_offset -= delta.1;
        self.scroll_offset = self.scroll_offset.clamp(0, self.scroll_distance);
        self.dirty_scroll = true;
        true
    }

    pub fn add_group_list(&mut self, color: [u8; 4], selected_color: [u8; 4], items: Vec<String>) {
        let mut g_items: Vec<GroupItem> = vec![];
        for t in &items {
            let item = GroupItem {
                rect: (0, 0, 0, 0),
                text: t.to_string(),
            };
            g_items.push(item);
        }
        let group = GroupedList {
            color: color,
            selected_color,
            items: g_items,
        };
        self.groups.push(group);
    }

    fn contains_pos(&self, pos: (usize, usize)) -> bool {
        let rect = self.rect;

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3
        {
            true
        } else {
            false
        }
    }

    fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3
        {
            true
        } else {
            false
        }
    }

    /// Returns the height for this widget
    pub fn get_height(&self, context: &ScreenContext) -> usize {
        if self.atom_widget_type == AtomWidgetType::NodePositionButton {
            return context.node_button_height * 2;
        } else if self.atom_widget_type == AtomWidgetType::NodePropertyLog {
            return context.node_button_height * 8;
        }
        context.node_button_height
    }

    /// Height of a grouped list
    fn grouped_list_height(&self) -> usize {
        let mut height = 2;
        for g_index in 0..self.groups.len() {
            for _i_index in 0..self.groups[g_index].items.len() {
                height += 33
            }
        }
        height
    }

    /// Set the state of a switch button
    pub fn set_switch_button_state(&mut self, left_selected: bool, right_selected: bool) {
        self.selected = left_selected;
        self.right_selected = right_selected;
        self.dirty = true;
    }

    /// Sets the next slider button state
    pub fn next_slider_button_state(&mut self) {
        if self.text.len() > 1 {
            if self.curr_index < self.text.len() - 1 {
                self.curr_index += 1;
            } else {
                self.curr_index = 0;
            }
            self.dirty = true;
        }
    }

    pub fn tile_scale(&mut self) {
        if self.atom_widget_type == AtomWidgetType::TileScaleButton {
            let size = self.atom_data.value.to_integer().unwrap();
            match size {
                32 => {
                    self.atom_data.value = Value::Integer(48);
                }
                48 => {
                    self.atom_data.value = Value::Integer(64);
                }
                64 => {
                    self.atom_data.value = Value::Integer(2);
                }
                2 => {
                    self.atom_data.value = Value::Integer(4);
                }
                4 => {
                    self.atom_data.value = Value::Integer(8);
                }
                8 => {
                    self.atom_data.value = Value::Integer(16);
                }
                16 => {
                    self.atom_data.value = Value::Integer(24);
                }
                24 => {
                    self.atom_data.value = Value::Integer(32);
                }
                _ => {}
            }
            self.dirty = true;
        }
    }
}
