
use crate::prelude::UI_ELEMENT_HEIGHT;
use crate::widget::*;
use crate::asset::TextAlignment;

use core::cell::Cell;


pub struct TabWidget {
    rect            : (u32, u32, u32, u32),
    content_rect    : Cell<(u32, u32, u32, u32)>,
    pages           : Cell<u32>,
    pub curr_page   : Cell<u32>,
    helper          : TabWidgetHelper
}

impl Widget for TabWidget {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), _asset: &Asset) -> Self where Self: Sized {
        Self {
            rect,
            content_rect    : Cell::new((0,0,0,0)),
            pages           : Cell::new(1),
            curr_page       : Cell::new(0),
            helper          : TabWidgetHelper {}
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], _anim_counter: u32, asset: &mut Asset) {
        //if self.pages.get() > 1 {
            self.content_rect.set((self.rect.0, self.rect.1, self.rect.2, self.rect.3 - asset.get_text_element_height()));

            let pages = self.pages.get();

            for p in 0..pages {
                let r: (u32,u32,u32,u32) = self.helper.get_page_rect(&self, p);
                let mut background = self.get_color_background();
                if p == self.curr_page.get() {
                    background = self.get_color_selection_blue();
                }
                asset.draw_text_rect(frame, &r, &format!("Page {}", p + 1),self.get_color_text(), background, TextAlignment::Center);            
            }
        //}
    }

    fn mouse_down(&mut self, pos: (u32, u32), _asset: &mut Asset) -> bool {
        if self.pages.get() > 1 {

            let pages = self.pages.get();

            for p in 0..pages {
                let r: (u32,u32,u32,u32) = self.helper.get_page_rect(&self, p);
                if self.contains_pos_for(pos, r) {
                    self.curr_page.set(p);
                    return true;
                }
            }
        }        
        false
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        &self.rect
    }

    fn get_content_rect(&self) -> (u32, u32, u32, u32) {
        let r = self.content_rect.get();
        (r.0, r.1, r.2, r.3)
    }
}

pub struct TabWidgetHelper;

impl TabWidgetHelper {

    pub fn set_pagination(&self, tab_widget: &mut TabWidget, pages: u32) {
        tab_widget.pages.set(pages);

        tab_widget.content_rect.set((tab_widget.rect.0, tab_widget.rect.1, tab_widget.rect.2, tab_widget.rect.3 - UI_ELEMENT_HEIGHT));
        tab_widget.content_rect.set((tab_widget.rect.0, tab_widget.rect.1, tab_widget.rect.2, tab_widget.rect.3 - UI_ELEMENT_HEIGHT));
    }

    pub fn get_page_rect(&self, tab_widget: &TabWidget, page: u32) -> (u32, u32, u32, u32) {

        let pages = tab_widget.pages.get();
        let page_width = tab_widget.rect.2 / pages;

        (tab_widget.rect.0 + page_width * page, tab_widget.rect.1 + tab_widget.rect.3 - UI_ELEMENT_HEIGHT, page_width, UI_ELEMENT_HEIGHT)
    }
}