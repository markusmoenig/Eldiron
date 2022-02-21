
use crate::draw2d::Draw2D;

pub struct ScreenContext {
    pub draw2d                  : Draw2D,

    pub target_fps              : usize,
    pub default_fps             : usize,

    pub width                   : usize,
    pub height                  : usize,

    pub toolbar_height          : usize,
    pub toolbar_button_height   : usize,
    pub toolbar_button_rounding : (f64, f64, f64, f64),
    pub toolbar_button_text_size: f32,

    pub button_height           : usize,
    pub button_text_size        : f32,
    pub button_rounding         : (f64, f64, f64, f64),

    pub color_black             : [u8;4],
    pub color_gray              : [u8;4],
    pub color_light_gray        : [u8;4],
    pub color_white             : [u8;4],
    pub color_light_white       : [u8;4],
    pub color_yellow            : [u8;4],
    pub color_light_yellow      : [u8;4],

    pub curr_tileset_index      : usize,
    pub curr_tile               : Option<(usize, usize)>,
    pub selection_end           : Option<(usize, usize)>,
}

impl ScreenContext {

    pub fn new(width: usize, height: usize) -> Self {

        Self {
            draw2d                      : Draw2D {},

            target_fps                  : 4,
            default_fps                 : 4,

            width, height,

            // Editor statics
            toolbar_height              : 45,
            toolbar_button_height       : 35,
            toolbar_button_rounding     : (18.0, 18.0, 18.0, 18.0),
            toolbar_button_text_size    : 27.0,

            button_height               : 25,
            button_text_size            : 24.0,
            button_rounding             : (12.0, 12.0, 12.0, 12.0),

            color_black                 : [25, 25, 25, 255],
            color_white                 : [255, 255, 255, 255],
            color_light_white           : [240, 240, 240, 255],
            color_gray                  : [105, 105, 105, 255],
            color_light_gray            : [155, 155, 155, 255],
            color_yellow                : [208, 115, 50, 255],
            color_light_yellow          : [208, 156, 112, 255],

            // Editor state

            // Tiles
            curr_tileset_index          : 0,
            curr_tile                   : None,
            selection_end               : None,
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
    pub fn _contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
        if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
            true
        } else {
            false
        }
    }

    /// Returns true if the given rect (with an isize offset) contains the given position
    pub fn contains_pos_for_isize(&self, pos: (usize, usize), rect: (isize, isize, usize, usize)) -> bool {
        if pos.0 as isize >= rect.0 && (pos.0 as isize) < rect.0 + rect.2 as isize && pos.1 as isize >= rect.1 && (pos.1 as isize) < rect.1 + rect.3 as isize {
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