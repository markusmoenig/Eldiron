
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

        let editor = CodeEditor::new(100, 100);

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

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
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

            self.editor.draw(buffer_frame, (200, 50, safe_rect.2 - 300, safe_rect.3 - 100), self.rect.2, asset.get_editor_font("SourceCodePro"), &context.draw2d);
        }

        self.dirty = false;
        context.draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, rect.2);
    }

    pub fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let consumed = self.editor.key_down(char, key, asset.get_editor_font("SourceCodePro"));
        if consumed {
            self.dirty = true;
            context.code_editor_value = self.editor.code.clone();
            context.code_editor_update_node = true;
        }
        consumed
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        true
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        let consumed = false;
        consumed
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        true
    }
}