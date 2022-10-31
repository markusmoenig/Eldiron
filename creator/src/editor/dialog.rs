use crate::prelude::*;

#[derive(PartialEq, Debug)]
pub enum DialogEntry {
    None,
    NodeNumber,
    NodeSize2D,
    NodeExpression,
    NodeExpressionValue,
    NodeScript,
    NodeText,
    NodeName,
    NodeTile,
    NewName,
    Tags,
    NodeGridSize,
    NewProjectName,
}

#[derive(PartialEq, Debug)]
pub enum DialogState {
    Closed,
    Open,
    Opening,
    Closing
}

pub struct DialogWidget {
    pub rect                    : (usize, usize, usize, usize),
    pub text                    : String,

    pub widgets                 : Vec<AtomWidget>,

    dirty                       : bool,
    buffer                      : Vec<u8>,

    clicked_id                  : String,

    tile_selector_widget        : TileSelectorWidget,
    large                       : bool,
}

impl DialogWidget {

    pub fn new(asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let cancel_button = AtomWidget::new(vec!["Cancel".to_string()], AtomWidgetType::ToolBarButton,
        AtomData::new("Cancel", Value::Empty()));
        widgets.push(cancel_button);

        let ok_button = AtomWidget::new(vec!["Accept".to_string()], AtomWidgetType::ToolBarButton,
        AtomData::new("Accept", Value::Empty()));
        widgets.push(ok_button);

        let tile_selector_widget = TileSelectorWidget::new(vec!(), (0,0,0,0), asset, &context);

        Self {
            rect                : (0, 0, 600, 200),
            text                : "".to_string(),

            widgets             : widgets,

            dirty               : true,
            buffer              : vec![0],

            clicked_id          : "".to_string(),

            tile_selector_widget,

            large               : false,
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        let mut rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        // Animation
        if context.dialog_state == DialogState::Opening {
            // Set the size based on the content

            self.rect.2 = 600;
            self.rect.3 = 200;
            self.large = false;

            if context.dialog_entry == DialogEntry::NodeTile {
                self.rect.2 = 800;
                self.rect.3 = 600;
                self.large = true;
            }

            context.dialog_height += 20;
            rect.3 = context.dialog_height;
            if context.dialog_height >= self.rect.3 {
                context.dialog_state = DialogState::Open;
                context.target_fps = context.default_fps;

                self.widgets[0].state = WidgetState::Normal;
                self.widgets[1].state = WidgetState::Normal;

                if context.dialog_entry == DialogEntry::NodeNumber  {
                    self.text = format!("{}", context.dialog_node_behavior_value.0);
                } else
                if context.dialog_entry == DialogEntry::NodeSize2D {
                    self.text = format!("{}", context.dialog_node_behavior_value.0);
                } else
                if context.dialog_entry == DialogEntry::NodeExpression || context.dialog_entry == DialogEntry::NodeExpressionValue || context.dialog_entry == DialogEntry::NodeScript || context.dialog_entry == DialogEntry::NodeText || context.dialog_entry == DialogEntry::NodeGridSize || context.dialog_entry == DialogEntry::NodeName {
                    self.text = context.dialog_value.to_string_value();
                } else
                if context.dialog_entry == DialogEntry::NodeTile {
                    self.tile_selector_widget.set_tile_type(context.dialog_tile_usage.clone(), None, None, &asset);
                    self.text = "".to_string();
                    self.tile_selector_widget.grid_size = 32;
                    self.tile_selector_widget.selected = context.dialog_value.to_tile_data();
                } else
                if context.dialog_entry == DialogEntry::NewName || context.dialog_entry == DialogEntry::Tags || context.dialog_entry == DialogEntry::NewProjectName {
                    self.text = context.dialog_value.to_string_value();
                } else {
                }
            }
            self.dirty = true;
        } else
        if context.dialog_state == DialogState::Closing {
            context.dialog_height -= 20;
            rect.3 = context.dialog_height;
            if context.dialog_height <= 20 {
                context.dialog_state = DialogState::Closed;
                context.target_fps = context.default_fps;
                return;
            }
            self.dirty = true;
        }

        if self.buffer.len() != rect.2 * rect.3 * 4 {
            self.buffer = vec![0;rect.2 * rect.3 * 4];
        }

        let buffer_frame = &mut self.buffer[..];

        if self.dirty {

            buffer_frame.iter_mut().map(|x| *x = 0).count();

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(rect.2 as f64 - 1.0, rect.3 as f64 - 1.0), &context.color_black, &(20.0, 0.0, 20.0, 0.0), &context.color_light_gray, 1.5);

            if context.dialog_state == DialogState::Open {

                let mut border_color : [u8; 4] = context.color_light_gray;

                let input_rect = (20, 60, rect.2 - 40, rect.3 - 150);

                let title_text_size = 30.0;

                if context.dialog_entry == DialogEntry::NodeNumber {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Number".to_string(), &context.color_white, &context.color_black);

                    if self.text.parse::<f64>().is_err() {
                        border_color = context.color_red;
                        self.widgets[1].state = WidgetState::Disabled;
                    } else
                    if self.widgets[1].state == WidgetState::Disabled {
                        self.widgets[1].state = WidgetState::Normal;
                    }
                } else
                if context.dialog_entry == DialogEntry::NodeSize2D {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Size".to_string(), &context.color_white, &context.color_black);

                    let mut valid = false;
                    let txt = self.text.split("x").collect::<Vec<&str>>();
                    if txt.len() == 2 {
                        if txt[0].parse::<f64>().is_ok() && txt[1].parse::<f64>().is_ok() {
                            valid = true;
                        }
                    }

                    if valid == false {
                        border_color = context.color_red;
                        self.widgets[1].state = WidgetState::Disabled;
                    } else
                    if self.widgets[1].state == WidgetState::Disabled {
                        self.widgets[1].state = WidgetState::Normal;
                    }
                } else
                if context.dialog_entry == DialogEntry::NodeExpression  {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Boolean Expression".to_string(), &context.color_white, &context.color_black);

                    //self.code_editor.draw(frame, input_rect, rect.2, asset.get_editor_font("OpenSans"), &context.draw2d);
                    let has_error = false;
                    /*
                    let behavior_id = context.dialog_node_behavior_id.0.clone();
                    if context.dialog_entry == DialogEntry::NodeExpression {
                        if server::gamedata::script::eval_bool_expression_behavior(self.text.as_str(), behavior_id, &mut context.data) == None {
                            has_error = true;
                        }
                    } else
                    if context.dialog_entry == DialogEntry::NodeScript {
                        if server::gamedata::script::eval_dynamic_script_behavior(self.text.as_str(), behavior_id, &mut context.data) == false {
                            has_error = true;
                        }
                    } else {
                        if server::gamedata::script::eval_number_expression_behavior(self.text.as_str(), behavior_id, &mut context.data) == None {
                            has_error = true;
                        }
                    }*/
                    if has_error {
                        border_color = context.color_red;
                        self.widgets[1].state = WidgetState::Disabled;
                    } else
                    if self.widgets[1].state == WidgetState::Disabled {
                        self.widgets[1].state = WidgetState::Normal;
                    }
                } else
                if context.dialog_entry == DialogEntry::NodeExpressionValue {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Number Expression".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NodeScript {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Script".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NodeText {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Text".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NodeGridSize {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Grid Size".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NewName {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Name".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NewProjectName {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"New Project".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::Tags {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Tags".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NodeName {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Node Name".to_string(), &context.color_white, &context.color_black);
                } else
                if context.dialog_entry == DialogEntry::NodeTile {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Select Tile".to_string(), &context.color_white, &context.color_black);
                    self.tile_selector_widget.rect = input_rect.clone();
                    self.tile_selector_widget.draw(buffer_frame, rect.2, anim_counter, asset, context);
                }

                if context.dialog_entry != DialogEntry::NodeTile {
                    context.draw2d.draw_rounded_rect_with_border(buffer_frame, &input_rect, rect.2, &(input_rect.2 as f64 - 1.0, input_rect.3 as f64 - 1.0), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &border_color, 1.5);
                }

                if !self.text.is_empty() {
                    context.draw2d.draw_text_rect(buffer_frame, &input_rect, rect.2, &asset.get_editor_font("OpenSans"), 30.0, &self.text, &context.color_white, &context.color_black, crate::draw2d::TextAlignment::Center);
                }

                // Draw Cancel / Accept buttons
                self.widgets[0].set_rect((rect.2 - 280, rect.3 - 60, 120, 40), asset, context);
                self.widgets[1].set_rect((rect.2 - 140, rect.3 - 60, 120, 40), asset, context);

                for atom in &mut self.widgets {
                    if context.dialog_entry == DialogEntry::NewProjectName && atom.text[0] == "Cancel" {
                        continue;
                    }
                    atom.draw(buffer_frame, rect.2, anim_counter, asset, context);
                }
            }
        }
        self.dirty = false;
        context.draw2d.blend_slice(frame, buffer_frame, &(self.rect.0, self.rect.1, rect.2, rect.3), context.width);
        if context.dialog_entry == DialogEntry::NodeTile {
            self.dirty = true;
        }
    }

    /// Accepts the given value (if correct)
    pub fn accept_value(&mut self, context: &mut ScreenContext) -> bool {

        if context.dialog_entry == DialogEntry::NodeNumber {
            let int_value = self.text.parse::<i64>();
            if int_value.is_ok() {
                context.dialog_node_behavior_value.0 = int_value.unwrap() as f64;
                // TODO context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_node_behavior_value.clone(), context.curr_graph_type);
                return true;
            }
            let float_value = self.text.parse::<f64>();
            if float_value.is_ok() {
                context.dialog_node_behavior_value.0 = float_value.unwrap();
                // TODO context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_node_behavior_value.clone(), context.curr_graph_type);
                return true;
            }
        } else
        if context.dialog_entry == DialogEntry::NodeExpression || context.dialog_entry == DialogEntry::NodeExpressionValue || context.dialog_entry == DialogEntry::NodeScript {

            let has_error = false;

            /*
            let behavior_id = context.dialog_node_behavior_id.0.clone();
            if context.dialog_entry == DialogEntry::NodeExpression {
                if server::gamedata::script::eval_bool_expression_behavior(self.text.as_str(), behavior_id, &mut context.data) == None {
                    has_error = true;
                }
            } else
            if context.dialog_entry == DialogEntry::NodeScript {

                if server::gamedata::script::eval_dynamic_script_behavior(self.text.as_str(), behavior_id, &mut context.data) == false {
                    has_error = true;
                }
            } else {
                if server::gamedata::script::eval_number_expression_behavior(self.text.as_str(), behavior_id, &mut context.data) == None {
                    has_error = true;
                }
            }
            */
            if has_error == false {
                context.dialog_node_behavior_value.4 = self.text.clone();
                // TODO context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_node_behavior_value.clone(), context.curr_graph_type);
                return true;
            }
        } else
        if context.dialog_entry == DialogEntry::NodeText || context.dialog_entry == DialogEntry::NodeName {
            context.dialog_node_behavior_value.4 = self.text.clone();
            // TODO context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_node_behavior_value.clone(), context.curr_graph_type);
            return true;
        } else
        if context.dialog_entry == DialogEntry::NodeGridSize {
            context.dialog_value = Value::String(self.text.clone());
            return true;
        } else
        if context.dialog_entry == DialogEntry::NewName || context.dialog_entry == DialogEntry::NewProjectName  {
            context.dialog_new_name = self.text.clone();
            return true;
        } else
        if context.dialog_entry == DialogEntry::Tags {
            context.dialog_value = Value::String(self.text.clone());
            return true;
        } else
        if context.dialog_entry == DialogEntry::NodeTile {
            if let Some(selected) = &self.tile_selector_widget.selected {
                context.dialog_value = Value::TileData(selected.clone());
                return true;
            }
        }
        false
    }

    pub fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        //println!("dialog {:?}, {:?}", char, key);

        if let Some(key) = key {
            match key {
                WidgetKey::Delete => {
                    self.text.pop();
                    self.dirty = true;
                    return  true;
                },
                WidgetKey::Escape => {
                    if context.dialog_entry != DialogEntry::NewProjectName {
                        context.dialog_state = DialogState::Closing;
                        context.target_fps = 60;
                        context.dialog_accepted = false;
                        return  true;
                    }
                },
                WidgetKey::Return => {
                    if self.accept_value(context) {
                        context.dialog_state = DialogState::Closing;
                        context.target_fps = 60;
                        context.dialog_accepted = true;
                        return  true;
                    }
                },
                _ => {}
            }
        }

        if let Some(c) = char {
            if c.is_ascii() && c.is_control() == false {
                self.text.push(c);
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        self.clicked_id = "".to_string();

        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_down(local, asset, context) {
                self.dirty = true;
                self.clicked_id = atom.atom_data.id.clone();
                return true;
            }
        }

        if context.dialog_entry == DialogEntry::NodeTile {
            if self.tile_selector_widget.mouse_down(local, asset, context) {
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_up(local, asset, context) {
                self.dirty = true;

                if self.clicked_id == "Cancel" {
                    if context.dialog_entry != DialogEntry::NewProjectName {
                        context.dialog_state = DialogState::Closing;
                        context.target_fps = 60;
                        context.dialog_accepted = false;
                    }
                } else
                if self.clicked_id == "Accept" {
                    if self.accept_value(context) {
                        context.dialog_state = DialogState::Closing;
                        context.target_fps = 60;
                        context.dialog_accepted = true;
                    }
                }

                return true;
            }
        }

        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_dragged(local, asset, context) {
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_hover(local, asset, context) {
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if context.dialog_entry == DialogEntry::NodeTile {
            if self.tile_selector_widget.mouse_wheel(delta, asset, context) {
                self.dirty = true;
                return true;
            }
        }

        false
    }
}