use std::collections::HashMap;

use super::WidgetKey;
use super::codeeditor_theme::CodeEditorTheme;
use super::draw2d::Draw2D;

use fontdue::{ Font, Metrics };

use super::text_editor_trait::TextEditorWidget;

pub struct CodeEditor {

    pub rect                : (usize, usize, usize, usize),
    pub text                : String,

    pub font_size           : f32,

    cursor_offset           : usize,
    pub cursor_pos          : (usize, usize),
    pub cursor_rect         : (usize, usize, usize, usize),

    needs_update            : bool,
    dirty                   : bool,
    buffer                  : Vec<u8>,

    text_buffer             : Vec<u8>,
    text_buffer_size        : (usize, usize),

    metrics                 : HashMap<char, (Metrics, Vec<u8>)>,
    advance_width           : usize,

    shift                   : bool,
    ctrl                    : bool,
    alt                     : bool,
    logo                    : bool,

    theme                   : CodeEditorTheme,
}

impl TextEditorWidget for CodeEditor {

    fn new() -> Self where Self: Sized {

        Self {
            rect            : (0, 0, 0, 0),
            text            : "".to_string(),

            font_size       : 20.0,

            cursor_offset   : 0,
            cursor_pos      : (0, 0),
            cursor_rect     : (0, 0, 2, 0),

            needs_update    : true,
            dirty           : true,
            buffer          : vec![0;1],

            text_buffer     : vec![0;1],
            text_buffer_size  : (0, 0),

            metrics         : HashMap::new(),
            advance_width   : 12,

            shift           : false,
            ctrl            : false,
            alt             : false,
            logo            : false,

            theme           : CodeEditorTheme::new(),
        }
    }

    fn set_text(&mut self, text: String) {
        self.text = text;
        self.needs_update = true;
    }

    fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), stride: usize, font: &Font, draw2d: &Draw2D) {

        if self.needs_update {
            self.process_text(font, draw2d);
        }

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

            draw2d.draw_rect(buffer_frame, &safe_rect, stride, &self.theme.background);
            draw2d.draw_rect(buffer_frame, &(0, 0, 95, safe_rect.3), stride, &self.theme.line_numbers_bg);

            draw2d.blend_slice(buffer_frame, &mut self.text_buffer[..], &(0, 0, self.text_buffer_size.0, self.text_buffer_size.1), stride);

            //println!("{:?}", self.cursor_rect);
            draw2d.draw_rect(buffer_frame, &self.cursor_rect, stride, &self.theme.cursor);
        }

        self.dirty = false;
        draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, stride);
    }

    /// Takes the current text and renders it to the text_buffer bitmap
    fn process_text(&mut self, font: &Font, draw2d: &Draw2D) {

        let mut lines = self.text.lines();

        let mut screen_width = 0_usize;
        let mut screen_height = 0_usize;

        while let Some(line) = lines.next() {

            let mut chars = line.chars();
            let mut line_width = 0;
            while let Some(c) = chars.next() {

                if self.metrics.contains_key(&c) == false {
                    let m= font.rasterize(c, self.font_size);
                    println!("{} {:?}", c.to_string(), m.0);
                    self.metrics.insert(c, m);
                }

                if let Some((metrics, _bitmap)) = self.metrics.get(&c) {

                    line_width += metrics.advance_width.ceil() as usize;
                }
            }

            if line_width > screen_width {
                screen_width = line_width;
            }

            screen_height += 26;
        }

        //println!("{} x {}", screen_width, screen_height);

        let left_size = 100;
        screen_width += left_size;
        screen_height += left_size;
        self.needs_update = false;

        self.text_buffer = vec![0; screen_width * screen_height * 4];
        self.text_buffer_size = (screen_width, screen_height);

        // Draw it

        lines = self.text.lines();

        let mut y = 0;

        let stride = screen_width;

        let mut line_number = 1;

        while let Some(line) = lines.next() {

            let mut chars = line.chars();
            let mut x = left_size;
            while let Some(c) = chars.next() {

                if let Some((metrics, bitmap)) = self.metrics.get(&c) {
                    let text_buffer_frame = &mut self.text_buffer[..];

                    for cy in 0..metrics.height {
                        for cx in 0..metrics.width {

                            let fy = (self.font_size as isize - metrics.height as isize - metrics.ymin as isize) as usize;

                            let i = (x + cx) * 4 + (y + cy + fy) * stride * 4;
                            let m = bitmap[cx + cy * metrics.width];

                            text_buffer_frame[i..i + 4].copy_from_slice(&draw2d.mix_color(&self.theme.background, &self.theme.text, m as f64 / 255.0));
                        }
                    }

                    x += self.advance_width;//metrics.advance_width as usize;
                }
            }

            draw2d.draw_text_rect(&mut self.text_buffer[..], &(0, y, 60, 26), stride, font, self.font_size, format!("{}", line_number).as_str(), &self.theme.line_numbers, &self.theme.background, crate::draw2d::TextAlignment::Right);

            y += 26;
            line_number += 1;
        }
    }

    /// Sets the cursor offset based on the given screen position
    fn set_cursor_offset_from_pos(&mut self, pos: (usize, usize), _font: &Font) -> bool {

        let mut lines = self.text.lines();

        let px = pos.0;
        let py = pos.1;

        let left_size = 100_usize;
        let line_height = 26;

        self.cursor_offset = 0;

        let mut curr_line_index = 0_usize;

        let mut y = 0;

        while let Some(line) = lines.next() {

            if py >= y && py <= y + 26 {

                self.cursor_pos.0 = 0;
                self.cursor_pos.1 = curr_line_index;
                self.cursor_rect.0 = left_size;
                self.cursor_rect.1 = y;
                self.cursor_rect.3 = line_height;
                self.dirty = true;

                if px > 100 {
                    self.cursor_pos.0 = std::cmp::min((px - 100) / self.advance_width + 1, line.len());
                    self.cursor_rect.0 += self.cursor_pos.0 * self.advance_width - 2;
                }

                self.cursor_offset += self.cursor_pos.0;

                break;
            } else {
                self.cursor_offset += line.len();
            }

            curr_line_index += 1;
            y += line_height;
            self.cursor_offset += 1;
        }

        true
    }

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, font: &Font, draw2d: &Draw2D) -> bool {

        if let Some(key) = key {
            match key {
                WidgetKey::Delete => {
                    if self.cursor_offset >= 1 {
                        let index  = self.cursor_offset - 1;

                        let delete_line;
                        if self.cursor_pos.0 == 0 {
                            delete_line = true;
                        } else {
                            delete_line = false;
                        }

                        self.text.drain(index..index+1).next();
                        self.process_text(font, draw2d);

                        if delete_line == false {
                            self.set_cursor_offset_from_pos((self.cursor_rect.0 - self.advance_width, self.cursor_rect.1 + 10), font);
                        } else {
                            self.set_cursor_offset_from_pos((100000, self.cursor_rect.1 - 5), font);
                        }
                    }
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Return => {
                    self.text.insert(self.cursor_offset, '\n');
                    self.process_text(font, draw2d);
                    self.set_cursor_offset_from_pos((100, self.cursor_rect.1 + 30), font);
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Up => {
                    if self.cursor_rect.1 >= 5 {
                        self.set_cursor_offset_from_pos((self.cursor_rect.0, self.cursor_rect.1 - 5), font);
                    }
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Down => {
                    self.set_cursor_offset_from_pos((self.cursor_rect.0, self.cursor_rect.1 + 30), font);
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Left => {

                    if self.logo || self.ctrl {
                        self.set_cursor_offset_from_pos((100, self.cursor_rect.1 + 10), font);
                    } else {

                        if self.cursor_pos.0 > 0 && self.cursor_rect.0 >= 100 {
                            // Go one left
                            self.set_cursor_offset_from_pos((self.cursor_rect.0 - self.advance_width, self.cursor_rect.1 + 10), font);
                        } else {
                            // Go one up
                            if self.cursor_rect.1 >= 5 {
                                self.set_cursor_offset_from_pos((100000, self.cursor_rect.1 - 5), font);
                            }
                        }
                    }
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Right => {
                    if self.logo || self.ctrl {
                        self.set_cursor_offset_from_pos((100000, self.cursor_rect.1 + 10), font);
                    } else {
                        if let Some(c) = self.text.chars().nth(self.cursor_offset) {
                            if c == '\n' {
                                // Go down
                                self.set_cursor_offset_from_pos((100, self.cursor_rect.1 + 30), font);
                            } else {
                                // Go Right
                                self.set_cursor_offset_from_pos((self.cursor_rect.0 + 6, self.cursor_rect.1 + 10), font);
                            }
                        }
                    }
                    self.dirty = true;
                    return  true;
                },
                _ => {}
            }
        }

        if let Some(c) = char {
            if c.is_ascii() && c.is_control() == false {
                self.text.insert(self.cursor_offset, c);
                self.process_text(font, draw2d);
                self.set_cursor_offset_from_pos((self.cursor_rect.0 + self.advance_width, self.cursor_rect.1 + 10), font);
                self.dirty = true;
                return true;
            }
        }
        false
    }

    fn mouse_down(&mut self, pos: (usize, usize), font: &Font) -> bool {
        let consumed = self.set_cursor_offset_from_pos(pos, font);
        //println!("{:?}", pos);
        consumed
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _font: &Font) -> bool {
        false
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), font: &Font) -> bool {
        let consumed = self.set_cursor_offset_from_pos(pos, font);
        //println!("{:?}", self.cursor_offset);
        consumed
    }

    fn mouse_hover(&mut self, _pos: (usize, usize), _font: &Font) -> bool {
        false
    }

    fn mouse_wheel(&mut self, _delta: (isize, isize), _font: &Font) -> bool {
        false
    }

    fn modifier_changed(&mut self, shift: bool, ctrl: bool, alt: bool, logo: bool, _font: &Font) -> bool {
        self.shift = shift;
        self.ctrl = ctrl;
        self.alt = alt;
        self.logo = logo;
        false
    }
}