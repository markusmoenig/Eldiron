
use crate::atom::AtomData;
use crate::widget::*;
use server::asset::Asset;

use crate::widget::atom:: { AtomWidget, AtomWidgetType };
use crate::widget::context::ScreenContext;

pub struct ToolBar {
    rect                    : (usize, usize, usize, usize),
    pub widgets             : Vec<AtomWidget>,
}

impl Widget for ToolBar {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut item_button = AtomWidget::new(asset.tileset.maps_names.clone(), AtomWidgetType::ToolBarMenuButton,
        AtomData::new_as_int("Game".to_string(), 0));
        item_button.set_rect((rect.0 + 10, rect.1, 200, rect.3), asset, context);
        widgets.push(item_button);

        let mut tiles_button = AtomWidget::new(vec!["Tiles".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new_as_int("Tiles".to_string(), 0));
        tiles_button.set_rect((rect.0 + 220, rect.1, 130, rect.3), asset, context);
        tiles_button.selected = true;
        widgets.push(tiles_button);

        let mut areas_button = AtomWidget::new(vec!["Regions".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new_as_int("Regions".to_string(), 0));
        areas_button.set_rect((rect.0 + 360, rect.1, 155, rect.3), asset, context);
        widgets.push(areas_button);

        let mut behavior_button = AtomWidget::new(vec!["Behavior".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new_as_int("Areas".to_string(), 0));
        behavior_button.set_rect((rect.0 + 525, rect.1, 165, rect.3), asset, context);
        widgets.push(behavior_button);

        let mut systems_button = AtomWidget::new(vec!["Systems".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new_as_int("Systems".to_string(), 0));
        systems_button.set_rect((rect.0 + 700, rect.1, 165, rect.3), asset, context);
        widgets.push(systems_button);

        let mut items_button = AtomWidget::new(vec!["Items".to_string()], AtomWidgetType::ToolBarSwitchButton,
        AtomData::new_as_int("Items".to_string(), 0));
        items_button.set_rect((rect.0 + 700 + 175, rect.1, 140, rect.3), asset, context);
        widgets.push(items_button);

        let mut game_button = AtomWidget::new(vec!["Game".to_string()], AtomWidgetType::ToolBarButton,
            AtomData::new_as_int("Game".to_string(), 0));
        game_button.set_rect((rect.2 - 110, rect.1, 100, rect.3), asset, context);
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
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
            atom.draw(frame, context.width, anim_counter, asset, context);
        }
    }

    fn draw_overlay(&mut self, frame: &mut [u8], rect: &(usize, usize, usize, usize), anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        for atom in &mut self.widgets {
            atom.draw_overlay(frame, rect, anim_counter, asset, context);
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

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_dragged(pos, asset, context) {
                return true;
            }
        }
        false
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