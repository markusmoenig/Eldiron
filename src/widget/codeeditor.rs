use super::WidgetKey;
use super::codeeditor_theme::CodeEditorTheme;
use super::draw2d::Draw2D;

use fontdue::layout::{ Layout, LayoutSettings, CoordinateSystem, TextStyle };//, VerticalAlign, HorizontalAlign };
use fontdue::Font;

use super::text_editor_trait::TextEditorWidget;

pub struct CodeEditor {

    pub rect                : (usize, usize, usize, usize),
    pub text                : String,

    pub font_size           : f32,

    cursor_offset           : usize,
    pub cursor_pos          : (usize, usize),
    pub cursor_rect         : (usize, usize, usize, usize),

    pub jump_to_screen_pos  : Option<(usize, usize)>,

    needs_update            : bool,
    dirty                   : bool,
    buffer                  : Vec<u8>,

    text_buffer             : Vec<u8>,
    text_buffer_size        : (usize, usize),

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

            jump_to_screen_pos : None,

            needs_update    : true,
            dirty           : true,
            buffer          : vec![0;1],

            text_buffer     : vec![0;1],
            text_buffer_size  : (0, 0),

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

        if let Some(pos) = self.jump_to_screen_pos {
            self.set_cursor_offset_from_pos(pos, font);
            self.jump_to_screen_pos = None;
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

        let mut w = 0_usize;
        let mut h = 0_usize;

        while let Some(line) = lines.next() {

            let fonts = &[font];
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                ..LayoutSettings::default()
            });

            layout.append(fonts, &TextStyle::new(line, self.font_size, 0));

            if layout.height() == 0.0 {
                h += 26;
            } else {
                h += layout.height().ceil() as usize;
            }

