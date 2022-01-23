mod tileset;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::prelude::*;

// pub const TILE_SIZE: i16    = 64;

/// Gets the current time in milliseconds
pub fn get_time() -> u128 {
    let stop = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");    
    stop.as_millis()
}

/// The main Game struct
pub struct Game {
    tileset                 : tileset::TileSet,
    // box_x: i16,
    // box_y: i16,
    // velocity_x: i16,
    // velocity_y: i16,
}

impl Game {
    /// Create a new `World` instance that can draw a moving box.
    pub fn new() -> Self {

        Self {
            tileset         : tileset::TileSet::new()
            // box_x: 24,
            // box_y: 16,
            // velocity_x: 1,
            // velocity_y: 1,
        }
    }

    /// Update the `World` internal state; bounce the box around the screen.
    pub fn update(&mut self) {
        /* 
        if self.box_x <= 0 || self.box_x + BOX_SIZE > WIDTH as i16 {
            self.velocity_x *= -1;
        }
        if self.box_y <= 0 || self.box_y + BOX_SIZE > HEIGHT as i16 {
            self.velocity_y *= -1;
        }

        self.box_x += self.velocity_x;
        self.box_y += self.velocity_y;
        */
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    pub fn draw(&self, frame: &mut [u8]) {

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
}