use crate::widget::*;
use crate::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct TileMapOptions {
    rect                    : (usize, usize, usize, usize),
    widgets                 : Vec<AtomWidget>,
}

impl Widget for TileMapOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let margin = 5_usize;

        let mut group_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_button("GroupedList".to_string()));

        group_list.add_group_list(context.color_yellow, context.color_light_yellow, vec!["Unused".to_string(), "Environment".to_string(), "Blocking".to_string(), "Character".to_string(), "Utility".to_string(), "Water".to_string(), "Effect".to_string()]);
        group_list.set_rect(rect, asset, context);
        widgets.push(group_list);

        // let mut unused_button = AtomWidget::new(vec!["Unused".to_string()], AtomWidgetType::CheckButton,
        //     AtomData::new_as_button("Unused".to_string()));

        // let mut environment_button = AtomWidget::new(vec!["Environment".to_string()], AtomWidgetType::CheckButton,
        // AtomData::new_as_button("Environment".to_string()));

        // widgets.push(unused_button);
        // widgets.push(environment_button);

        // let mut r = (rect.0 + 5, rect.1 + 10, rect.2 - 10, 35);

        // for w in &mut widgets {
        //     w.set_rect(r, asset, context);
        //     r.1 += 35;
        // }

        Self {
            rect,
            widgets             : widgets,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, anim_counter, asset, context);
        }

        if let Some(grid_pos) = context.curr_tile {
            context.draw2d.draw_animated_tile(frame, &(0, 300), asset.get_map_of_id(context.curr_tileset_index), context.width, &grid_pos, anim_counter, 100);
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {

                }
                return true;
            }
        }
        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;

        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }
        false
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize) {
        return &self.rect;
    }
}