
use crate::widget::*;
use crate::asset::*;

use rusttype::{point, Font, Scale};

pub struct TextWidget {
    title           : String,
    rect            : [u32; 4]
}

impl Widget for TextWidget {
    
    fn new(title: String, rect: [u32; 4]) -> Self where Self: Sized {
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

        asset.draw_text(frame, &[0 as usize, 0 as usize], &self.title, [255, 255, 255]);
    }
}