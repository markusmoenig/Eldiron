
use crate::widget::*;
use crate::Asset;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct ToolBar {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,
}

impl Widget for ToolBar {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut item_slider_button = AtomWidget::new(asset.tileset.maps_names.clone(), AtomWidgetType::ToolBarSliderButton, 
        AtomData::new_as_button("Game".to_string()));
        item_slider_button.set_rect((rect.0 + 10, rect.1, 200, rect.3), asset, context);
        widgets.push(item_slider_button);


        let mut game_button = AtomWidget::new(vec!["Game".to_string()], AtomWidgetType::ToolBarButton, 
            AtomData::new_as_button("Game".to_string()));
        game_button.set_rect((rect.0 + 220, rect.1, 100, rect.3), asset, context);
        widgets.push(game_button);

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
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[25, 25, 25, 255]);

        for atom in &mut self.widgets {
            atom.draw(frame, anim_counter, asset, context);
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
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