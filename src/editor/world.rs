use crate::prelude::*;

use crate::widget::*;

use crate::tab::TabWidget;
use crate::button::ButtonWidget;
use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::asset::tileset::TileUsage;
use crate::menu::MenuWidget;

use crate::asset::tileset::TileMap;

use core::cmp::max;
use core::cell::Cell;

pub struct WorldEditor {
    rect                    : (u32, u32, u32, u32),
    scale                   : f32,
}

impl Widget for WorldEditor {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), asset: &Asset) -> Self where Self: Sized {

        // let set_anim_button = ButtonWidget::new(vec!["Set Anim".to_string()], (20 + 100 + 40, HEIGHT / 2 + 96, 120,  UI_ELEMENT_HEIGHT), asset);
        // let clear_anim_button = ButtonWidget::new(vec!["Clear Anim".to_string()], (20 + 100 + 40 + 120 + 8, HEIGHT / 2 + 96, 120,  UI_ELEMENT_HEIGHT), asset);

        // let mut names : Vec<String> = vec![];
        // for tm in &asset.tileset.maps_names {
        //     names.push(tm.to_string());
        // }

        // let tilemap_menu = MenuWidget::new(names, (WIDTH - 120 - 10, 0, 120,  UI_ELEMENT_HEIGHT), asset);

        Self {
            rect,
            scale                   : 2_f32
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {
        //asset.draw_rect(frame, &self.get_content_rect(), [0,0,0,255]);
    }

    fn mouse_down(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;

        consumed
    }

    fn mouse_up(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        
        consumed
    }

    /// Set the screen_end_selected point
    fn mouse_dragged(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        false
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}