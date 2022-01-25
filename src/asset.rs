
pub mod tileset;

use rusttype::{point, Font, Scale};

use crate::prelude::*;
use tileset::*;

pub struct Asset<'a> {
    tileset                 : TileSet,
    //gohu_font_11            : Font<'a>,
    gohu_font_14            : Font<'a>,
}

impl Asset<'_>  {
    
    pub fn new() -> Self where Self: Sized {
        Self {
            tileset         : tileset::TileSet::new(),
            //gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-14.ttf") as &[u8]).expect("Error constructing Font")
        }
    }

    /// Draws the given rectangle
    pub fn draw_rect(&self, frame: &mut [u8], rect: &[usize; 4], color: [u8; 4]) {
        for y in rect[1]..rect[1]+rect[3] {
            for x in rect[0]..rect[0]+rect[2] {

                let i = x * 4 + y * (WIDTH as usize) * 4;

                frame[i..i + 4].copy_from_slice(&color);
            }
        }
    }

    /// Draws the given tilemap
    pub fn draw_tilemap(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap) {
        let pixels = &map.pixels;

        for y in 0..map.height {
            for x in 0..map.width {

                let d = (x as usize + pos[0] as usize) * 4 + (y as usize + pos[1] as usize) * (WIDTH as usize) * 4;
                let s = (x as usize) * 4 + (y as usize) * (map.width as usize) * 4;

                frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
            }
        }
        /*
        let mut rgba = [0,0,0,0];
        if x < 256 && y < 300 {
            let off = x * 4 + y * 256 * 4;
            rgba = [u4b[off], u4b[off + 1], u4b[off + 2], 255];
        }*/
    }

    pub fn draw_text(&self,  frame: &mut [u8], pos: &(u32, u32),  text: &String, color: [u8; 3]) {

        let font = &self.gohu_font_14;

        // The font size to use
        let scale = Scale::uniform(16.0);

        // The text to render
        //let text = "This is RustType rendered into a png!";

        // Use a dark red colour
        //let colour = (150, 0, 0);

        let v_metrics = font.v_metrics(scale);

        // layout the glyphs in a line with 20 pixels padding
        let glyphs: Vec<_> = font
            .layout(text.as_str(), scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        // work out the layout size
        
        /*
        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let glyphs_width = {
            let min_x = glyphs
                .first()
                .map(|g| g.pixel_bounding_box().unwrap().min.x)
                .unwrap();
            let max_x = glyphs
                .last()
                .map(|g| g.pixel_bounding_box().unwrap().max.x)
                .unwrap();
            (max_x - min_x) as u32
        };*/

        // Loop through the glyphs in the text, positing each one on a line
        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                // Draw the glyph into the image per-pixel by using the draw closure
                glyph.draw(|x, y, v| {

                    let d = ((x + bounding_box.min.x as u32) as usize + pos.0 as usize) * 4 + ((y + bounding_box.min.y as u32) as usize + pos.1 as usize) * (WIDTH as usize) * 4;

                    frame[d..d + 4].copy_from_slice(&[color[0], color[1], color[2], (v * 255.0) as u8]);
                });
            }
        }
    }
}
