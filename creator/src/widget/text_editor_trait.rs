use fontdue::Font;
use super::{WidgetKey, draw2d::Draw2D};

#[allow(unused)]
pub trait TextEditorWidget {

    fn new() -> Self where Self: Sized;

    fn resize(&mut self, width: usize, height: usize) {
    }

    fn set_text(&mut self, text: String);
    fn set_error(&mut self, error: Option<(String, Option<usize>)>);

    fn set_text_mode(&mut self, value: bool);

    fn process_text(&mut self, font: &Font, draw2d: &Draw2D);
    fn set_cursor_offset_from_pos(&mut self, pos: (usize, usize), font: &Font) -> bool;

    fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), stride: usize, font: &Font, draw2d: &Draw2D);

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, font: &Font, draw2d: &Draw2D) -> bool {
        false
    }

    fn mouse_down(&mut self, pos: (usize, usize), font: &Font) -> bool {
        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), font: &Font) -> bool {
        false
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), font: &Font) -> bool {
        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), font: &Font) -> bool {
        false
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), font: &Font) -> bool {
        false
    }

    fn modifier_changed(&mut self, shift: bool, ctrl: bool, alt: bool, logo: bool, _font: &Font) -> bool {
        false
    }
}
