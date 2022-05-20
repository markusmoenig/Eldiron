use super::WidgetKey;
use super::draw2d::Draw2D;

use fontdue::layout::{ Layout, LayoutSettings, CoordinateSystem, TextStyle };//, VerticalAlign, HorizontalAlign };
use fontdue::Font;

use super::text_editor_trait::TextEditorWidget;

pub struct CodeEditor {

    pub rect                : (usize, usize, usize, usize),
    pub text                : String,

    pub font_size           : f32,

    cursor_offset           : usize,

    needs_update            : bool,
    dirty                   : bool,
    buffer                  : Vec<u8>,

    text_buffer             : Vec<u8>,
    text_buffer_size        : (usize, usize)

}

impl TextEditorWidget for CodeEditor {

    fn new() -> Self where Self: Sized {

        Self {
            rect            : (0, 0, 0, 0),
            text            : "".to_string(),

            font_size       : 20.0,

            cursor_offset   : 0,

            needs_update    : true,
            dirty           : true,
            buffer          : vec![0;1],

            text_buffer     : vec![0;1],
            text_buffer_size  : (0, 0),
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

            draw2d.draw_rect(buffer_frame, &safe_rect, stride, &[23, 23, 24, 255]);

            draw2d.copy_slice(buffer_frame, &mut self.text_buffer[..], &(0, 0, self.text_buffer_size.0, self.text_buffer_size.1), stride);
        }

        self.dirty = false;
        draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, stride);
    }

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

                        text_buffer_frame[i..i + 4].copy_from_slice(&draw2d.mix_color(&[0,0,0,255], &[255, 255, 255, 255], m as f64 / 255.0));
                    }
                }
            }

            let adv_y = layout.height() as usize;

            draw2d.draw_text_rect(&mut self.text_buffer[..], &(0, pos_y, 60, if adv_y == 0 { 26 } else { adv_y }), stride, font, self.font_size, format!("{}", line_number).as_str(), &[160, 160, 160, 255], &[0, 0, 0, 255], crate::draw2d::TextAlignment::Right);

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

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, _font: &Font) -> bool {

        if let Some(key) = key {
            match key {
                WidgetKey::Delete => {
                    self.text.pop();
                    self.dirty = true;
                    self.needs_update = true;
                    return  true;
                },
                _ => {}
            }

            match key {
                WidgetKey::Return => {
                    self.text.push('\n');
                    self.dirty = true;
                    self.needs_update = true;
                    return  true;
                },
                _ => {}
            }
        }

        if let Some(c) = char {
            if c.is_ascii() && c.is_control() == false {
                self.text.push(c);
                self.dirty = true;
                self.needs_update = true;
                return true;
            }
        }
        false
    }

    fn mouse_down(&mut self, pos: (usize, usize), _font: &Font) -> bool {

        println!("{:?}", pos);
        false
    }

    fn mouse_up(&mut self, _pos: (usize, usize), _font: &Font) -> bool {
        false
    }

    fn mouse_dragged(&mut self, _pos: (usize, usize), _font: &Font) -> bool {
        false
    }

    fn mouse_hover(&mut self, _pos: (usize, usize), _font: &Font) -> bool {
        false
    }

    fn mouse_wheel(&mut self, _delta: (isize, isize), _font: &Font) -> bool {
        false
    }
}