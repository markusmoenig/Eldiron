use core_server::gamedata::region::GameRegion;
use core_shared::prelude::*;
use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
};
use fontdue::Font;

use super::context::ScreenContext;

#[derive(PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

pub struct Draw2D {
    pub mask: Option<Vec<f32>>,
    pub mask_size: (usize, usize),

    pub scissor: Option<(usize, usize, usize, usize)>,
}

impl Draw2D {
    pub fn new() -> Self {
        Self {
            mask: None,
            mask_size: (0, 0),
            scissor: None,
        }
    }

    /// Safe to write a pixel
    fn is_safe(&self, x: usize, y: usize) -> bool {
        if let Some(s) = &self.scissor {
            // if x < s.0 || x >= s.0 + s.2 || y < s.1 || y >= s.1 + s.3 {
            //     return false;
            // }
            if x >= s.0 && x < s.0 + s.2 && y >= s.1 && y < s.1 + s.3 {
                return true;
            }
        }

        false
    }

    /// Draws the mask
    pub fn blend_mask(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        mask_frame: &[u8],
        mask_size: &(usize, usize),
        color: &[u8; 4],
    ) {
        for y in 0..mask_size.1 {
            for x in 0..mask_size.0 {
                let i = (x + rect.0) * 4 + (y + rect.1) * stride * 4;
                let m = mask_frame[x + y * mask_size.0];
                let c: [u8; 4] = [color[0], color[1], color[2], m];

                let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &c, m as f32 / 255.0));
            }
        }
    }

    /// Draws the given rectangle
    pub fn draw_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(color);
            }
        }
    }

    /// Draws the given rectangle
    pub fn _draw_rect_safe(
        &self,
        frame: &mut [u8],
        rect: &(isize, isize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        safe_rect: &(usize, usize, usize, usize),
    ) {
        let dest_stride_isize = stride as isize;
        for y in rect.1..rect.1 + rect.3 as isize {
            if y >= safe_rect.1 as isize && y < (safe_rect.1 + safe_rect.3) as isize {
                for x in rect.0..rect.0 + rect.2 as isize {
                    if x >= safe_rect.0 as isize && x < (safe_rect.0 + safe_rect.2) as isize {
                        let i = (x * 4 + y * dest_stride_isize * 4) as usize;
                        frame[i..i + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    /// Blend the given rectangle
    pub fn blend_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(
                    &background,
                    &color,
                    color[3] as f32 / 255.0,
                ));
            }
        }
    }

    /// Draws the outline of a given rectangle
    pub fn draw_rect_outline(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: [u8; 4],
    ) {
        let y = rect.1;
        for x in rect.0..rect.0 + rect.2 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = x * 4 + (y + rect.3 - 1) * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }

        let x = rect.0;
        for y in rect.1..rect.1 + rect.3 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = (x + rect.2 - 1) * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }
    }

    /// Draws the outline of a given rectangle
    pub fn draw_rect_outline_safe(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: [u8; 4],
    ) {
        let y = rect.1;
        for x in rect.0..rect.0 + rect.2 {
            if self.is_safe(x, y) {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(&color);
            }

            if self.is_safe(x, y + rect.3 - 1) {
                let i = x * 4 + (y + rect.3 - 1) * stride * 4;
                frame[i..i + 4].copy_from_slice(&color);
            }
        }

        let x = rect.0;
        for y in rect.1..rect.1 + rect.3 {
            if self.is_safe(x, y) {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(&color);
            }

            if self.is_safe(x + rect.2 - 1, y) {
                let i = (x + rect.2 - 1) * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(&color);
            }
        }
    }

    /// Draws a circle
    pub fn draw_circle(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        radius: f32,
    ) {
        let center = (
            rect.0 as f32 + rect.2 as f32 / 2.0,
            rect.1 as f32 + rect.3 as f32 / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f32 - center.0).powf(2.0) + (y as f32 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 0.0 {
                    let t = self.fill_mask(d);
                    //let t = self.smoothstep(0.0, -2.0, r);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mixed_color = self.mix_color(&background, &color, t);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a circle with a border of a given size
    pub fn _draw_circle_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        radius: f32,
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let center = (
            rect.0 as f32 + rect.2 as f32 / 2.0,
            rect.1 as f32 + rect.3 as f32 / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f32 - center.0).powf(2.0) + (y as f32 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t);

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect
    pub fn draw_rounded_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        size: &(f32, f32),
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
    ) {
        let center = (
            rect.0 as f32 + size.0 / 2.0,
            rect.1 as f32 + size.1 / 2.0 + (rect.3 as f32 - size.1) / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - size.0 / 2.0 + r.0,
                    p.1.abs() - size.1 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color =
                        self.mix_color(&background, &color, t * (color[3] as f32 / 255.0));
                    mixed_color[3] = (mixed_color[3] as f32 * (color[3] as f32 / 255.0)) as u8;
                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect
    pub fn _draw_rounded_rect_2(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
    ) {
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - rect.2 as f32 / 2.0 + r.0,
                    p.1.abs() - rect.3 as f32 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color =
                        self.mix_color(&background, &color, t * (color[3] as f32 / 255.0));
                    mixed_color[3] = (mixed_color[3] as f32 * (color[3] as f32 / 255.0)) as u8;
                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// crate a rounded rect mask
    pub fn create_rounded_rect_mask(
        &self,
        frame: &mut [f32],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        rounding: &(f32, f32, f32, f32),
    ) {
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x + y * stride;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - rect.2 as f32 / 2.0 + r.0,
                    p.1.abs() - rect.3 as f32 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);
                    frame[i] = t as f32;
                }
            }
        }
    }

    /// Blends a rounded rect
    pub fn blend_rounded_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        size: &(f32, f32),
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
    ) {
        let center = (
            rect.0 as f32 + size.0 / 2.0,
            rect.1 as f32 + size.1 / 2.0 + (rect.3 as f32 - size.1) / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - size.0 / 2.0 + r.0,
                    p.1.abs() - size.1 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color =
                        self.mix_color(&background, &color, t * 0.5 * (color[3] as f32 / 255.0));
                    mixed_color[3] = (mixed_color[3] as f32 * (color[3] as f32 / 255.0)) as u8;
                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect with a border
    pub fn draw_rounded_rect_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        size: &(f32, f32),
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let center = (
            (rect.0 as f32 + size.0 / 2.0).round(),
            (rect.1 as f32 + size.1 / 2.0 + (rect.3 as f32 - size.1) / 2.0).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - size.0 / 2.0 + r.0,
                    p.1.abs() - size.1 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color =
                        self.mix_color(&background, &color, t * (color[3] as f32 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect with a border
    pub fn draw_rounded_rect_with_border_2(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let hb = border_size / 2.0;
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0 - hb).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0 - hb).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - rect.2 as f32 / 2.0 + hb + r.0,
                    p.1.abs() - rect.3 as f32 / 2.0 + hb + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mut mixed_color =
                        self.mix_color(&background, &color, t * (color[3] as f32 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws the given rectangle
    pub fn draw_square_pattern(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        line_color: &[u8; 4],
        pattern_size: usize,
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                if x % pattern_size == 0 || y % pattern_size == 0 {
                    frame[i..i + 4].copy_from_slice(line_color);
                } else {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }

    /// Draws a text aligned inside a rect
    pub fn draw_text_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        background: &[u8; 4],
        align: TextAlignment,
    ) {
        let mut text_to_use = text.trim_end().to_string().clone();
        text_to_use = text_to_use.replace('\n', "");
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 >= rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(rect.2 as f32),
            max_height: Some(rect.3 as f32),
            horizontal_align: if align == TextAlignment::Left {
                HorizontalAlign::Left
            } else {
                if align == TextAlignment::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                }
            },
            vertical_align: VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text_to_use.as_str(), size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + rect.0 + glyph.x as usize) * 4
                        + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        &background,
                        &color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    /// Blends a text aligned inside a rect and blends it with the existing background
    pub fn blend_text_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        align: TextAlignment,
    ) {
        let mut text_to_use = text.trim_end().to_string().clone();
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 >= rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(rect.2 as f32),
            max_height: Some(rect.3 as f32),
            horizontal_align: if align == TextAlignment::Left {
                HorizontalAlign::Left
            } else {
                if align == TextAlignment::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                }
            },
            vertical_align: VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(&text_to_use.as_str(), size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + rect.0 + glyph.x as usize) * 4
                        + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        &background,
                        &color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    /// Draws the given text
    pub fn draw_text(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        background: &[u8; 4],
    ) {
        if text.is_empty() {
            return;
        }

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text, size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + pos.0 + glyph.x as usize) * 4
                        + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        &background,
                        &color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    /// Returns the size of the given text
    pub fn get_text_size(&self, font: &Font, size: f32, text: &str) -> (usize, usize) {
        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text, size, 0));

        let x = layout.glyphs()[layout.glyphs().len() - 1].x.ceil() as usize
            + layout.glyphs()[layout.glyphs().len() - 1].width
            + 1;
        (x, layout.height() as usize)
    }

    /// Copies rect from the source frame into the dest frame
    pub fn copy_slice(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;
            dest[d..d + rect.2 * 4].copy_from_slice(&source[s..s + rect.2 * 4]);
        }
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    &background,
                    &color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame with a vertical source offset (used by scrolling containers)
    pub fn blend_slice_offset(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        offset: usize,
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = (y + offset) * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    &background,
                    &color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame and honors the safe rect
    pub fn blend_slice_safe(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(isize, isize, usize, usize),
        dest_stride: usize,
        safe_rect: &(usize, usize, usize, usize),
    ) {
        let dest_stride_isize = dest_stride as isize;
        for y in 0..rect.3 as isize {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride_isize * 4;
            let s = y * (rect.2 as isize) * 4;

            // TODO: Make this faster

            if (y + rect.1 as isize) >= safe_rect.1 as isize
                && (y + rect.1 as isize) < (safe_rect.1 + safe_rect.3) as isize
            {
                for x in 0..rect.2 as isize {
                    if (x + rect.0 as isize) >= safe_rect.0 as isize
                        && (x + rect.0 as isize) < (safe_rect.0 + safe_rect.2) as isize
                    {
                        let dd = (d + x * 4) as usize;
                        let ss = (s + x * 4) as usize;

                        let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                        let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                        dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                            &background,
                            &color,
                            (color[3] as f32) / 255.0,
                        ));
                    }
                }
            }
        }
    }

    /// Scale a chunk to the destination size
    pub fn scale_chunk(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        source_frame: &[u8],
        source_size: &(usize, usize),
        blend_factor: f32,
    ) {
        let x_ratio = source_size.0 as f32 / rect.2 as f32;
        let y_ratio = source_size.1 as f32 / rect.3 as f32;

        for sy in 0..rect.3 {
            let y = (sy as f32 * y_ratio) as usize;

            for sx in 0..rect.2 {
                let x = (sx as f32 * x_ratio) as usize;

                let d = (rect.0 + sx) * 4 + (sy + rect.1) * stride * 4;
                let s = x * 4 + y * source_size.0 * 4;

                frame[d..d + 4].copy_from_slice(&[
                    source_frame[s],
                    source_frame[s + 1],
                    source_frame[s + 2],
                    ((source_frame[s + 3] as f32) * blend_factor) as u8,
                ]);
            }
        }
    }

    /// The fill mask for an SDF distance
    fn fill_mask(&self, dist: f32) -> f32 {
        (-dist).clamp(0.0, 1.0)
    }

    /// The border mask for an SDF distance
    fn border_mask(&self, dist: f32, width: f32) -> f32 {
        (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
    }

    /// Smoothstep for f32
    pub fn _smoothstep(&self, e0: f32, e1: f32, x: f32) -> f32 {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
    }

    /// Mixes two colors based on v
    pub fn mix_color(&self, a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        [
            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
            255,
        ]
    }

    // Length of a 2d vector
    pub fn length(&self, v: (f32, f32)) -> f32 {
        ((v.0).powf(2.0) + (v.1).powf(2.0)).sqrt()
    }

    /// Draw a tile
    pub fn draw_tile(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        map: &TileMap,
        stride: usize,
        grid_pos: &(usize, usize),
        scale: f32,
    ) {
        let pixels = &map.pixels;

        let new_size = (
            (map.settings.grid_size as f32 * scale) as usize,
            (map.settings.grid_size as f32 * scale) as usize,
        );

        let g_pos = (
            grid_pos.0 * map.settings.grid_size,
            grid_pos.1 * map.settings.grid_size,
        );

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {
                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                frame[d..d + 4].copy_from_slice(&[
                    pixels[s],
                    pixels[s + 1],
                    pixels[s + 2],
                    pixels[s + 3],
                ]);
            }
        }
    }

    /// Draws a tile mixed with a given color
    pub fn draw_tile_mixed(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        map: &TileMap,
        stride: usize,
        grid_pos: &(usize, usize),
        color: [u8; 4],
        scale: f32,
    ) {
        let pixels = &map.pixels;

        let new_size = (
            (map.settings.grid_size as f32 * scale) as usize,
            (map.settings.grid_size as f32 * scale) as usize,
        );

        let g_pos = (
            grid_pos.0 * map.settings.grid_size,
            grid_pos.1 * map.settings.grid_size,
        );

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {
                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let mixed_color = self.mix_color(
                    &[pixels[s], pixels[s + 1], pixels[s + 2], pixels[s + 3]],
                    &color,
                    0.5,
                );

                frame[d..d + 4].copy_from_slice(&mixed_color);
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        map: &TileMap,
        stride: usize,
        grid_pos: &(usize, usize),
        anim_counter: usize,
        target_size: usize,
    ) {
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let new_size = (
            (map.settings.grid_size as f32 * scale) as usize,
            (map.settings.grid_size as f32 * scale) as usize,
        );

        let mut cg_pos = grid_pos;

        if let Some(tile) = map.get_tile(grid_pos) {
            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len();
                cg_pos = &tile.anim_tiles[index];
            }
        }

        let g_pos = (
            cg_pos.0 * map.settings.grid_size,
            cg_pos.1 * map.settings.grid_size,
        );

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {
                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                if let Some(mask) = &self.mask {
                    if sy + pos.1 >= self.mask_size.1 {
                        return;
                    }

                    if pos.0 + sx >= self.mask_size.0 {
                        continue;
                    }

                    let dmask = pos.0 + sx + (sy + pos.1) * self.mask_size.0;

                    let masked = mask[dmask];

                    if masked > 0.0 {
                        let background = &[frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                        let c = self.mix_color(
                            &background,
                            &[pixels[s], pixels[s + 1], pixels[s + 2], pixels[s + 3]],
                            (pixels[s + 3] as f32) * masked as f32 / 255.0,
                        );
                        frame[d..d + 4].copy_from_slice(&c);
                    }
                } else {
                    let background = &[frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                    let c = self.mix_color(
                        &background,
                        &[pixels[s], pixels[s + 1], pixels[s + 2], pixels[s + 3]],
                        pixels[s + 3] as f32 / 255.0,
                    );
                    frame[d..d + 4].copy_from_slice(&c);
                }
            }
        }
    }

    /// Draws the given region with the given offset into the rectangle
    pub fn draw_region(
        &self,
        frame: &mut [u8],
        region: &GameRegion,
        rect: &(usize, usize, usize, usize),
        offset: &(isize, isize),
        stride: usize,
        tile_size: usize,
        anim_counter: usize,
        asset: &Asset,
        show_overlay: bool,
    ) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                let values;

                if show_overlay == false {
                    values = region.get_value((x + offset.0, y + offset.1));
                } else {
                    values = region.get_value_overlay((x + offset.0, y + offset.1));
                }

                for value in values {
                    let pos = (
                        rect.0 + left_offset + (x as usize) * tile_size,
                        rect.1 + top_offset + (y as usize) * tile_size,
                    );

                    if let Some(map) = asset.get_map_of_id(value.tilemap) {
                        self.draw_animated_tile(
                            frame,
                            &pos,
                            map,
                            stride,
                            &(value.x_off as usize, value.y_off as usize),
                            anim_counter,
                            tile_size,
                        );
                    }
                }
            }
        }
    }

    /// Draws the given region with the given offset into the rectangle (and draws all character behaviors)
    pub fn draw_region_with_behavior(
        &self,
        frame: &mut [u8],
        region: &GameRegion,
        rect: &(usize, usize, usize, usize),
        offset: &(isize, isize),
        stride: usize,
        tile_size: usize,
        anim_counter: usize,
        asset: &Asset,
        context: &ScreenContext,
    ) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                let values = region.get_value((x + offset.0, y + offset.1));
                for value in values {
                    let pos = (
                        rect.0 + left_offset + (x as usize) * tile_size,
                        rect.1 + top_offset + (y as usize) * tile_size,
                    );

                    if let Some(map) = asset.get_map_of_id(value.tilemap) {
                        self.draw_animated_tile(
                            frame,
                            &pos,
                            map,
                            stride,
                            &(value.x_off as usize, value.y_off as usize),
                            anim_counter,
                            tile_size,
                        );
                    }
                }
            }
        }

        let mut draw_tile = |id: Uuid, position: &Position, item: bool| {
            // In the same region ?
            if position.region == region.data.id {
                // Row check
                if position.x >= offset.0 && position.x < offset.0 + x_tiles {
                    // Column check
                    if position.y >= offset.1 && position.y < offset.1 + y_tiles {
                        // Visible
                        let tile: Option<TileData>;

                        if item == false {
                            tile = context.data.get_behavior_default_tile(id);
                        } else {
                            tile = context.data.get_item_default_tile(id);
                        }

                        if let Some(tile) = tile {
                            let pos = (
                                rect.0
                                    + left_offset
                                    + ((position.x - offset.0) as usize) * tile_size,
                                rect.1
                                    + top_offset
                                    + ((position.y - offset.1) as usize) * tile_size,
                            );

                            if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                self.draw_animated_tile(
                                    frame,
                                    &pos,
                                    map,
                                    stride,
                                    &(tile.x_off as usize, tile.y_off as usize),
                                    anim_counter,
                                    tile_size,
                                );
                            }
                        }
                    }
                }
            }
        };

        // Draw Behaviors
        for (id, behavior) in &context.data.behaviors {
            if let Some(position) = context.data.get_behavior_default_position(*id) {
                draw_tile(*id, &position, false);
            }
            if let Some(instances) = &behavior.data.instances {
                for position in instances {
                    draw_tile(*id, &position.position, false);
                }
            }
        }

        // Draw Items
        for (id, behavior) in &context.data.items {
            if let Some(loot) = &behavior.data.loot {
                for position in loot {
                    draw_tile(*id, &position.position, true);
                }
            }
        }
    }

    /// Draws the given region centered at the given center and returns the top left offset into the region
    pub fn draw_region_centered_with_behavior(
        &self,
        frame: &mut [u8],
        region: &GameRegion,
        rect: &(usize, usize, usize, usize),
        center: &(isize, isize),
        scroll_offset: &(isize, isize),
        stride: usize,
        tile_size: usize,
        anim_counter: usize,
        asset: &Asset,
        context: &ScreenContext,
    ) -> (isize, isize) {
        let mut left_offset = 0;
        let mut top_offset = 0;

        let mut x_tiles = (rect.2 / tile_size) as isize;
        let mut y_tiles = (rect.3 / tile_size) as isize;

        if self.mask.is_none() {
            left_offset = (rect.2 % tile_size) / 2;
            top_offset = (rect.3 % tile_size) / 2;
        } else {
            x_tiles += 1;
            y_tiles += 1;
        }

        let mut offset = center.clone();

        offset.0 -= scroll_offset.0;
        offset.1 -= scroll_offset.1;

        offset.0 -= x_tiles / 2;
        offset.1 -= y_tiles / 2;

        if self.mask.is_some() {
            offset.0 += 1;
            offset.1 += 1;
        }

        // Draw Environment
        for y in 0..y_tiles {
            for x in 0..x_tiles {
                let p = (x + offset.0, y + offset.1);
                let values = region.get_value(p);

                let pos = (
                    rect.0 + left_offset + (x as usize) * tile_size,
                    rect.1 + top_offset + (y as usize) * tile_size,
                );
                for value in values {
                    if let Some(map) = asset.get_map_of_id(value.tilemap) {
                        self.draw_animated_tile(
                            frame,
                            &pos,
                            map,
                            stride,
                            &(value.x_off as usize, value.y_off as usize),
                            anim_counter,
                            tile_size,
                        );
                    }
                }

                // Draw an outline ?
                if p.0 == center.0 && p.1 == center.1 {
                    self.draw_rect_outline(
                        frame,
                        &(pos.0, pos.1, tile_size, tile_size),
                        stride,
                        context.color_red,
                    );
                }
            }
        }

        // Draw Behaviors
        for (id, _behavior) in &context.data.behaviors {
            if let Some(position) = context.data.get_behavior_default_position(*id) {
                // In the same region ?
                if position.region == region.data.id {
                    // Row check
                    if position.x >= offset.0 && position.y < offset.0 + x_tiles {
                        // Column check
                        if position.y >= offset.1 && position.y < offset.1 + y_tiles {
                            // Visible
                            if let Some(tile) = context.data.get_behavior_default_tile(*id) {
                                let pos = (
                                    rect.0
                                        + left_offset
                                        + ((position.x - offset.0) as usize) * tile_size,
                                    rect.1
                                        + top_offset
                                        + ((position.y - offset.1) as usize) * tile_size,
                                );

                                if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                                    self.draw_animated_tile(
                                        frame,
                                        &pos,
                                        map,
                                        stride,
                                        &(tile.x_off as usize, tile.y_off as usize),
                                        anim_counter,
                                        tile_size,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        offset
    }

    /*
    /// Draws the given region centered at the given center and returns the top left offset into the region
    pub fn draw_region_centered_with_instances(&self, frame: &mut [u8], region: &GameRegion, rect: &(usize, usize, usize, usize), index_to_center: usize, stride: usize, tile_size: usize, anim_counter: usize, asset: &Asset, context: &ScreenContext) -> (isize, isize) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        let mut center = (0, 0);
        if let Some(position) = context.data.instances[index_to_center].position {
            center.0 = position.1;
            center.1 = position.2;
        } else {
            return region.data.min_pos.clone();
        }
        let mut offset = center.clone();

        offset.0 -= x_tiles / 2;
        offset.1 -= y_tiles / 2;

        // Draw Environment
        for y in 0..y_tiles {
            for x in 0..x_tiles {

                let values = region.get_value((x + offset.0, y + offset.1));
                for value in values {
                    let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                    let map = asset.get_map_of_id(value.0);
                    self.draw_animated_tile(frame, &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                }
            }
        }

        for index in 0..context.data.instances.len() {

            if context.data.instances[index].state == BehaviorInstanceState::Killed || context.data.instances[index].state == BehaviorInstanceState::Purged {
                continue;
            }

            if let Some(position) = context.data.instances[index].position {
                if let Some(tile) = context.data.instances[index].tile {
                    // In the same region ?
                    if position.0 == region.data.id {

                        // Row check
                        if position.1 >= offset.0 && position.1 < offset.0 + x_tiles {
                            // Column check
                            if position.2 >= offset.1 && position.2 < offset.1 + y_tiles {
                                // Visible
                                let pos = (rect.0 + left_offset + ((position.1 - offset.0) as usize) * tile_size, rect.1 + top_offset + ((position.2 - offset.1) as usize) * tile_size);

                                let map = asset.get_map_of_id(tile.0);
                                self.draw_animated_tile(frame, &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                            }
                        }
                    }
                }
            }
        }

        offset
    }

    /// Draws the given region with the given offset into the rectangle
    pub fn draw_region_with_instances(&self, frame: &mut [u8], region: &GameRegion, rect: &(usize, usize, usize, usize), offset: &(isize, isize), stride: usize, tile_size: usize, anim_counter: usize, asset: &Asset, context: &ScreenContext) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                let values = region.get_value((x + offset.0, y + offset.1));

                for value in values {
                    let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                    let map = asset.get_map_of_id(value.0);
                    self.draw_animated_tile(frame, &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                }
            }
        }

        for index in 0..context.data.instances.len() {

            if context.data.instances[index].state == BehaviorInstanceState::Killed || context.data.instances[index].state == BehaviorInstanceState::Purged {
                continue;
            }

            if let Some(position) = context.data.instances[index].position {
                if let Some(tile) = context.data.instances[index].tile {
                    // In the same region ?
                    if position.0 == region.data.id {

                        // Row check
                        if position.1 >= offset.0 && position.1 < offset.0 + x_tiles {
                            // Column check
                            if position.2 >= offset.1 && position.2 < offset.1 + y_tiles {
                                // Visible
                                let pos = (rect.0 + left_offset + ((position.1 - offset.0) as usize) * tile_size, rect.1 + top_offset + ((position.2 - offset.1) as usize) * tile_size);

                                let map = asset.get_map_of_id(tile.0);
                                self.draw_animated_tile(frame, &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                            }
                        }
                    }
                }
            }
        }
    }*/

    /// Draw hover help
    pub fn _draw_hover_help(
        &self,
        frame: &mut [u8],
        pos: (usize, usize),
        font: &Font,
        title: Option<String>,
        text: String,
        safe_rect: (usize, usize, usize, usize),
    ) {
        let mut rect = (pos.0, pos.1, 300, 200);
        let stride = safe_rect.2;

        let mut title_space = 10;
        if title.is_some() {
            title_space = 20;
        };
        let font_size_title = 16.0;
        let font_size_text = 14.0;

        let mut vert_size = 30 + title_space;
        if title.is_none() {
            vert_size -= 20;
        };

        let background = [40, 40, 40, 255];
        let border_color = [128, 128, 128, 255];
        let title_color = [255, 255, 255, 255];
        let text_color = [240, 240, 240, 255];

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(rect.2 as f32 - 20.0),
            max_height: None, //Some(rect.3 as f32 - vert_size as f32),
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text.as_str(), font_size_text, 0));

        rect.3 = layout.height().ceil() as usize + vert_size;

        if rect.0 + rect.2 > safe_rect.2 {
            rect.0 -= rect.0 + rect.2 - safe_rect.2 + 10;
        }

        if rect.1 + rect.3 > safe_rect.3 {
            rect.1 -= rect.1 + rect.3 - safe_rect.3 + 10;
        }

        self.draw_rect(frame, &rect, stride, &background);
        self.draw_rect_outline(frame, &rect, stride, border_color);

        if let Some(title) = title {
            self.draw_text(
                frame,
                &(rect.0 + 10, rect.1 + 10),
                stride,
                font,
                font_size_title,
                title.as_str(),
                &title_color,
                &background,
            );
            rect.1 += title_space;
            rect.1 += 10;
        }

        rect.0 += 10;
        rect.1 += 10;

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + rect.0 + glyph.x as usize) * 4
                        + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        &background,
                        &text_color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }
}
