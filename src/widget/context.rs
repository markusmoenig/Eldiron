
use crate::draw2d::Draw2D;

pub struct ScreenContext {
    pub draw2d                  : Draw2D,

    pub toolbar_height          : usize,
    pub toolbar_button_height   : usize,
    pub toolbar_button_rounding : (f64, f64, f64, f64),
    pub toolbar_button_text_size: f32,

    pub button_height           : usize,
    pub button_text_size        : f32,
    pub button_rounding         : (f64, f64, f64, f64),

    pub color_black             : [u8;4],
    pub color_light_gray        : [u8;4],
    pub color_white             : [u8;4],
    pub color_yellow            : [u8;4],
    pub color_light_yellow      : [u8;4],

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
            toolbar_button_text_size    : 30.0,

            button_height               : 25,
            button_text_size            : 24.0,
            button_rounding             : (12.0, 12.0, 12.0, 12.0),

            color_black                 : [25, 25, 25, 255],
            color_white                 : [255, 255, 255, 255],
            color_light_gray            : [105, 105, 105, 255],
            color_yellow                : [208, 115, 50, 255],
            color_light_yellow          : [208, 156, 112, 255],

            width, height,
        }
    }

    // pub fn layout_hor_fixed(&self, margin: usize, width: usize, total_width: usize, widgets: usize) -> Vec<usize> {
    //     let mut v: Vec<usize> = vec![];

    //     let total_space_used = width * widgets + margin * 2;
    //     let mut spacing = 0_usize;

    //     if total_width > total_space_used {
    //         //spacing = (total_width - total_space_used) / (widgets - 1);
    //     }

    //     let mut r = margin;
    //     for i in 0..widgets {
    //         v.push(r);
    //         r += width + spacing;
    //     }

    //     v
    // }

    /// Returns true if the given rect contains the given position
    pub fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    /*
    pub fn process_mouse_hover(&self, pos: (usize, usize,), widgets: &mut Vec<Box<dyn Widget>>, asset: &mut Asset) -> bool {
        let mut consumed = false;

        for w in widgets {
            if w.mouse_hover(pos, asset) {
                consumed = true;
                break;
            }
        }
        consumed
    }*/
}