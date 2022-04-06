
use rusttype::{point, Font, Scale};

use server::asset::TileMap;
use server::asset::Asset;
use server::gamedata::area::GameArea;

use super::context::ScreenContext;

#[derive(PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    //Right
}

pub struct Draw2D {
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
                    let mixed_color = self.mix_color(&background, &color, t);

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

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 > rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        if align == TextAlignment::Left {
            let y =  rect.1 + (rect.3 - text_size.1) / 2;
            self.draw_text(frame, &(rect.0, y), stride, font, size, text_to_use.as_str(), color, background);
        } else
        if align == TextAlignment::Center {
            let x =  rect.0 + (rect.2 - text_size.0) / 2;
            let y =  rect.1 + (rect.3 - text_size.1) / 2;
            self.draw_text(frame, &(x, y), stride, font, size, text_to_use.as_str(), color, background);
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
        }

        if add_trail {
            text_to_use = text_to_use + "...";
        }

        if align == TextAlignment::Left {
            let y =  rect.1 + (rect.3 - text_size.1) / 2;
            self.blend_text(frame, &(rect.0, y), stride, font, size, text_to_use.as_str(), color);
        } else
        if align == TextAlignment::Center {
            let text_size = self.get_text_size(font, size, text_to_use.as_str());
            let x =  rect.0 + (rect.2 - text_size.0) / 2;
            let y =  rect.1 + (rect.3 - text_size.1) / 2;
            self.blend_text(frame, &(x, y), stride, font, size, text, color);
        }
    }

    /// Draws the given text
    pub fn draw_text(&self,  frame: &mut [u8], pos: &(usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4], background: &[u8; 4]) {
        if text.is_empty() { return; }

        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout( text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let d = (x as usize + bounding_box.min.x as usize + pos.0) * 4 + ((y + bounding_box.min.y as u32) as usize + pos.1) * (stride as usize) * 4;
                    if v > 0.0 {
                        //frame[d..d + 4].copy_from_slice(&self.mix_color(&background, &color, self.smoothstep(0.0, 1.0, v as f64)));
                        frame[d..d + 4].copy_from_slice(&self.mix_color(&background, &color, v as f64));
                    }
                });
            }
        }
    }

    /// Draws the given text and blends it with the existing background
    pub fn blend_text(&self,  frame: &mut [u8], pos: &(usize, usize), stride: usize, font: &Font, size: f32, text: &str, color: &[u8; 4]) {
        if text.is_empty() { return; }

        let scale = Scale::uniform(size);

        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout( text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let d = (x as usize + bounding_box.min.x as usize + pos.0) * 4 + ((y + bounding_box.min.y as u32) as usize + pos.1) * (stride as usize) * 4;
                    if v > 0.0 {
                        let background = &[frame[d], frame[d+1], frame[d+2], frame[d+3]];
                        frame[d..d + 4].copy_from_slice(&self.mix_color(&background, &color, v as f64));
                    }
                });
            }
        }
    }

    /// Returns the size of the given text
    pub fn get_text_size(&self, font: &Font, size: f32, text: &str) -> (usize, usize) {

        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout(text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let glyphs_width = {
            let min_x = glyphs
                .first()
                .map(|g| g.pixel_bounding_box().unwrap_or(rusttype::Rect{min: rusttype::Point{x: 0, y: 0}, max: rusttype::Point{x: 0, y: 0}}).min.x)
                .unwrap();
            let max_x = glyphs
                .last()
                .map(|g| g.pixel_bounding_box().unwrap_or(rusttype::Rect{min: rusttype::Point{x: 0, y: 0}, max: rusttype::Point{x: 0, y: 0}}).max.x)
                .unwrap();
            (max_x - min_x) as u32
        };

        (glyphs_width as usize, glyphs_height as usize)
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

                frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
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

        let tile = map.get_tile(grid_pos);

        let mut cg_pos = grid_pos;

        if tile.anim_tiles.len() > 0 {
            let index = anim_counter % tile.anim_tiles.len();
            cg_pos = &tile.anim_tiles[index];
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as usize;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as usize;

                let d = pos.0 * 4 + sx * 4 + (sy + pos.1) * stride * 4;
                let s = (x + g_pos.0) * 4 + (y + g_pos.1) * map.width * 4;

                frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
            }
        }
    }

    /// Draws the given area with the given offset into the rectangle
    pub fn draw_area(&self, frame: &mut [u8], area: &GameArea, rect: &(usize, usize, usize, usize), offset: &(isize, isize), stride: usize, tile_size: usize, anim_counter: usize, asset: &Asset) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                if let Some(value) = area.get_value((x + offset.0, y + offset.1)) {
                    let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                    let map = asset.get_map_of_id(value.0);
                    self.draw_animated_tile(frame, &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                }
            }
        }
    }

    /// Draws the given area centered at the given center and returns the top left offset into the area
    pub fn draw_area_centered_with_behavior(&self, frame: &mut [u8], area: &GameArea, rect: &(usize, usize, usize, usize), center: &(isize, isize), stride: usize, tile_size: usize, anim_counter: usize, asset: &Asset, context: &ScreenContext) -> (isize, isize) {
        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        let mut offset = center.clone();

        offset.0 -= x_tiles / 2;
        offset.1 -= y_tiles / 2;

        // Draw Environment
        for y in 0..y_tiles {
            for x in 0..x_tiles {
                if let Some(value) = area.get_value((x + offset.0, y + offset.1)) {
                    let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                    let map = asset.get_map_of_id(value.0);
                    self.draw_animated_tile(frame, &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                }
            }
        }

        // Draw Behaviors
        for (id, _behavior) in &context.data.behaviors {
            if let Some(position) = context.data.get_behavior_default_position(*id) {
                // In the same area ?
                if position.0 == area.data.id {

                    // Row check
                    if position.1 >= offset.0 && position.1 < offset.0 + x_tiles {
                        // Column check
                        if position.2 >= offset.1 && position.2 < offset.1 + y_tiles {
                            // Visible
                            if let Some(tile) = context.data.get_behavior_default_tile(*id) {

                                let pos = (rect.0 + left_offset + ((position.1 - offset.0) as usize) * tile_size, rect.1 + top_offset + ((position.2 - offset.1) as usize) * tile_size);

                                let map = asset.get_map_of_id(tile.0);
                                self.draw_animated_tile(frame, &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                            }
                        }
                    }
                }
            }
        }

        // Draw center mark

        self.draw_rect_outline(frame, &(rect.0 + left_offset + x_tiles as usize / 2 * tile_size, rect.1 + top_offset + y_tiles as usize / 2 * tile_size, tile_size, tile_size), stride, context.color_red);

        offset
    }

        /// Draws the given area centered at the given center and returns the top left offset into the area
        pub fn draw_area_centered_with_instances(&self, frame: &mut [u8], area: &GameArea, rect: &(usize, usize, usize, usize), index_to_center: usize, stride: usize, tile_size: usize, anim_counter: usize, asset: &Asset, context: &ScreenContext) -> (isize, isize) {
            let left_offset = (rect.2 % tile_size) / 2;
            let top_offset = (rect.3 % tile_size) / 2;

            let x_tiles = (rect.2 / tile_size) as isize;
            let y_tiles = (rect.3 / tile_size) as isize;

            let mut center = (0, 0);
            if let Some(position) = context.data.instances[index_to_center].position {
                center.0 = position.1;
                center.1 = position.2;
            } else {
                return area.data.min_pos.clone();
            }
            let mut offset = center.clone();

            offset.0 -= x_tiles / 2;
            offset.1 -= y_tiles / 2;

            // Draw Environment
            for y in 0..y_tiles {
                for x in 0..x_tiles {
                    if let Some(value) = area.get_value((x + offset.0, y + offset.1)) {
                        let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                        let map = asset.get_map_of_id(value.0);
                        self.draw_animated_tile(frame, &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                    }
                }
            }

            for index in 0..context.data.instances.len() {

                if let Some(position) = context.data.instances[index].position {
                    if let Some(tile) = context.data.instances[index].tile {
                        // In the same area ?
                        if position.0 == area.data.id {

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
}