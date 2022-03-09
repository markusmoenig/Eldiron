use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use server::asset::Asset;

use self::context::ScreenContext;

pub mod context;
pub mod draw2d;

/// Top level screen widget
pub trait ScreenWidget {

    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized;

    fn update(&mut self);
    fn resize(&mut self, _width: usize, _height: usize) {
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset);

    fn mouse_down(&mut self, _pos: (usize, usize), _asset: &mut Asset) -> bool {
        false
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset) -> bool {
        false
    }

    fn mouse_dragged(&mut self, _pos: (usize, usize), _asset: &mut Asset) -> bool {
        false
    }

    fn mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset) -> bool {
        false
    }

    fn mouse_wheel(&mut self, _delta: (isize, isize), _asset: &mut Asset) -> bool {
        false
    }

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
            stop.as_millis()
    }

    fn get_target_fps(&self) -> usize;
}

// General purpose widgets
pub mod atom;
pub mod node;
pub mod node_preview;
pub mod tileselector;

/// The widget state

#[derive(PartialEq)]
pub enum WidgetState {
    Disabled,
    Normal,
    Hover,
}

pub trait Widget {

    fn new(text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized;

    fn update(&mut self) {}
    fn resize(&mut self, _width: usize, _height: usize, _context: &ScreenContext) {
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext);

    fn mouse_down(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    fn mouse_dragged(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    fn mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    fn mouse_wheel(&mut self, _delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    //fn set_cb(&mut self) {
    //}

    //fn set_cb<T: FnMut()>(&mut self, f: T) {
        // { self.f = Some(Box::new(f)//); }
    //}

    fn contains_pos(&self, pos: (usize, usize)) -> bool {
        let rect = self.get_rect();

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    fn get_rect(&self) -> &(usize, usize, usize, usize);

    fn get_content_rect(&self) -> (usize, usize, usize, usize) {
        let r = self.get_rect();
        (r.0, r.1, r.2, r.3)
    }

    /// Set the current state of the widget
    fn set_state(&self, _state: u32) {
    }

    // Default colors

    fn get_color_background(&self) -> [u8; 4] {
        [43, 43, 43, 255]
    }

    fn get_color_selection(&self) -> [u8; 4] {
        [73, 73, 73, 255]
    }

    fn get_color_selection_blue(&self) -> [u8; 4] {
        [59, 70, 90, 255]
    }

    fn get_color_text(&self) -> [u8; 4] {
        [255, 255, 255, 255]
    }

    fn get_color_text_disabled(&self) -> [u8; 4] {
        [128, 128, 128, 255]
    }
}