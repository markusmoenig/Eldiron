use crate::prelude::*;

pub struct NodePreviewWidget {
    pub rect                    : (usize, usize, usize, usize),
    pub widgets                 : Vec<AtomWidget>,

    graph_type                  : BehaviorType,

    pub clicked                 : bool,

    pub id                      : usize,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,
    pub mask_buffer             : Vec<f32>,

    pub disabled                : bool,

    pub size                    : (usize, usize),

    pub clicked_id              : Option<(Uuid, Uuid, String)>,

    pub drag_size               : Option<(usize, usize)>,

    // For showing region
    pub preview_rect            : (usize, usize, usize, usize),
    pub region_offset           : (isize, isize),
    pub region_scroll_offset    : (isize, isize),

    pub curr_position           : Option<Position>,

    pub tile_size               : usize,

    pub graph_offset            : (isize, isize),

    pub clicked_region_id       : Option<(usize, isize, isize)>,

    pub curr_region_index       : usize,

    debug_update                : Option<GameUpdate>,
}

impl NodePreviewWidget {

    pub fn new(_context: &ScreenContext, graph_type: BehaviorType) -> Self {

        Self {
            rect                : (0,0,0,0),
            widgets             : vec![],

            graph_type,

            clicked             : false,

            id                  : 0,

            dirty               : true,
            buffer              : vec![],
            mask_buffer         : vec![],

            disabled            : false,

            size                : (310, 257),

            clicked_id          : None,

            drag_size           : None,

            preview_rect        : (0,0,0,0),
            region_offset       : (0,0),
            region_scroll_offset: (0,0),

            curr_position       : None,

            tile_size           : 32,

            graph_offset        : (0,0),

            clicked_region_id   : None,

            curr_region_index   : 0,

            debug_update        : None,
        }
    }

