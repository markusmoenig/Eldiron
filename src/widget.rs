use std::time::{SystemTime, UNIX_EPOCH};

use crate::asset::Asset;

pub trait ScreenWidget {

    fn new() -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8]);

    fn mouse_down(&self, pos: (u32, u32)) {
        for w in self.get_widgets() {
            if w.contains(pos) {
                w.mouse_down(pos)
            }
        }
    }

    fn mouse_up(&self, _pos: (u32, u32)) {
    }

    fn get_asset(&self) -> &Asset;
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>>;

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");    
            stop.as_millis()
    }
}

// General purpose widgets
pub mod text;
pub mod tab;

pub trait Widget {

    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8], asset: &Asset);

    fn mouse_down(&self, _pos: (u32, u32)) {
    }

    fn mouse_up(&self, _pos: (u32, u32)) {
    }

    fn contains(&self, pos: (u32, u32)) -> bool {
        let rect = self.get_rect();

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32);

    fn get_content_rect(&self) -> (u32, u32, u32, u32) {
        let r = self.get_rect();
        (r.0, r.1, r.2, r.3)
    }

    // The following are widget specific and optional

    fn set_pagination(&self, _pages: u32) {
    }
}