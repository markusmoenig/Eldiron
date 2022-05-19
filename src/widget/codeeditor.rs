use super::WidgetKey;
use super::draw2d::Draw2D;

use fontdue::layout::{ Layout, LayoutSettings, CoordinateSystem, TextStyle };//, VerticalAlign, HorizontalAlign };
use fontdue::Font;

use super::text_editor_trait::TextEditorWidget;

pub struct CodeEditor {

    pub rect                : (usize, usize, usize, usize),
    pub code                : String,

    dirty                   : bool,
    buffer                  : Vec<u8>,
}

impl TextEditorWidget for CodeEditor {

    fn new() -> Self where Self: Sized {

        Self {
            rect            : (0, 0, 0, 0),
            code            : "".to_string(),

            dirty           : true,
            buffer          : vec![0;1],
        }
    }

    fn set_text(&mut self, text: String) {
        self.code = text;
    }

    fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), stride: usize, font: &Font, draw2d: &Draw2D) {

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

            //draw2d.draw_rounded_rect_with_border(buffer_frame, &safe_rect, stride, &(rect.2 as f64 - 1.0, rect.3 as f64 - 1.0), &[0,0,0,255], &(0.0, 0.0, 0.0, 0.0), &[155, 155, 155, 255], 1.5);

            let pos_x = 4;
            let pos_y = 4;

            let fonts = &[font];

            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                ..LayoutSettings::default()
            });
            layout.append(fonts, &TextStyle::new(self.code.as_str(), 20.0, 0));

            for glyph in layout.glyphs() {
                let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
                //println!("Metrics: {:?}", glyph);

                for y in 0..metrics.height {
                    for x in 0..metrics.width {
                        let i = (x+pos_x+glyph.x as usize) * 4 + (y + pos_y + glyph.y as usize) * stride * 4;
                        let m = alphamap[x + y * metrics.width];

                        buffer_frame[i..i + 4].copy_from_slice(&draw2d.mix_color(&[0,0,0,255], &[255, 255, 255, 255], m as f64 / 255.0));
                    }
                }
            }
        }

        self.dirty = false;
        draw2d.copy_slice(frame, &mut self.buffer[..], &self.rect, stride);
    }

    fn key_down(&mut self, char: Option<char>, key: Option<WidgetKey>, _font: &Font) -> bool {

        if let Some(key) = key {
            match key {
                WidgetKey::Delete => {
                    self.code.pop();
                    self.dirty = true;

                    return  true;
                },
                _ => {}
            }
        }

        if let Some(c) = char {
            if c.is_ascii() && c.is_control() == false {
                self.code.push(c);
                self.dirty = true;
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