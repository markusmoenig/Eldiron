use crate::prelude::*;

pub mod context;
pub mod draw2d;

// General purpose widgets
pub mod atom;
pub mod characterselector;
pub mod lootselector;
pub mod tileselector;

// Layouts

pub mod hlayout;
pub mod vlayout;

/// The widget state

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum WidgetState {
    Disabled,
    Normal,
    Hover,
    Clicked,
}

#[allow(unused)]
pub trait Widget {
    fn new(
        text: Vec<String>,
        rect: (usize, usize, usize, usize),
        asset: &Asset,
        context: &mut ScreenContext,
    ) -> Self
    where
        Self: Sized;

    fn update(&mut self) {}
    fn resize(&mut self, width: usize, height: usize, context: &ScreenContext) {}

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
    );
    fn draw_overlay(
        &mut self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
    );

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn mouse_up(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn mouse_dragged(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn modifier_changed(
        &mut self,
        shift: bool,
        ctrl: bool,
        alt: bool,
        logo: bool,
        asset: &mut Asset,
        context: &mut ScreenContext,
    ) -> bool {
        false
    }

    fn get_atom_at_index(&mut self, index: usize) -> Option<&mut AtomWidget> {
        None
    }

    fn contains_pos(&self, pos: (usize, usize)) -> bool {
        let rect = self.get_rect();

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3
        {
            true
        } else {
            false
        }
    }

    fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3
        {
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
    fn set_state(&self, _state: u32) {}

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

    fn stop_debugging(&mut self, context: &mut ScreenContext) {}
}
