use crate::Asset;
use crate::atom:: { AtomWidget, AtomWidgetType, AtomData };
use crate::editor::ScreenContext;

pub struct NodePreviewWidget {
    pub rect                    : (usize, usize, usize, usize),
    pub widgets                 : Vec<AtomWidget>,

    pub clicked                 : bool,

    pub id                      : usize,

    pub dirty                   : bool,
    pub buffer                  : Vec<u8>,

    pub disabled                : bool,

    pub size                    : (usize, usize),

    pub clicked_id              : Option<(usize, usize, String)>,

    pub drag_size               : Option<(usize, usize)>,

    // For showing maps
    pub map_tile_size           : usize,
    pub map_rect                : (usize, usize, usize, usize),

    pub graph_offset            : (isize, isize),

    pub curr_area_index         : usize,
}

impl NodePreviewWidget {

    pub fn new(context: &ScreenContext) -> Self {

        let run_button = AtomWidget::new(vec!["Run Behavior".to_string()], AtomWidgetType::LargeButton,
        AtomData::new_as_int("run".to_string(), 0));

        let mut areas_button = AtomWidget::new(context.data.areas_names.clone(), AtomWidgetType::NodeMenuButton,
        AtomData::new_as_int("area".to_string(), 0));
        areas_button.atom_data.text = "Area".to_string();
        areas_button.curr_index = 0;

        Self {
            rect                : (0,0,0,0),
            widgets             : vec![run_button, areas_button],
            clicked             : false,

            id                  : 0,

            dirty               : true,
            buffer              : vec![],

            disabled            : false,

            size                : (300, 250),

            clicked_id          : None,

            drag_size           : None,

            map_tile_size       : 32,
            map_rect            : (0,0,0,0),

            graph_offset        : (0,0),

            curr_area_index     : 0,
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

        let rect = (0, 0, self.size.0, self.size.1);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = self.size.0;

            context.draw2d.draw_rounded_rect_with_border(buffer_frame, &rect, stride, &((rect.2 - 1) as f64, (rect.3 - 2) as f64), &context.color_black, &(0.0, 0.0, 20.0, 0.0), &context.color_gray, 1.5);
            context.draw2d.draw_rect(buffer_frame, &(2, 0, rect.2 - 4, 4), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(rect.2-2, 0, 2, rect.3 - 1), stride, &context.color_black);
            context.draw2d.draw_rect(buffer_frame, &(1, 1, 1, 1), stride, &[65, 65, 65, 255]);

            self.widgets[0].set_rect((20, 4, 140, 32), asset, context);
            self.widgets[0].draw(buffer_frame, stride, anim_counter, asset, context);

            self.widgets[1].set_rect((15, self.size.1 - 50, self.size.0 - 20, 25), asset, context);
            self.widgets[1].draw(buffer_frame, stride, anim_counter, asset, context);

            self.map_rect.0 = 10;
            self.map_rect.1 = 50;
            self.map_rect.2 = rect.2 - 20;
            self.map_rect.3 = rect.3 - 100;

            if let Some(area) = context.data.areas.get(&self.curr_area_index) {
                let offset = area.data.min_pos;

                context.draw2d.draw_area(buffer_frame, area, &self.map_rect, &offset, stride, 32, anim_counter, asset);
            }
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

        // Test dragging area
        if context.contains_pos_for(pos, (0, self.size.1 - 20, 20, 20)) {
            self.drag_size = Some(self.size.clone());
            context.target_fps = 60;
            return true;
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        self.clicked = false;
        self.clicked_id = None;
        for atom_widget in &mut self.widgets {
            if atom_widget.mouse_up(pos, asset, context) {
                self.dirty = true;
                self.clicked = false;

                if atom_widget.atom_data.text == "Area" {
                    self.curr_area_index = atom_widget.curr_index;
                }
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
}