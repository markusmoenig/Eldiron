
pub mod tileset;

use std::path::PathBuf;

use rusttype::{Font};

pub use tileset::*;

pub struct Asset<'a> {
    pub tileset                 : TileSet,
    //gohu_font_11              : Font<'a>,
    //gohu_font_14                : Font<'a>,
    pub open_sans               : Font<'a>,
    pub grid_size               : u32,
}

impl Asset<'_>  {

    pub fn new() -> Self where Self: Sized {

        Self {
            tileset         : tileset::TileSet::new(),
            //gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            //gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/Open_Sans/static/OpenSans/OpenSans-SemiBold.ttf") as &[u8]).expect("Error constructing Font"),
            open_sans       : Font::try_from_bytes(include_bytes!("../../assets/fonts/Open_Sans/static/OpenSans/OpenSans-Regular.ttf") as &[u8]).expect("Error constructing Font"),
            grid_size       : 32,
        }
    }

    pub fn load_from_path(&mut self, path: PathBuf) {
        self.tileset = tileset::TileSet::load_from_path(path);
    }

    /// Returns the tilemap of the given id
    pub fn get_map_of_id(&self, id: usize) -> &TileMap {
        &self.tileset.maps[&id]
    }

    /// Returns the tile fo the given id
    pub fn get_tile(&self, id: &(usize, usize, usize)) -> Tile {
        let map = self.get_map_of_id(id.0);
        map.get_tile(&(id.1, id.2))
    }
}
