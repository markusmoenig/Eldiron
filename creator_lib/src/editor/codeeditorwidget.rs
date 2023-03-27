use crate::prelude::*;

#[derive(PartialEq, Debug)]
pub enum CodeEditorWidgetState {
    Closed,
    Open,
    Opening,
    Closing
}

#[derive(PartialEq, Debug)]
pub enum CodeEditorSize {
    Small,
    Medium,
    Full,
}

pub struct CodeEditorWidget {
    pub rect                : (usize, usize, usize, usize),
    pub editor_rect         : (usize, usize, usize, usize),
    dirty                   : bool,
    buffer                  : Vec<u8>,

    size                    : CodeEditorSize,

    editor                  : CodeEditor,
}

impl CodeEditorWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self {

        let editor = CodeEditor::new();

        Self {
            rect,
            editor_rect     : (0, 0, 0, 0),

            dirty           : true,
            buffer          : vec![0;1],

            size            : CodeEditorSize::Small,

            editor,
        }
    }

    pub fn init(&mut self, context: &ScreenContext) {
        let path = context.resource_path.join("resources/Source_Code_Pro/static/SourceCodePro-Regular.ttf");
        if let Some(path_str) = path.to_str(){
            self.editor.set_font(path_str);
        }
    }

    pub fn set_code(&mut self, value: String) {
        self.editor.set_text(value);
        self.editor.set_error(None);
        self.dirty = true;
    }

    pub fn set_mode(&mut self, mode: CodeEditorMode) {
        self.editor.set_mode(mode);
        self.dirty = true;
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn has_undo(&mut self) -> bool {
        self.editor.has_undo()
    }

    pub fn has_redo(&mut self) -> bool {
        self.editor.has_redo()
    }

    pub fn undo(&mut self, context: &mut ScreenContext) {
        self.dirty = true;
        self.editor.undo();
        context.code_editor_value = self.editor.get_text().clone();
        context.code_editor_update_node = true;
    }

    pub fn redo(&mut self, context: &mut ScreenContext) {
        self.dirty = true;
        self.editor.redo();
        context.code_editor_value = self.editor.get_text().clone();
        context.code_editor_update_node = true;
    }

    pub fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), _anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        self.rect = rect.clone();

        let width = rect.2;
        let height = self.get_height();

        if self.buffer.len() != width * height * 4 {
            self.buffer = vec![0; width * height * 4];
            self.dirty = true;
        }

        let safe_rect = (0_usize, 0_usize, width, height);
        let editor_rect = (0, 0, safe_rect.2, height);

        let mut dest_rect = rect.clone();
        dest_rect.1 = dest_rect.1 + dest_rect.3 - height;
        dest_rect.3 = height;

        if self.dirty || self.editor.drag_pos.is_some() {
            for i in &mut self.buffer[..] { *i = 0 }
            let buffer_frame = &mut self.buffer[..];

            let mut trans_black = context.color_black.clone();
            trans_black[3] = 128;
            context.draw2d.draw_rect(buffer_frame, &safe_rect, rect.2, &trans_black);

            self.editor.set_error(context.code_editor_error.clone());
            self.editor_rect = editor_rect;
            self.editor.draw(buffer_frame, editor_rect, rect.2);
            if self.editor.cursor_rect.3 > 0 {
                context.draw2d.blend_rect(buffer_frame, &(0, height - 30, rect.2, 30), rect.2, &trans_black);

                if let Some(error) = &context.code_editor_error {
                    context.draw2d.blend_text_rect(buffer_frame, &(10, height - 30, rect.2 - 200, 30), rect.2, asset.get_editor_font("OpenSans"), 15.0, error.0.as_str(), &self.editor.theme.error,  crate::draw2d::TextAlignment::Left);
                }

                let mut size_text = "Small".to_string();
                if self.size == CodeEditorSize::Medium {
                    size_text = "Medium".to_owned();
                } else
                if self.size == CodeEditorSize::Full {
                    size_text = "Full".to_owned();
                }

                context.draw2d.blend_text_rect(buffer_frame, &(rect.2 - 200, height - 30, 70, 30), rect.2, asset.get_editor_font("OpenSans"), 15.0, size_text.as_str(), &context.color_light_white, crate::draw2d::TextAlignment::Left);

                context.draw2d.blend_text_rect(buffer_frame, &(0, height - 30, rect.2 - 20, 30), rect.2, asset.get_editor_font("OpenSans"), 15.0, format!("Ln {}, Col {}", self.editor.cursor_pos.1 + 1, self.editor.cursor_pos.0).as_str(), &context.color_light_white, crate::draw2d::TextAlignment::Right);
            }
        }
        self.dirty = false;

        if context.code_editor_state == CodeEditorWidgetState::Opening {
            if context.code_editor_visible_y < height {
                context.code_editor_visible_y += 20;
                if context.code_editor_visible_y  > height {
                    context.code_editor_visible_y = height;
                }
                dest_rect.1 = dest_rect.1 + dest_rect.3 - context.code_editor_visible_y;
                dest_rect.3 = context.code_editor_visible_y;

                if dest_rect.3 >= height {
                    dest_rect.3 = height;
                }
            } else {
                context.code_editor_state = CodeEditorWidgetState::Open;
                context.target_fps = context.default_fps;
            }
        }

        if context.code_editor_state == CodeEditorWidgetState::Closing {
            if context.code_editor_visible_y >= 20 {
                context.code_editor_visible_y -= 20;
                dest_rect.1 = dest_rect.1 + dest_rect.3 - context.code_editor_visible_y;
                dest_rect.3 = context.code_editor_visible_y;
            } else {
                context.code_editor_visible_y = 0;
            }

            if context.code_editor_visible_y == 0 {
                context.code_editor_state = CodeEditorWidgetState::Closed;
                context.target_fps = context.default_fps;
                context.code_editor_is_active = false;
            }
        }

        if context.code_editor_is_active {
            context.draw2d.blend_slice(frame, &mut self.buffer[..], &dest_rect, context.width);
        }
        self.rect = dest_rect;
    }

    pub fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if key == Some(WidgetKey::Escape) && context.code_editor_node_behavior_id.2 != "region_settings" {
            context.code_editor_state = CodeEditorWidgetState::Closing;
            context.target_fps = 60;
            context.code_editor_visible_y = self.get_height();
            return true;
        }

        let consumed = self.editor.key_down(char, key);
        if consumed {
            self.dirty = true;
            context.code_editor_value = self.editor.get_text().clone();
            context.code_editor_update_node = true;
        }
        consumed
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if pos.1 > self.rect.1 + self.rect.3 - 30 {
            if self.size == CodeEditorSize::Small {
                self.size = CodeEditorSize::Medium;
            } else
            if self.size == CodeEditorSize::Medium {
                self.size = CodeEditorSize::Full;
            } else
            if self.size == CodeEditorSize::Full {
                self.size = CodeEditorSize::Small;
            }
            return true;
        } else
        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor_rect) {
                local_pos.0 -= self.editor_rect.0; local_pos.1 -= self.editor_rect.1;
                if self.editor.mouse_down(local_pos) {
                    self.dirty = true;
                    return true;
                }
            }
        }
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor_rect) {
                local_pos.0 -= self.editor_rect.0; local_pos.1 -= self.editor_rect.1;
                consumed = self.editor.mouse_up(local_pos);
            }
        }
        consumed
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;

        if let Some(mut local_pos) = self.pos_to_local(pos) {
            if context.contains_pos_for(local_pos, self.editor_rect) {
                local_pos.0 -= self.editor_rect.0; local_pos.1 -= self.editor_rect.1;
                if self.editor.mouse_dragged(local_pos) {
                    self.dirty = true;
                    consumed = true;
                }
            }
        }

        consumed
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        let consumed;
        consumed = self.editor.mouse_wheel(delta);
        self.dirty = consumed;
        consumed
    }

    pub fn modifier_changed(&mut self, shift: bool, ctrl: bool, alt: bool, logo: bool, _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.editor.modifier_changed(shift, ctrl, alt, logo)
    }

    fn pos_to_local(&mut self, pos: (usize, usize)) -> Option<(usize, usize)> {
        if pos.0 > self.rect.0 && pos.1 > self.rect.1 {
            return Some((pos.0 - self.rect.0, pos.1 - self.rect.1));
        }
        None
    }

    fn get_height(&mut self) -> usize{
        if self.size == CodeEditorSize::Small {
            return 250;
        }
        if self.size == CodeEditorSize::Medium {
            return self.rect.3 / 2;
        }
        self.rect.3
    }
}