            if let Some(last) = layout.glyphs().last() {
                if w < last.x.ceil() as usize + last.width {
                    w = last.x.ceil() as usize + last.width;
                }
            }
        }

        if let Some(last) = self.text.chars().last() {
            if last == '\n' {
                h += 26;
            }
        }

        let left_size = 100;

        w += left_size;

        self.needs_update = false;

        //println!("{} {}", w, h);

        self.text_buffer = vec![0; w * h * 4];
        self.text_buffer_size = (w, h);

        // Draw it

        lines = self.text.lines();

        let pos_x = left_size;
        let mut pos_y = 0;

        let stride = w;

        let mut line_number = 1;

        while let Some(line) = lines.next() {

            let fonts = &[font];
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                ..LayoutSettings::default()
            });

            layout.append(fonts, &TextStyle::new(line, self.font_size, 0));

            for glyph in layout.glyphs() {
                let text_buffer_frame = &mut self.text_buffer[..];
                let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
                //println!("Metrics: {:?}", glyph);

                for y in 0..metrics.height {
                    for x in 0..metrics.width {
                        let i = (x+pos_x+glyph.x as usize) * 4 + (y + pos_y + glyph.y as usize) * stride * 4;
                        let m = alphamap[x + y * metrics.width];

                        text_buffer_frame[i..i + 4].copy_from_slice(&draw2d.mix_color(&self.theme.background, &self.theme.text, m as f64 / 255.0));
                    }
                }
            }

            let adv_y = layout.height() as usize;

            draw2d.draw_text_rect(&mut self.text_buffer[..], &(0, pos_y, 60, if adv_y == 0 { 26 } else { adv_y }), stride, font, self.font_size, format!("{}", line_number).as_str(), &self.theme.line_numbers, &self.theme.background, crate::draw2d::TextAlignment::Right);

            if adv_y == 0 {
                pos_y += 26;
            } else {
                pos_y += layout.height().ceil() as usize;
            }

            line_number += 1;
        }

        if let Some(last) = self.text.chars().last() {
            if last == '\n' {
                draw2d.draw_text_rect(&mut self.text_buffer[..], &(0, pos_y, 60, 26), stride, font, self.font_size, format!("{}", line_number).as_str(), &[160, 160, 160, 255], &[0, 0, 0, 255], crate::draw2d::TextAlignment::Right);
            }
        }
    }

    /// Sets the cursor offset based on the given screen position
    fn set_cursor_offset_from_pos(&mut self, pos: (usize, usize), font: &Font) -> bool {

        let mut lines = self.text.lines();

        let x = pos.0;
        let y = pos.1;

        //let mut w = 0_usize;
        let mut h = 0_usize;

        let mut curr_line_index = 0_usize;
        let mut found = false;

        self.cursor_offset = 0;

        while let Some(line) = lines.next() {

            let fonts = &[font];
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                ..LayoutSettings::default()
            });

            layout.append(fonts, &TextStyle::new(line, self.font_size, 0));

            let line_height = if layout.height() == 0.0 { 26 } else { layout.height().ceil() as usize };

            if y >= h && y <= h + line_height {

                self.cursor_pos.1 = curr_line_index;
                self.cursor_rect.1 = h;
                self.cursor_rect.3 = line_height;
                self.dirty = true;

                if x <= 100 {
                    self.cursor_rect.0 = 100;
                    self.cursor_offset += line.len();
                    self.cursor_pos.0 = 0;
                } else
                if layout.height() == 0.0{
                    self.cursor_rect.0 = 100;
                    self.cursor_offset += line.len();
                    self.cursor_pos.0 = 0;
                } else {

                    self.cursor_rect.0 = 100;
                    let mut offset = 0_usize;
                    let mut found_x = false;

                    let mut adv_x = 0;

                    for glyph in layout.glyphs() {
                        let (metrics, _alphamap) = font.rasterize(glyph.parent, glyph.key.px);
                        //println!("Metrics: {:?}", glyph);

                        let glyph_x = glyph.x.ceil() as usize;

                        if x - 100 < glyph_x {
                            self.cursor_rect.0 = 100 + glyph_x - self.cursor_rect.2;
                            self.cursor_pos.0 = offset;
                            found_x = true;
                            //println!("smaller {}", self.cursor_rect.0);
                            break;
                        } else
                        if x - 100 < glyph_x + metrics.width {
                            self.cursor_rect.0 = 100 + glyph_x + metrics.width - self.cursor_rect.2;
                            self.cursor_pos.0 = offset + 1;
                            self.cursor_offset += 1;
                            found_x = true;
                            //println!("isndie {}", self.cursor_rect.0);
                            break;
                        }

                        offset += 1;
                        self.cursor_offset += 1;
                        adv_x = glyph_x + metrics.width;
                    }

                    if found_x == false {
                        self.cursor_rect.0 = 100 + adv_x - self.cursor_rect.2;
                        self.cursor_pos.0 = offset;
                    }
                }

                found = true;
                break;
            } else {
                self.cursor_offset += line.len();
            }

            h += line_height;

            if found == false {
                self.cursor_offset += 1;
            }

            curr_line_index += 1;
        }

        if let Some(last) = self.text.chars().last() {
            if last == '\n' {
                if found == false {
                    self.cursor_rect.0 = 100;
                    self.cursor_rect.1 = h;
                    self.cursor_rect.3 = 26;
                    self.cursor_pos.0 = 0;
                    self.cursor_pos.1 = curr_line_index + 1;
                    self.cursor_offset = self.text.len();
                    found = true;
                    self.dirty = true;
                }
            }
        }

        found
    }

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, font: &Font, draw2d: &Draw2D) -> bool {

        if let Some(key) = key {
            match key {
                WidgetKey::Delete => {
                    if self.cursor_offset >= 1 {
                        let index  = self.cursor_offset - 1;

                        let mut pos = (self.cursor_rect.0, self.cursor_rect.1 + 10);
                        if let Some(c) = self.text.drain(index..index+1).next() {
                            let size = draw2d.get_text_size(font, self.font_size, c.to_string().as_str());
                            //println!("s {}", size.0);
                            pos.0 -= size.0;
                            if pos.0 < 100 {
                                pos.0 = 100;
                            }
                        }
                        self.process_text(font, draw2d);
                        self.set_cursor_offset_from_pos(pos, font);

                    }
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Return => {
                    self.text.insert(self.cursor_offset, '\n');
                    self.dirty = true;
                    self.needs_update = true;
                    return  true;
                },

                WidgetKey::Up => {
                    self.set_cursor_offset_from_pos((self.cursor_rect.0, self.cursor_rect.1 - 5), font);
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Down => {
                    self.set_cursor_offset_from_pos((self.cursor_rect.0, self.cursor_rect.1 + 30), font);
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Left => {

                    let mut size = 14_usize;
                    if self.cursor_pos.0 > 0 {
                        // Go one left
                        if let Some(c) = self.text.chars().nth(self.cursor_offset - 1) {
                            let width = draw2d.get_text_size(font, self.font_size, c.to_string().as_str()).0;
                            if width > 0 {
                                size = width;
                            }
                        }
                    } else {
                        // Go one up
                        self.set_cursor_offset_from_pos((100000, self.cursor_rect.1 - 5), font);
                    }
                    self.set_cursor_offset_from_pos((self.cursor_rect.0 - size - 3, self.cursor_rect.1 + 10), font);
                    self.dirty = true;
                    return  true;
                },

                WidgetKey::Right => {
                    if let Some(c) = self.text.chars().nth(self.cursor_offset) {
                        if c == '\n' {
                            // Go down
                            println!("1 {:?}", self.cursor_pos);
                            self.set_cursor_offset_from_pos((101, self.cursor_rect.1 + 30), font);
                            println!("2 {:?}", self.cursor_pos);
                        } else {
                            // Go Right
                            self.set_cursor_offset_from_pos((self.cursor_rect.0 + 6, self.cursor_rect.1 + 10), font);
                        }
                    }
                    //self.set_cursor_offset_from_pos((self.cursor_rect.0 + 6, self.cursor_rect.1 + 10), font);
                    self.dirty = true;
                    return  true;
                },
                _ => {}
            }
        }

        if let Some(c) = char {
            if c.is_ascii() && c.is_control() == false {
                //self.text.push(c);
                self.text.insert(self.cursor_offset, c);
                self.process_text(font, draw2d);

                let mut size = draw2d.get_text_size(font, self.font_size, c.to_string().as_str());

                if size.0 == 0 {
                    size.0 = 12;
                }

                self.set_cursor_offset_from_pos((self.cursor_rect.0 + size.0, self.cursor_rect.1 + 10), font);
                self.dirty = true;
                //self.needs_update = true;
                return true;
            }
        }
        false
    }

    fn mouse_down(&mut self, pos: (usize, usize), font: &Font) -> bool {
        let consumed = self.set_cursor_offset_from_pos(pos, font);
        println!("{:?}", pos);
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
}