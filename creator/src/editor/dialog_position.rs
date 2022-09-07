use crate::prelude::*;

pub struct DialogPositionWidget {
    pub rect                    : (usize, usize, usize, usize),

    pub widgets                 : Vec<AtomWidget>,

    dirty                       : bool,
    buffer                      : Vec<u8>,

    clicked_id                  : String,

    curr_area_id                : Uuid,

    region_rect                 : (usize, usize, usize, usize),
    region_offset               : (isize, isize),
    region_scroll_offset        : (isize, isize),

    pub new_value               : bool
}

impl DialogPositionWidget {

    pub fn new(_asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let region_menu = AtomWidget::new(context.data.regions_names.clone(), AtomWidgetType::ToolBarMenuButton,
        AtomData::new("Regions", Value::Empty()));
        widgets.push(region_menu);

        let areas_button = AtomWidget::new(vec!["Areas".to_string()], AtomWidgetType::ToolBarCheckButton,
        AtomData::new("Areas", Value::Empty()));
        widgets.push(areas_button);

        let ok_button = AtomWidget::new(vec!["Accept".to_string()], AtomWidgetType::ToolBarButton,
        AtomData::new("Accept", Value::Empty()));
        widgets.push(ok_button);

        Self {
            rect                    : (0, 0, 800, 600),

            widgets                 : widgets,

            dirty                   : true,
            buffer                  : vec![0],

            clicked_id              : "".to_string(),

            curr_area_id            : Uuid::new_v4(),

            region_rect             : (0,0,0,0),
            region_offset           : (0,0),
            region_scroll_offset    : (0,0),

            new_value               : false,
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let mut rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        // Animation
        if context.dialog_position_state == DialogState::Opening {

            self.rect.2 = 800;
            self.rect.3 = 600;

            context.dialog_height += 20;
            rect.3 = context.dialog_height;
            if context.dialog_height >= self.rect.3 {
                context.dialog_position_state = DialogState::Open;
                context.target_fps = context.default_fps;

                self.widgets[0].text = context.data.regions_names.clone();
                self.widgets[0].dirty = true;

                self.widgets[1].state = WidgetState::Normal;
                self.widgets[2].state = WidgetState::Normal;
                self.widgets[1].dirty = true;
                self.widgets[2].dirty = true;

                match context.dialog_value {
                    Value::Area(region_id, area_id) => {
                        self.curr_area_id = area_id;
                        for (index, id) in context.data.regions_ids.iter().enumerate() {
                            if region_id == *id {
                                self.widgets[0].curr_index = index;
                                break;
                            }
                        }
                    },
                    _ => {}
                }

                self.region_scroll_offset = (0, 0);
                self.new_value = false;
            }
            self.dirty = true;
        } else
        if context.dialog_position_state == DialogState::Closing {
            context.dialog_height -= 20;
            rect.3 = context.dialog_height;
            if context.dialog_height <= 20 {
                context.dialog_position_state = DialogState::Closed;
                context.target_fps = context.default_fps;
                return;
            }
            self.dirty = true;
        }

        if self.buffer.len() != rect.2 * rect.3 * 4 {
            self.buffer = vec![0;rect.2 * rect.3 * 4];
        }

        let area_mode = self.area_mode();
        let buffer_frame = &mut self.buffer[..];

        if self.dirty {

            buffer_frame.iter_mut().map(|x| *x = 0).count();

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, rect.2, &(rect.2 as f64 - 1.0, rect.3 as f64 - 1.0), &context.color_black, &(20.0, 0.0, 20.0, 0.0), &context.color_light_gray, 1.5);

            if context.dialog_position_state == DialogState::Open {

                let border_color : [u8; 4] = context.color_light_gray;

                let region_rect = (20, 60, rect.2 - 40, rect.3 - 150);

                let title_text_size = 30.0;

                if area_mode {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Select Area".to_string(), &context.color_white, &context.color_black);
                } else {
                    context.draw2d.draw_text(buffer_frame, &(40, 10), rect.2, &asset.get_editor_font("OpenSans"), title_text_size, &"Select Position".to_string(), &context.color_white, &context.color_black);
                }

                context.draw2d.draw_rounded_rect_with_border(buffer_frame, &region_rect, rect.2, &(region_rect.2 as f64 - 1.0, region_rect.3 as f64 - 1.0), &context.color_black, &(20.0, 20.0, 20.0, 20.0), &border_color, 1.5);

                if context.data.regions_ids.is_empty() == false {

                    let region_id = context.data.regions_ids[self.widgets[0].curr_index];
                    if let Some(region) = context.data.regions.get(&region_id) {

                        let mut position = (0,0);

                        match &context.dialog_value {
                            Value::Position(pos) => {
                                position = (pos.x as isize, pos.y as isize);
                            },
                            _ => {}
                        }

                        self.region_offset = context.draw2d.draw_region_centered_with_behavior(buffer_frame, region, &region_rect, &position, &self.region_scroll_offset, rect.2, 32, 0, asset, context);

                        // Draw areas
                        if area_mode {

                            let grid_size = 32;

                            let x_tiles = (region_rect.2 / grid_size) as isize;
                            let y_tiles = (region_rect.3 / grid_size) as isize;

                            let left_offset = (region_rect.2 % grid_size) / 2;
                            let top_offset = (region_rect.3 % grid_size) / 2;

                            for y in 0..y_tiles {
                                for x in 0..x_tiles {

                                    let rx = x + self.region_offset.0;
                                    let ry = y + self.region_offset.1;

                                    for area_index in 0..region.data.areas.len() {

                                        if region.data.areas[area_index].area.contains(&(rx, ry)) {
                                            let pos = (region_rect.0 + left_offset + (x as usize) * grid_size, region_rect.1 + top_offset + (y as usize) * grid_size);

                                            let mut c = context.color_white.clone();

                                            if self.curr_area_id == region.data.areas[area_index].id {
                                                c = context.color_red.clone();
                                                c[3] = 100;
                                            } else {
                                                c[3] = 50;
                                            }
                                            context.draw2d.blend_rect(buffer_frame, &(pos.0, pos.1, grid_size, grid_size), rect.2, &c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.region_rect = region_rect;

                // Draw Cancel / Accept buttons
                self.widgets[0].emb_offset.0 = self.rect.0 as isize;
                self.widgets[0].emb_offset.1 = 0;
                self.widgets[0].set_rect((20, rect.3 - 60, 800 - 320, 40), asset, context);
                self.widgets[1].set_rect((rect.2 - 280, rect.3 - 60, 120, 40), asset, context);
                self.widgets[2].set_rect((rect.2 - 140, rect.3 - 60, 120, 40), asset, context);

                for atom in &mut self.widgets {
                    atom.draw(buffer_frame, rect.2, anim_counter, asset, context);
                }
            }
        }
        self.dirty = false;
        context.draw2d.blend_slice(frame, buffer_frame, &(self.rect.0, self.rect.1, rect.2, rect.3), context.width);

        for atom in &mut self.widgets {
            atom.draw_overlay(frame, &self.rect, anim_counter, asset, context);
        }
    }

    pub fn key_down(&mut self, _char: Option<char>, key: Option<WidgetKey>, _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        //println!("dialog {:?}, {:?}", char, key);

        if let Some(key) = key {
            match key {
                WidgetKey::Escape => {
                    context.dialog_position_state = DialogState::Closing;
                    context.target_fps = 60;
                    context.dialog_accepted = false;

                    return  true;
                },
                WidgetKey::Return => {
                    context.dialog_position_state = DialogState::Closing;
                    context.target_fps = 60;
                    context.dialog_accepted = true;

                    context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_value.clone(), context.curr_graph_type);

                    self.new_value = true;

                    return  true;
                },
                _ => {}
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
                if atom.atom_data.id == "Regions" {
                    self.dirty = true;
                    self.clicked_id = atom.atom_data.id.clone();
                    return true;
                } else {
                    self.dirty = true;
                    self.clicked_id = atom.atom_data.id.clone();
                    return true;
                }
            }
        }

        // Test region map
        if context.contains_pos_for(pos, (self.region_rect.0 + self.rect.0, self.region_rect.1 + self.rect.1, self.region_rect.2, self.region_rect.3)) {

            let mut cpos = pos.clone();
            cpos.0 -= self.rect.0;
            cpos.1 -= self.rect.1;

            let region_tile_size = 32;

            let left_offset = (self.region_rect.2 % region_tile_size) / 2;
            let top_offset = (self.region_rect.3 % region_tile_size) / 2;

            let x = self.region_offset.0 + ((cpos.0 - self.region_rect.0 - left_offset) / region_tile_size) as isize;
            let y = self.region_offset.1 + ((cpos.1 - self.region_rect.1 - top_offset) / region_tile_size) as isize;

            self.dirty = true;

            if self.area_mode() == false {
                self.region_scroll_offset = (0, 0);

                let region_id = context.data.regions_ids[self.widgets[0].curr_index];
                context.dialog_value = Value::Position(Position::new(region_id, x, y));

                //context.dialog_node_behavior_value = (context.data.regions_ids[self.widgets[0].curr_index] as f64, x as f64, y as f64, -1.0, "".to_string());
            } else {
                let region_id = context.data.regions_ids[self.widgets[0].curr_index];
                if let Some(region) = context.data.regions.get(&region_id) {

                    for area_index in 0..region.data.areas.len() {
                        if region.data.areas[area_index].area.contains(&(x, y)) {
                            self.curr_area_id = region.data.areas[area_index].id;

                            let region_id = context.data.regions_ids[self.widgets[0].curr_index];
                            let area_id = region.data.areas[area_index].id;

                            context.dialog_value = Value::Area(region_id, area_id);
                            //let id = region.data.areas[area_index].id as f64;
                            //context.dialog_node_behavior_value = (context.data.regions_ids[self.widgets[0].curr_index] as f64, x as f64, y as f64, id, "".to_string());
                        }
                    }
                }
            }
            return true;
        }

        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if pos.0 < self.rect.0 || pos.1 < self.rect.1 { return false; }
        let local = (pos.0 - self.rect.0, pos.1 - self.rect.1);
        for atom in &mut self.widgets {
            if atom.mouse_up(local, asset, context) {
                self.dirty = true;

                if self.clicked_id == "Clear" {
                    context.dialog_position_state = DialogState::Closing;
                    context.target_fps = 60;
                    context.dialog_accepted = false;

                    context.dialog_node_behavior_value.1 = 100000.0;
                    context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_value.clone(), context.curr_graph_type);

                    self.new_value = true;
                } else
                if self.clicked_id == "Accept" {
                    context.dialog_position_state = DialogState::Closing;
                    context.target_fps = 60;
                    context.dialog_accepted = true;

                    context.data.set_behavior_id_value(context.dialog_node_behavior_id.clone(), context.dialog_value.clone(), context.curr_graph_type);

                    self.new_value = true;
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

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.region_scroll_offset.0 -= delta.0 / 8 as isize;
        self.region_scroll_offset.1 += delta.1 / 8 as isize;
        self.dirty = true;
        true
    }

    fn area_mode(&self) -> bool {
        self.widgets[1].checked
    }
}