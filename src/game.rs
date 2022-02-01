use crate::asset::Asset;

use crate::widget:: {ScreenWidget, Widget};

/// Which window do we show currently
enum GameState {
    StartMenu
}

/// The main Game struct
pub struct Game<'a>  {
    window                  : GameState,
    asset                   : Asset<'a>,
    widgets                 : Vec<Box<dyn Widget>>
}

impl ScreenWidget for Game<'_>  {
    
    fn new() -> Self where Self: Sized {
        Self {
            window          : GameState::StartMenu,
            asset           : Asset::new(),
            widgets         : vec!()
        }
    }

    /// Update the game state
    fn update(&mut self) {
    }
    
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, _frame: &mut [u8], _anim_counter: u32) {

        // Draw the current window
        match self.window {
            GameState::StartMenu => println!("{}", 1),
        }

        let start = self.get_time();

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


        //self.asset.draw_rect(frame, &[0, 0, WIDTH as usize, HEIGHT as usize], [0, 0, 0, 255]);
        //self.asset.draw_tilemap(frame, &[0, 0], &self.asset.tileset.maps[&0]);
        //self.asset.draw_text(frame, &[100, 100]);

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }
    
    
    /// Returns the asset structure
    fn get_asset(&self) -> &Asset {
        &self.asset
    }    

    /// Returns the current widgets
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>> {
        &self.widgets
    }
}