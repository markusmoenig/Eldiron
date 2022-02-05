use crate::prelude::*;

use crate::widget::*;

use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::tab::TabWidget;

pub struct TileSetWidget {
    rect                    : (u32, u32, u32, u32),
    options_grid            : OptionsGridWidget,
    tab_widget              : TabWidget,
}

impl Widget for TileSetWidget {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), asset: &Asset) -> Self where Self: Sized {

        let options_grid = OptionsGridWidget::new(vec!["Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "UtilityChar".to_string(), "Water".to_string(), "Harmful".to_string()], 
             (rect.0 + 100, rect.1, rect.2 - 200, UI_ELEMENT_HEIGHT), asset);

        Self {
            rect,
            options_grid             : options_grid,    
             tab_widget              : TabWidget::new(vec!(),(rect.0, rect.1 + UI_ELEMENT_HEIGHT, rect.2, rect.3 - UI_ELEMENT_HEIGHT - 1), asset),
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {
        self.options_grid.draw(frame, anim_counter, asset);
        self.tab_widget.draw(frame, anim_counter, asset);
    }

    fn mouse_down(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.options_grid.mouse_down(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn mouse_up(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.options_grid.mouse_down(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}