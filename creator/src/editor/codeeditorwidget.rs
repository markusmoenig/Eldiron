
use core_server::asset::{ Asset };
use crate::widget::WidgetKey;
use crate::widget::codeeditor::CodeEditor;
use crate::widget::context::ScreenContext;
use crate::widget::text_editor_trait::TextEditorWidget;
//use fontdue::Font;

#[derive(PartialEq, Debug)]
pub enum CodeEditorWidgetState {
    Closed,
    Open,
    Opening,
    Closing
}

pub struct CodeEditorWidget {
    pub rect                : (usize, usize, usize, usize),
    dirty                   : bool,
    buffer                  : Vec<u8>,

    editor                  : CodeEditor,
}

impl CodeEditorWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self {

        let editor = CodeEditor::new();

        Self {
            rect,

            dirty           : true,
            buffer          : vec![0;1],

            editor,
        }
    }

    pub fn set_code(&mut self, value: String) {
        self.editor.set_text(value);
        self.dirty = true;
    }

    pub fn set_text_mode(&mut self, value: bool) {
        self.editor.set_text_mode(value);
        self.dirty = true;
    }

    pub fn _resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        let width = rect.2;
        let height = 240;

        if self.buffer.len() != width * height * 4 {
            self.buffer = vec![0; width * height * 4];
            self.dirty = true;
        }

        self.rect = rect.clone();
        let safe_rect = (0_usize, 0_usize, width, height);
        let editor_rect = (0, 0, safe_rect.2, height - 30);

        let mut dest_rect = rect.clone();
        dest_rect.1 = dest_rect.1 + dest_rect.3 - height;
        dest_rect.3 = height;

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            let mut trans_black = context.color_black.clone();
            trans_black[3] = 128;
            context.draw2d.draw_rect(buffer_frame, &safe_rect, rect.2, &trans_black);

            self.editor.draw(buffer_frame, editor_rect, rect.2, asset.get_editor_font("SourceCodePro"), &context.draw2d);

            if self.editor.cursor_rect.3 > 0 {
                context.draw2d.draw_text_rect(buffer_frame, &(0, height - 30, rect.2 - 20, 30), rect.2, asset.get_editor_font("OpenSans"), 15.0, format!("Ln {}, Col {}", self.editor.cursor_pos.1 + 1, self.editor.cursor_pos.0).as_str(), &context.color_light_white, &context.color_black, crate::draw2d::TextAlignment::Right);
            }
        }
        self.dirty = false;

        if context.code_editor_state == CodeEditorWidgetState::Opening {
            if context.code_editor_visible_y < height {
                context.code_editor_visible_y += 20;
                dest_rect.1 = dest_rect.1 + dest_rect.3 - context.code_editor_visible_y;
                dest_rect.3 = context.code_editor_visible_y;
            } else {
                context.code_editor_state = CodeEditorWidgetState::Open;
                context.target_fps = context.default_fps;
            }
        }

        if context.code_editor_state == CodeEditorWidgetState::Closing {
            if context.code_editor_visible_y > 0 {
                context.code_editor_visible_y -= 20;
                dest_rect.1 = dest_rect.1 + dest_rect.3 - context.code_editor_visible_y;
                dest_rect.3 = context.code_editor_visible_y;
            }

            if context.code_editor_visible_y == 0 {
                context.code_editor_state = CodeEditorWidgetState::Closed;
                context.target_fps = context.default_fps;
                context.code_editor_is_active = false;
            }
        }

        context.draw2d.blend_slice(frame, &mut self.buffer[..], &dest_rect, context.width);
        self.rect = dest_rect;
    }

    pub fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if key == Some(WidgetKey::Escape) {
            context.code_editor_state = CodeEditorWidgetState::Closing;
            context.target_fps = 60;
            context.code_editor_visible_y = 240;
            return true;
        }

        let consumed = self.editor.key_down(char, key, asset.get_editor_font("SourceCodePro"), &context.draw2d);
        if consumed {
            self.dirty = true;
            context.code_editor_value = self.editor.text.clone();
            context.code_editor_update_node = true;
        }
        consumed
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor.rect) {
                local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
                if self.editor.mouse_down(local_pos, asset.get_editor_font("SourceCodePro")) {
                    self.dirty = true;
                    return true;
                }
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor.rect) {
                local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
                consumed = self.editor.mouse_up(local_pos, asset.get_editor_font("SourceCodePro"));
            }
        }
        consumed
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;

        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor.rect) {
                local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
                if self.editor.mouse_dragged(local_pos, asset.get_editor_font("SourceCodePro")) {
                    self.dirty = true;
                    consumed = true;
                }
            }
        }

        consumed
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        let consumed;
        consumed = self.editor.mouse_wheel(delta, asset.get_editor_font("SourceCodePro"));
        consumed
    }

    pub fn modifier_changed(&mut self, shift: bool, ctrl: bool, alt: bool, logo: bool, asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.editor.modifier_changed(shift, ctrl, alt, logo, asset.get_editor_font("SourceCodePro"))
    }

    fn pos_to_local(&mut self, pos: (usize, usize)) -> Option<(usize, usize)> {
        if pos.0 > self.rect.0 && pos.1 > self.rect.1 {
            return Some((pos.0 - self.rect.0, pos.1 - self.rect.1));
        }
        None
    }
}