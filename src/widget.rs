

use crate::asset::*;

trait DrawWidget {
    fn drawWidgets(&self, frame: &mut [u8]);
}

pub trait ScreenWidget {

    fn new() -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8]);

    fn get_asset(&self) -> &Asset;
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>>;

    //fn draw_rect(&self, frame: &mut [u8], rect: &[usize; 4], color: [u8; 4]);
    //fn draw_tilemap(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap);
    //fn draw_text(&self,  frame: &mut [u8], pos: &[usize; 2]);
}

impl<T> DrawWidget for T where T: ScreenWidget {
    fn drawWidgets(&self, frame: &mut [u8]) {
        //println!("The {} said {}", self.animal_type(), self.noise());
        println!("{}", "here")
    }
}

pub mod text;

pub trait Widget {

    fn new(title: String, rect: [u32; 4]) -> Self where Self: Sized;

    fn update(&mut self);
    fn draw(&self, frame: &mut [u8], asset: &Asset);

    //fn draw_rect(&self, frame: &mut [u8], rect: &[usize; 4], color: [u8; 4]);
    //fn draw_tilemap(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap);
    //fn draw_text(&self,  frame: &mut [u8], pos: &[usize; 2]);
}