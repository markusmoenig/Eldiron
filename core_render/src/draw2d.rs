
use fontdue::layout::{ Layout, LayoutSettings, CoordinateSystem, TextStyle, VerticalAlign, HorizontalAlign };
use fontdue::Font;

use colori::{RgbColor, HslColor};

use core_shared::prelude::*;

#[derive(PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    Right
}

pub struct Draw2D {
    pub scissor             : Option<(usize, usize, usize, usize)>
}

impl Draw2D {

    /// Draws the mask
    pub fn blend_mask(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, mask_frame: &[u8], mask_size: &(usize, usize), color: &[u8; 4]) {
        for y in 0..mask_size.1 {
            for x in 0..mask_size.0 {
                let i = (x+rect.0) * 4 + (y+rect.1) * stride * 4;
                let m = mask_frame[x + y * mask_size.0];
                let c : [u8;4] = [color[0], color[1], color[2], m];

                let background = &[frame[i], frame[i+1], frame[i+2], frame[i+3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &c, m as f64 / 255.0));
            }
        }
    }

    /// Draws the given rectangle
    pub fn draw_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4]) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(color);
            }
        }
    }

    /// Draws the given rectangle within the scissor
    pub fn draw_safe_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4]) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                if self.is_safe(x, y) {
                    let i = x * 4 + y * stride * 4;
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }

    /// Blend the given rectangle
    pub fn blend_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4]) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let background = &[frame[i], frame[i+1], frame[i+2], frame[i+3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, color[3] as f64 / 255.0));
            }
        }
    }

    /// Draws the outline of a given rectangle
    pub fn draw_rect_outline(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: [u8; 4]) {

        let y = rect.1;
        for x in rect.0..rect.0+rect.2 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = x * 4 + (y + rect.3- 1) * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }

        let x = rect.0;
        for y in rect.1..rect.1+rect.3 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = (x + rect.2 - 1) * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }
    }

    /// Draws a circle
    pub fn draw_circle(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], radius: f64) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f64 - center.0).powf(2.0) + (y as f64 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 0.0 {
                    let t = self.fill_mask(d);
                    //let t = self.smoothstep(0.0, -2.0, r);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mixed_color = self.mix_color(&background, &color, t);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a circle with a border of a given size
    pub fn _draw_circle_with_border(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], radius: f64, border_color: &[u8; 4], border_size: f64) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f64 - center.0).powf(2.0) + (y as f64 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t);

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect
    pub fn draw_rounded_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, size: &(f64, f64), color: &[u8; 4], rounding: &(f64, f64, f64, f64)) {
        let center = (rect.0 as f64 + size.0 / 2.0, rect.1 as f64 + size.1 / 2.0 + (rect.3 as f64 - size.1) / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f64 - center.0, y as f64 - center.1);
                let mut r : (f64, f64);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q : (f64, f64) = (p.0.abs() - size.0 / 2.0 + r.0, p.1.abs() - size.1 / 2.0 + r.0);
                let d = f64::min(f64::max(q.0, q.1), 0.0) + self.length((f64::max(q.0, 0.0), f64::max(q.1, 0.0))) - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t * (color[3] as f64 / 255.0));
                    mixed_color[3] = (mixed_color[3] as f64 * (color[3] as f64 / 255.0)) as u8;
                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect with a border
    pub fn draw_rounded_rect_with_border(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, size: &(f64, f64), color: &[u8; 4], rounding: &(f64, f64, f64, f64), border_color: &[u8; 4], border_size: f64) {
        let center = ((rect.0 as f64 + size.0 / 2.0).round(), (rect.1 as f64 + size.1 / 2.0 + (rect.3 as f64 - size.1) / 2.0).round());
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f64 - center.0, y as f64 - center.1);
                let mut r : (f64, f64);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q : (f64, f64) = (p.0.abs() - size.0 / 2.0 + r.0, p.1.abs() - size.1 / 2.0 + r.0);
                let d = f64::min(f64::max(q.0, q.1), 0.0) + self.length((f64::max(q.0, 0.0), f64::max(q.1, 0.0))) - r.0;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t * (color[3] as f64 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws the given rectangle
    pub fn draw_square_pattern(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], line_color: &[u8; 4], pattern_size: usize) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
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
    pub fn draw_text_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4], background: &[u8;4], align: TextAlignment) {

        let mut text_to_use = text.trim_end().to_string().clone();
        if text_to_use.trim_end().is_empty() { return; }

        text_to_use = text_to_use.lines().next().unwrap().to_owned();

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 > rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
            if text_to_use.len() == 0 {
                return;
            }
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width : Some(rect.2 as f32),
            max_height : Some(rect.3 as f32),
            horizontal_align : if align ==  TextAlignment::Left { HorizontalAlign::Left } else { if align == TextAlignment::Right {HorizontalAlign::Right} else { HorizontalAlign::Center } },
            vertical_align : VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text_to_use.as_str(), size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x+rect.0+glyph.x as usize) * 4 + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, m as f64 / 255.0));
                }
            }
        }
    }

    /// Blends a text aligned inside a rect and blends it with the existing background
    pub fn blend_text_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4], align: TextAlignment) {

        let mut text_to_use = text.trim_end().to_string().clone();
        if text_to_use.trim_end().is_empty() { return; }

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 > rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
            if text_to_use.len() == 0 {
                return;
            }
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width : Some(rect.2 as f32),
            max_height : Some(rect.3 as f32),
            horizontal_align : if align ==  TextAlignment::Left { HorizontalAlign::Left } else { if align == TextAlignment::Right {HorizontalAlign::Right} else { HorizontalAlign::Center } },
            vertical_align : VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(&text_to_use.as_str(), size, 0));

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x+rect.0+glyph.x as usize) * 4 + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i+1], frame[i+2], frame[i+3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, m as f64 / 255.0));
                }
            }
        }
    }

    /// Draws the given text
    pub fn draw_text(&self,  frame: &mut [u8], pos: &(usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4], background: &[u8; 4]) {
        if text.is_empty() { return; }

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
                    let i = (x+pos.0+glyph.x as usize) * 4 + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, m as f64 / 255.0));
                }
            }
        }
    }

    /// Draws the given text
    pub fn blend_text(&self,  frame: &mut [u8], pos: &(usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4]) {
        if text.is_empty() { return; }

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
                    let i = (x+pos.0+glyph.x as usize) * 4 + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i+1], frame[i+2], frame[i+3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, m as f64 / 255.0));
                }
            }
        }
    }

    /// Draws the given text
    pub fn blend_text_safe(&self,  frame: &mut [u8], pos: &(usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4], safe_rect: (usize, usize, usize, usize)) {
        if text.is_empty() { return; }

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

                if y < safe_rect.1 && y >= safe_rect.1 + safe_rect.3 {
                    continue;
                }

                for x in 0..metrics.width {

                    if x < safe_rect.0 && x >= safe_rect.0 + safe_rect.2 {
                        continue;
                    }

                    let i = (x+pos.0+glyph.x as usize) * 4 + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i+1], frame[i+2], frame[i+3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(&background, &color, m as f64 / 255.0));
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

        let x = layout.glyphs()[layout.glyphs().len()-1].x as usize + layout.glyphs()[layout.glyphs().len()-1].width;
        (x, layout.height() as usize)
    }

    /// Copies rect from the source frame into the dest frame
    pub fn copy_slice(&self, dest: &mut [u8], source: &[u8], rect: &(usize, usize, usize, usize), dest_stride: usize) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;
            dest[d..d + rect.2 * 4].copy_from_slice(&source[s..s + rect.2 * 4]);
        }
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice(&self, dest: &mut [u8], source: &[u8], rect: &(usize, usize, usize, usize), dest_stride: usize) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {

                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd+1], dest[dd+2], dest[dd+3]];
                let color = &[source[ss], source[ss+1], source[ss+2], source[ss+3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(&background, &color, (color[3] as f64) / 255.0));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame and honors the safe rect
    pub fn blend_slice_safe(&self, dest: &mut [u8], source: &[u8], rect: &(isize, isize, usize, usize), dest_stride: usize, safe_rect: &(usize, usize, usize, usize)) {
        let dest_stride_isize = dest_stride as isize;
        for y in 0..rect.3 as isize {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride_isize * 4;
            let s = y * (rect.2 as isize) * 4;

            // TODO: Make this faster

            if (y + rect.1 as isize) >= safe_rect.1 as isize && (y + rect.1 as isize) < (safe_rect.1 + safe_rect.3) as isize {
                for x in 0..rect.2 as isize {

                    if (x + rect.0 as isize) >= safe_rect.0 as isize && (x + rect.0 as isize) < (safe_rect.0 + safe_rect.2) as isize {
                        let dd = (d + x * 4) as usize;
                        let ss = (s + x * 4) as usize;

                        let background = &[dest[dd], dest[dd+1], dest[dd+2], dest[dd+3]];
                        let color = &[source[ss], source[ss+1], source[ss+2], source[ss+3]];
                        dest[dd..dd + 4].copy_from_slice(&self.mix_color(&background, &color, (color[3] as f64) / 255.0));
                    }
                }
            }
        }
    }

    /// The fill mask for an SDF distance
    fn fill_mask(&self, dist : f64) -> f64 {
        (-dist).clamp(0.0, 1.0)
    }

    /// The border mask for an SDF distance
    fn border_mask(&self, dist : f64, width: f64) -> f64 {
       (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
    }

    /// Smoothstep for f64
    pub fn _smoothstep(&self, e0: f64, e1: f64, x: f64) -> f64 {
        let t = ((x - e0) / (e1 - e0)). clamp(0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
    }

    /// Mixes two colors based on v
    pub fn mix_color(&self, a: &[u8;4], b: &[u8;4], v: f64) -> [u8; 4] {
        [   (((1.0 - v) * (a[0] as f64 / 255.0) + b[0] as f64 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f64 / 255.0) + b[1] as f64 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f64 / 255.0) + b[2] as f64 / 255.0 * v) * 255.0) as u8,
        255]
    }

    // Length of a 2d vector
    pub fn length(&self, v: (f64, f64)) -> f64 {
        ((v.0).powf(2.0) + (v.1).powf(2.0)).sqrt()
    }

    /// Draw a tile
    pub fn draw_tile(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), scale: f32) {
        let pixels = &map.pixels;

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let g_pos = (grid_pos.0 * map.settings.grid_size, grid_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];
                let c = self.mix_color(&background, &[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]], pixels[s+3] as f64 / 255.0);
                frame[d..d + 4].copy_from_slice(&c);
            }
        }
    }

    /// Draws a tile mixed with a given color
    pub fn draw_tile_mixed(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), color: [u8; 4], scale: f32) {
        let pixels = &map.pixels;

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let g_pos = (grid_pos.0 * map.settings.grid_size, grid_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let mixed_color = self.mix_color(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]], &color, 0.5);

                frame[d..d + 4].copy_from_slice(&mixed_color);
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), anim_counter: usize, target_size: usize) {
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let mut cg_pos = grid_pos;
        if let Some(tile) = map.get_tile(grid_pos) {
            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len();
                cg_pos = &tile.anim_tiles[index];
            }
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];
                let c = self.mix_color(&background, &[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]], pixels[s+3] as f64 / 255.0);

                frame[d..d + 4].copy_from_slice(&c);
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile_with_blended_color(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), anim_counter: usize, target_size: usize, blend_color: &[u8; 4], blend: f64) {
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let mut cg_pos = grid_pos;

        if let Some(tile) = map.get_tile(grid_pos) {
            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len();
                cg_pos = &tile.anim_tiles[index];
            }
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                if self.is_safe(pos.0 + sx, pos.1 + sy) {
                    let x = (sx as f32 / scale) as usize;

                    let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                    let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                    let mixed_color = self.mix_color(blend_color, &[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]], blend);

                    let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];
                    let c = self.mix_color(&background, &mixed_color, pixels[s+3] as f64 / 255.0);

                    frame[d..d + 4].copy_from_slice(&c);
                }
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile_sat(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), anim_counter: usize, target_size: usize, mult_color: [u8; 4]) {
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let color_hsl: HslColor = RgbColor(mult_color[0], mult_color[1], mult_color[2]).into();

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let mut cg_pos = grid_pos;

        if let Some(tile) = map.get_tile(grid_pos) {

            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len();
                cg_pos = &tile.anim_tiles[index];
            }
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];

                //let hsl: HslColor = RgbColor(pixels[s], pixels[s+1], pixels[s+2]).into();

                let mut color = color_hsl.clone();
                color.2 = pixels[s] as f32 / 255.0;
                let rgb: RgbColor = color.into_rgb();

                //let p = [pixels[s] * mult_color[0], pixels[s+1] * mult_color[1], pixels[s+2] * mult_color[2], pixels[s+3]];
                let p = [rgb.0, rgb.1, rgb.2, pixels[s+3]];

                let c = self.mix_color(&background, &p, pixels[s+3] as f64 / 255.0);

                frame[d..d + 4].copy_from_slice(&c);
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile_mult(&self,  frame: &mut [u8], pos: &(usize, usize), map: &TileMap, stride: usize, grid_pos: &(usize, usize), anim_counter: usize, target_size: usize, mult_color: [u8; 4]) {
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let new_size = ((map.settings.grid_size as f32 * scale) as usize, (map.settings.grid_size as f32 * scale) as usize);

        let mut cg_pos = grid_pos;

        if let Some(tile) = map.get_tile(grid_pos) {

            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len();
                cg_pos = &tile.anim_tiles[index];
            }
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        fn to_float(b: u8) -> f32 {
            b as f32 / 255.0
        }

        fn to_u8(b: f32) -> u8 {
            (b * 255.0) as u8
        }

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];

                //let hsl: HslColor = RgbColor(pixels[s], pixels[s+1], pixels[s+2]).into();

                let p = [to_u8(to_float(pixels[s]) * to_float(mult_color[0])), to_u8(to_float(pixels[s+1]) * to_float(mult_color[1])), to_u8(to_float(pixels[s+2]) * to_float(mult_color[2])), pixels[s+3]];

                let c = self.mix_color(&background, &p, pixels[s+3] as f64 / 255.0);

                frame[d..d + 4].copy_from_slice(&c);
            }
        }
    }

    /// Safe to write a pixel
    fn is_safe(&self, x: usize, y: usize) -> bool {
        if let Some(s) = &self.scissor {
            if x < s.0 || x >= s.0 + s.2 || y < s.1 || y >= s.1 + s.3 {
                return false;
            }
        }
        true
    }

    /// Draw hover help
    pub fn create_buffer_for_message(&self, max_width: usize, font: &Font, font_size: f32, message: &MessageData, color: &[u8; 4]) -> (usize, usize, Vec<u8>) {

        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width : Some(max_width as f32),
            max_height : None,
            horizontal_align : HorizontalAlign::Left,
            vertical_align : VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(message.message.as_str(), font_size, 0));

        let width = max_width;
        let height = layout.height().ceil() as usize;
        let mut buffer : Vec<u8> = vec![0; width * height * 4];

        let frame = &mut buffer[..];

        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x+glyph.x as usize) * 4 + (y + glyph.y as usize) * max_width * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], m]);
                }
            }
        }

        if let Some(right) = &message.right {
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                max_width : Some(max_width as f32),
                max_height : None,
                horizontal_align : HorizontalAlign::Right,
                vertical_align : VerticalAlign::Middle,
                ..LayoutSettings::default()
            });
            layout.append(fonts, &TextStyle::new(right.as_str(), font_size, 0));

            let frame = &mut buffer[..];
            for glyph in layout.glyphs() {
                let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

                for y in 0..metrics.height {
                    for x in 0..metrics.width {
                        let i = (x+glyph.x as usize) * 4 + (y + glyph.y as usize) * max_width * 4;
                        let m = alphamap[x + y * metrics.width];

                        frame[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], m]);
                    }
                }
            }
        }

        (width, height, buffer)
    }

    /// Draw hover help
    pub fn create_buffer_for_multi_choice(&self, max_width: usize, font: &Font, font_size: f32, mcd: &MultiChoiceData, color: &[u8; 4]) -> (usize, usize, Vec<u8>) {

        let fonts = &[font];

        let mut text = "".to_string();
        if mcd.header.len() > 0 {
            text = mcd.header.clone();
            text += "\n";
        }
        text += mcd.answer.as_str();
        text += ". ";
        text += mcd.text.as_str();
        if let Some(amount) = &mcd.item_amount {
            text += format!(" ({})", amount).as_str();
        }
        if let Some(price) = &mcd.item_price {
            text += format!(" : {}G", price).as_str();
        }

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width : Some(max_width as f32),
            max_height : None,
            horizontal_align : HorizontalAlign::Left,
            vertical_align : VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        layout.append(fonts, &TextStyle::new(text.as_str(), font_size, 0));

        let width = max_width;
        let height = layout.height().ceil() as usize;
        let mut buffer : Vec<u8> = vec![0; width * height * 4];

        let frame = &mut buffer[..];

        for glyph in layout.glyphs() {
            if glyph.char_data.rasterize() == false { continue; }
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x+glyph.x as usize) * 4 + (y + glyph.y as usize) * max_width * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], m]);
                }
            }
        }
        (width, height, buffer)
    }
}