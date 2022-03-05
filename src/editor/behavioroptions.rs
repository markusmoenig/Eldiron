use crate::widget::*;

use crate::atom::AtomData;
use crate::widget::context::ScreenDragContext;
use server::asset::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct BehaviorOptions {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,

    pub drag_context        : Option<ScreenDragContext>,
}

impl BehaviorOptions {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut tilemap_names = asset.tileset.maps_names.clone();
        tilemap_names.insert(0, "Tilemaps: All".to_string());

        // let mut tilemaps_slider_button = AtomWidget::new(tilemap_names, AtomWidgetType::ToolBarSliderButton,
        // AtomData::new_as_int("Tilemaps".to_string(), 0));
        // tilemaps_slider_button.set_rect((rect.0 + 10, rect.1 + 10, rect.2 - 10, 40), asset, context);
        // widgets.push(tilemaps_slider_button);

        let mut node_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("NodeList".to_string(), 0));
        node_list.drag_enabled = true;

        node_list.add_group_list(context.color_yellow, context.color_light_yellow, vec!["Behavior Tree".to_string(), "Blocking".to_string(), "Water".to_string() ]);

        node_list.set_rect(rect, asset, context);
        widgets.push(node_list);

        Self {
            rect,
            widgets,
            drag_context            : None
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if let Some(grid_pos) = context.curr_tile {
            context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, self.rect.1 + self.rect.3 - 140), asset.get_map_of_id(context.curr_tileset_index), context.width, &grid_pos, anim_counter, 100);

            context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 40, self.rect.2, 30), context.width, &asset.open_sans, 20.0, &format!("({},{})", grid_pos.0, grid_pos.1), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
        }
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {
                    if atom.atom_data.id == "NodeList" {

                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    pub fn mouse_dragged(&mut self, _pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if let Some(drag_context) = &self.widgets[0].drag_context {
            if context.drag_context == None {

                let mut buffer = [0; 180 * 32 * 4];

                context.draw2d.draw_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &drag_context.color.clone());
                context.draw2d.draw_text_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &asset.open_sans, context.toolbar_button_text_size, drag_context.text.as_str(), &context.color_white, &drag_context.color.clone(), draw2d::TextAlignment::Center);

                context.drag_context = Some(ScreenDragContext {
                    text    : drag_context.text.clone(),
                    color   : drag_context.color.clone(),
                    offset  : drag_context.offset.clone(),
                    buffer  : Some(buffer)
                });
                context.target_fps = 60;
            }
            self.widgets[0].drag_context = None;
        }
        false
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }
}