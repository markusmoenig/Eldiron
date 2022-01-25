

use crate::asset::*;

// trait DrawWidget {
//     fn drawWidgets(&self, frame: &mut [u8]);
// }

pub trait ScreenWidget {

    fn new() -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8]);

    fn mouse_down(&self, pos: (u32, u32)) {
        for w in self.get_widgets() {
            if w.contains(pos) {
                w.mouse_down(pos)
            }
        }
    }

    fn mouse_up(&self, pos: (u32, u32)) {
    }

    fn get_asset(&self) -> &Asset;
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>>;

    //fn draw_rect(&self, frame: &mut [u8], rect: &[usize; 4], color: [u8; 4]);
    //fn draw_tilemap(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap);
    //fn draw_text(&self,  frame: &mut [u8], pos: &[usize; 2]);
}

// impl<T> DrawWidget for T where T: ScreenWidget {
//     fn drawWidgets(&self, frame: &mut [u8]) {
//         //println!("The {} said {}", self.animal_type(), self.noise());
//         println!("{}", "here")
//     }
// }

pub mod text;

pub trait Widget {

    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8], asset: &Asset);

    fn mouse_down(&self, pos: (u32, u32));

    fn contains(&self, pos: (u32, u32)) -> bool {
        let rect = self.get_rect();

        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }


    fn get_rect(&self) -> &(u32, u32, u32, u32);

    //fn draw_rect(&self, frame: &mut [u8], rect: &[usize; 4], color: [u8; 4]);
    //fn draw_tilemap(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap);
    //fn draw_text(&self,  frame: &mut [u8], pos: &[usize; 2]);
}