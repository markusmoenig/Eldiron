mod tileset;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::prelude::*;
//use crate::editor::*;

use rusttype::{point, Font, Scale};

/// Gets the current time in milliseconds
pub fn get_time() -> u128 {
    let stop = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");    
    stop.as_millis()
}

/// Which window do we show currently
enum GameWindow {
    Game,
    Editor
}

/// The main Game struct
pub struct Game<'a> {
    window                  : GameWindow,
    tileset                 : tileset::TileSet,
    gohu_font_11            : Font<'a>,
    //gohu_font_14            : Font<'a>,

}

impl Game<'_>  {
    
    pub fn new() -> Self {
        Self {
            window          : GameWindow::Editor,
            tileset         : tileset::TileSet::new(),
            gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            //gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-14.ttf") as &[u8]).expect("Error constructing Font")
        }
    }

    /// Update the game state
    pub fn update(&mut self) {
    }

    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    pub fn draw(&self, frame: &mut [u8]) {

        // Draw the current window
        match self.window {
            GameWindow::Game => println!("{}", 1),
            GameWindow::Editor => println!("{}", 1),
        }

        let start = get_time();

        /*
        let u4b = &self.tile_set.ts1;

        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WINDOW_WIDTH as usize) as usize;
            let y = (i / WINDOW_HEIGHT as usize) as usize;

            /* 
            let inside_the_box = x >= self.box_x
                && x < self.box_x + BOX_SIZE
                && y >= self.box_y
                && y < self.box_y + BOX_SIZE;

            let rgba = if inside_the_box {
                [0x5e, 0x48, 0xe8, 0xff]
            } else {
                [0x48, 0xb2, 0xe8, 0xff]
            };*/
            
            let mut rgba = [0,0,0,0];
            if x < 256 && y < 300 {
                let off = x * 4 + y * 256 * 4;
                rgba = [u4b[off], u4b[off + 1], u4b[off + 2], 255];
            }

            pixel.copy_from_slice(&rgba);
        }*/


        self.draw_rect(frame, &[0, 0, WIDTH as usize, HEIGHT as usize], [0, 0, 0, 255]);
        self.draw_tilemap(frame, &[0, 0], &self.tileset.maps[&0]);
        self.draw_text(frame, &[100, 100]);

        let stop = get_time();

        println!("{:?}", stop - start);
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

    pub fn draw_text(&self,  frame: &mut [u8], pos: &[usize; 2]) {

        let font = &self.gohu_font_11;

        // The font size to use
        let scale = Scale::uniform(16.0);

        // The text to render
        let text = "This is RustType rendered into a png!";

        // Use a dark red colour
        //let colour = (150, 0, 0);

        let v_metrics = font.v_metrics(scale);

        // layout the glyphs in a line with 20 pixels padding
        let glyphs: Vec<_> = font
            .layout(text, scale, point(0.0, 0.0 + v_metrics.ascent))
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

                    let d = ((x + bounding_box.min.x as u32) as usize + pos[0] as usize) * 4 + ((y + bounding_box.min.y as u32) as usize + pos[1] as usize) * (WIDTH as usize) * 4;

                    frame[d..d + 4].copy_from_slice(&[255, 255, 255, (v * 255.0) as u8]);
                });
            }
        }
    }
}