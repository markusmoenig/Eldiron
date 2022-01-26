
use crate::widget::*;
use crate::asset::TextAlignment;

use core::cell::Cell;


pub struct TabWidget {
    title           : String,
    rect            : (u32, u32, u32, u32),
    content_rect    : Cell<(u32, u32, u32, u32)>,
    pages           : Cell<u32>,
    curr_page       : Cell<u32>
}

impl Widget for TabWidget {
    
    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {
        Self {
            title           : title,
            rect,
            content_rect    : Cell::new((0,0,0,0)),
            pages           : Cell::new(1),
            curr_page       : Cell::new(0)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {
        if self.pages.get() > 1 {
            self.content_rect.set((self.rect.0, self.rect.1, self.rect.2, self.rect.3 - asset.get_text_element_height()));

            let pages = self.pages.get();
            let page_width = self.rect.2 / pages;

            for p in 0..pages {
                let r: (u32,u32,u32,u32) = (self.rect.0 + page_width * p, self.rect.1 + self.rect.3 - asset.get_text_element_height(), page_width, asset.get_text_element_height());
                let mut background = [128, 128, 128, 255];
                if p == self.curr_page.get() {
                    background = [64, 64, 64, 255];
                }
                asset.draw_text_rect(frame, &r, &format!("Page {}", p + 1),[255, 255, 255, 255], background, TextAlignment::Center);            
            }
        }
    }

    fn mouse_down(&self, pos: (u32, u32)) {
        println!("text {:?}", pos);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        &self.rect
    }

    fn get_content_rect(&self) -> (u32, u32, u32, u32) {
        let r = self.content_rect.get();
        (r.0, r.1, r.2, r.3)
    }

    fn set_pagination(&self, pages: u32) {
        self.pages.set(pages);

        if pages == 1 {
            self.content_rect.set((self.rect.0, self.rect.1, self.rect.2, self.rect.3));
        } else {
            self.content_rect.set((self.rect.0, self.rect.1, self.rect.2, self.rect.3 - 20));
        }
    }
}