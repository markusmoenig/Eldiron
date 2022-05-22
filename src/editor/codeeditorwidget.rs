
use server::asset::{ Asset };
use crate::widget::WidgetKey;
use crate::widget::codeeditor::CodeEditor;
use crate::widget::context::ScreenContext;
use crate::widget::text_editor_trait::TextEditorWidget;
//use fontdue::Font;

pub struct CodeEditorWidget {
    rect                    : (usize, usize, usize, usize),
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
    }

    pub fn _resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        if self.buffer.len() != rect.2 * rect.3 * 4 {
            self.buffer = vec![0; rect.2 * rect.3 * 4];
            self.dirty = true;
        }
        self.rect = rect.clone();

        let safe_rect = (0_usize, 0_usize, self.rect.2, self.rect.3);

        if self.dirty {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];
            let stride = rect.2;

            context.draw2d.draw_rect(buffer_frame, &safe_rect, stride, &context.color_black);

            self.editor.draw(buffer_frame, (50, 50, safe_rect.2 - 100, safe_rect.3 - 100), self.rect.2, asset.get_editor_font("SourceCodePro"), &context.draw2d);

            if self.editor.cursor_rect.3 > 0 {
                context.draw2d.draw_text_rect(buffer_frame, &(0, rect.3 - 40, rect.2 - 20, 40), rect.2, asset.get_editor_font("OpenSans"), 15.0, format!("Ln {}, Col {}", self.editor.cursor_pos.1 + 1, self.editor.cursor_pos.0).as_str(), &context.color_light_gray, &context.color_black, crate::draw2d::TextAlignment::Right);
            }
        }

        self.dirty = false;
        context.draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, rect.2);
    }

    pub fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if key == Some(WidgetKey::Escape) {
            context.code_editor_is_active = false;
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

        if context.contains_pos_for(self.pos_to_local(pos), self.editor.rect) {
            let mut local_pos = self.pos_to_local(pos);
            local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
            if self.editor.mouse_down(local_pos, asset.get_editor_font("SourceCodePro")) {
                self.dirty = true;
                return true;
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        if context.contains_pos_for(self.pos_to_local(pos), self.editor.rect) {
            let mut local_pos = self.pos_to_local(pos);
            local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
            consumed = self.editor.mouse_up(local_pos, asset.get_editor_font("SourceCodePro"));
        }
        consumed
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;

        if context.contains_pos_for(self.pos_to_local(pos), self.editor.rect) {
            let mut local_pos = self.pos_to_local(pos);
            local_pos.0 -= self.editor.rect.0; local_pos.1 -= self.editor.rect.1;
            if self.editor.mouse_dragged(local_pos, asset.get_editor_font("SourceCodePro")) {
                self.dirty = true;
                consumed = true;
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

    fn pos_to_local(&mut self, pos: (usize, usize)) -> (usize, usize) {
        (pos.0 - self.rect.0, pos.1 - self.rect.1)
    }
}