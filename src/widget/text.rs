
use crate::widget::*;

//use rusttype::{point, Font, Scale};

pub struct TextWidget {
    title           : String,
    rect            : (u32, u32, u32, u32)
}

impl Widget for TextWidget {
    
    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {
        Self {
            title   : title,
            rect
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {
        //println!("{}", "here")

        asset.draw_text(frame, &(self.rect.0, self.rect.1), &self.title, [255, 255, 255]);
    }

    fn mouse_down(&self, pos: (u32, u32)) {
        println!("text {:?}", pos);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}