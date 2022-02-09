
use crate::draw2d::Draw2D;
//use crate::Widget;

pub struct ScreenContext {
    pub draw2d              : Draw2D,

    pub width               : usize,
    pub height              : usize,
}

impl ScreenContext {

    pub fn new(width: usize, height: usize) -> Self {

        Self {
            draw2d          : Draw2D {},

            width, height,
        }
    }
}