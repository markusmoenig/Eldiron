
use crate::draw2d::Draw2D;
//use crate::Widget;

pub struct ScreenContext {
    pub draw2d                  : Draw2D,

    pub toolbar_height          : usize,
    pub toolbar_button_height   : usize,
    pub toolbar_button_rounding : (f64, f64, f64, f64),


    pub color_black             : [u8;4],
    pub color_light_gray        : [u8;4],
    pub color_white             : [u8;4],

    pub width                   : usize,
    pub height                  : usize,
}

impl ScreenContext {

    pub fn new(width: usize, height: usize) -> Self {

        Self {
            draw2d                      : Draw2D {},
            
            toolbar_height              : 45,
            toolbar_button_height       : 35,
            toolbar_button_rounding     : (18.0, 18.0, 18.0, 18.0),

            color_black                 : [25, 25, 25, 255],
            color_white                 : [255, 255, 255, 255],
            color_light_gray            : [105, 105, 105, 255],

            width, height,
        }
    }
}