    /// Draw the node
    pub fn draw(&mut self, _frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        if self.buffer.len() != self.size.0 * self.size.1 * 4 {
            self.buffer = vec![0;self.size.0 * self.size.1 * 4];

            for w in &mut self.widgets {
                w.dirty = true;
            }
        }

        if self.mask_buffer.len() != self.size.0 * self.size.1 {
            self.mask_buffer = vec![0.0; self.size.0 * self.size.1];
            let r = (1, 0, self.size.0 - 1, self.size.1-1);
            context.draw2d.create_rounded_rect_mask(&mut self.mask_buffer[..], &r, self.size.0, &(0.0, 0.0, 20.0, 0.0));
        }

        let rect = (0, 0, self.size.0, self.size.1);

        // Go to this position
        if let Some(jump_to_position) = context.jump_to_position.clone() {
            self.dirty = true;
            self.curr_position = Some(jump_to_position);
            self.region_scroll_offset = (0, 0);
            context.jump_to_position = None;
        }

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }

            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f32, (rect.3 - 2) as f32), &context.color_black, &(0.0, 0.0, 20.0, 0.0), &context.color_gray, 1.5);
            context.draw2d.draw_rect(buffer_frame, &(2, 0, rect.2 - 4, 4), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(rect.2-2, 0, 2, rect.3 - 1), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(1, 1, 1, 1), stride, &[65, 65, 65, 255]);

            self.preview_rect.0 = 0;
            self.preview_rect.1 = 0;
            self.preview_rect.2 = rect.2;
            self.preview_rect.3 = rect.3;

            if self.graph_type == BehaviorType::Behaviors {
                if context.is_running {

                    if context.debug_render.is_none() {
                        context.debug_render = Some(GameRender::new(context.curr_project_path.clone(), context.player_id ));
                    }

                    if let Some(update) = &self.debug_update {
                        if let Some(render) = &mut context.debug_render {
                            render.process_update(update);
                            render.process_game_draw_2d(self.preview_rect, anim_counter, update, &mut Some(buffer_frame), stride, (0, 0));

                            let mut r = self.preview_rect.clone();
                            r.0 = 0;
                            r.1 = 0;
                            r.2 = 100;
                            r.3 = 20;

                            context.draw2d.draw_text_rect(buffer_frame, &r, stride, asset.get_editor_font("OpenSans"), 15.0, &update.date.to_time24(), &context.color_white, &context.color_black, crate::draw2d::TextAlignment::Center);
                        }
                    }
                } else {
                    if let Some(position) = context.data.get_behavior_default_position(context.data.behaviors_ids[context.curr_behavior_index]) {
                        if let Some(region) = context.data.regions.get(&position.region) {
                            context.draw2d.mask = Some(self.mask_buffer.clone());
                            context.draw2d.mask_size = self.size.clone();
                            self.region_offset = context.draw2d.draw_region_centered_with_behavior(buffer_frame, region, &self.preview_rect, &(position.x, position.y), &self.region_scroll_offset, stride, 32, 0, asset, context);
                            context.draw2d.mask = None;
                        }
                    }
                }
            } else {
                /*
                if context.is_running {
                    let source_frame = &context.data.game_frame;
                    let source_size = (context.data.game_screen_width, context.data.game_screen_height);

                    let mut fit_rect = self.region_rect.clone();

                    let ratio = (fit_rect.2 as f32 / source_size.0 as f32).min(fit_rect.3 as f32 / source_size.1 as f32);
                    let fit_size = ((source_size.0 as f32 * ratio) as usize, (source_size.1 as f32 * ratio) as usize);

                    if fit_size.0 < fit_rect.2 {
                        fit_rect.0 += (fit_rect.2 - fit_size.0) / 2;
                    }

                    if fit_size.1 < fit_rect.3 {
                        fit_rect.1 += (fit_rect.3 - fit_size.1) / 2;
                    }

                    fit_rect.2 = fit_size.0;
                    fit_rect.3 = fit_size.1;

                    context.draw2d.scale_chunk(buffer_frame, &fit_rect, stride, source_frame, &source_size);
                }*/
            }
            context.draw2d.blend_mask(buffer_frame, &(6, rect.3 - 23, rect.2, rect.3), rect.2, &context.preview_arc_mask[..], &(20, 20), &context.color_gray);
        }
        self.dirty = false;
    }

    /// Check if one of the atom widgets was clicked
    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_down(pos, asset, context) {
                self.dirty = true;
                self.clicked = true;
                self.clicked_id = atom_widget.behavior_id.clone();
                return true;
            }
        }

        // Test widget resizing
        if context.contains_pos_for(pos, (0, self.size.1 - 20, 30, 20)) {
            self.drag_size = Some(self.size.clone());
            context.target_fps = 60;
            return true;
        }

        // Test region map
        // if context.contains_pos_for(pos, self.preview_rect) {
        //     return true;
        // }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        self.clicked = false;
        self.clicked_id = None;
        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_up(pos, asset, context) {
                self.dirty = true;
                self.clicked = false;

                // if atom_widget.atom_data.text == "Region" {
                //     self.curr_region_index = atom_widget.curr_index;
                // }
                return true;
            }
        }

        if self.drag_size.is_some() {
            self.drag_size = None;
            context.target_fps = context.default_fps;
        }
        false
    }

    pub fn mouse_dragged(&mut self, _pos: (usize, usize), rel_pos: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {

        if let Some(drag_size) = &self.drag_size {
            let mut x: isize = drag_size.0 as isize + rel_pos.0;
            let mut y: isize =  drag_size.1 as isize + rel_pos.1;
            if x < 200 { x = 200; }
            if x > 600 { x = 600; }
            if y < 150 { y = 150; }
            if y > 600 { y = 600; }
            self.size = (x as usize, y as usize);
            self.dirty = true;
            return true;
        }

        false
    }

    pub fn mouse_wheel(&mut self, _delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        // self.region_scroll_offset.0 -= delta.0 / self.tile_size as isize;
        // self.region_scroll_offset.1 += delta.1 / self.tile_size as isize;
        // self.dirty = true;
        // true
        false
    }

    /// Apply an update when debugging. Previews only show behavior debug output.
    pub fn debug_update(&mut self, update: GameUpdate, _context: &mut ScreenContext) {
        self.debug_update = Some(update);
    }
}