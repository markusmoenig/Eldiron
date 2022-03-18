
use crate::draw2d::Draw2D;
use server::gamedata::GameData;
use crate::editor::dialog::DialogState;
use crate::TileUsage;

use zeno::{Mask, Stroke};

use super::atom::AtomWidgetType;

#[derive(PartialEq)]
pub struct ScreenDragContext {
    pub text                            : String,
    pub color                           : [u8;4],
    pub offset                          : (isize, isize),
    pub buffer                          : Option<[u8; 180 * 32 * 4]>
}

pub struct ScreenContext {
    pub draw2d                          : Draw2D,

    pub target_fps                      : usize,
    pub default_fps                     : usize,

    pub width                           : usize,
    pub height                          : usize,

    pub data                            : GameData,

    pub toolbar_height                  : usize,
    pub toolbar_button_height           : usize,
    pub toolbar_button_rounding         : (f64, f64, f64, f64),
    pub toolbar_button_text_size        : f32,

    pub button_height                   : usize,
    pub button_text_size                : f32,
    pub button_rounding                 : (f64, f64, f64, f64),

    pub node_button_height              : usize,
    pub node_button_text_size           : f32,
    pub node_button_header_text_size    : f32,
    pub node_button_rounding            : (f64, f64, f64, f64),
    pub node_connector_color            : [u8;4],

    pub large_button_height             : usize,
    pub large_button_text_size          : f32,
    pub large_button_rounding           : (f64, f64, f64, f64),

    pub color_black                     : [u8;4],
    pub color_gray                      : [u8;4],
    pub color_light_gray                : [u8;4],
    pub color_white                     : [u8;4],
    pub color_light_white               : [u8;4],
    pub color_orange                    : [u8;4],
    pub color_light_orange              : [u8;4],
    pub color_green                     : [u8;4],
    pub color_light_green               : [u8;4],
    pub color_blue                      : [u8;4],

    pub color_node_light_gray           : [u8;4],
    pub color_node_dark_gray            : [u8;4],
    pub color_node_picker               : [u8;4],

    pub color_red                       : [u8;4],
    pub color_green_conn                : [u8;4],

    pub curr_tileset_index              : usize,
    pub curr_tile                       : Option<(usize, usize)>,
    pub selection_end                   : Option<(usize, usize)>,

    pub curr_area_index                 : usize,
    pub curr_area_tile                  : Option<(usize, usize, usize, TileUsage)>,

    pub curr_behavior_index             : usize,
    pub curr_behavior_node_id           : usize,

    pub drag_context                    : Option<ScreenDragContext>,

    pub dialog_state                    : DialogState,
    pub dialog_atom_type                : AtomWidgetType,
    pub dialog_node_behavior_id         : (usize, usize, String),
    pub dialog_node_behavior_value      : (f64, f64, f64, f64),

    // Masks
    pub right_arrow_mask                : [u8;10*10],
    pub menu_triangle_mask              : [u8;10*10],
    pub preview_arc_mask                : [u8;20*20],
}

impl ScreenContext {

    pub fn new(width: usize, height: usize) -> Self {

        let mut right_arrow_mask = [0u8; 10 * 10];
        Mask::new("M 0,0 10,5 0,10")
            .size(10, 10)
            .style(
                Stroke::new(2.0)
            )
            .render_into(&mut right_arrow_mask, None);

        let mut menu_triangle_mask = [0u8; 10 * 10];
        Mask::new("M 0,0 10,0 5,7 0,0 Z")
            .size(10, 10)
            .render_into(&mut menu_triangle_mask, None);

        let mut preview_arc_mask = [0u8; 20 * 20];
        Mask::new("M 18,18 C0,16 2,4 1,0")
            .size(20, 20)
            .style(
                Stroke::new(1.0)
            )
            .render_into(&mut preview_arc_mask, None);

        Self {
            draw2d                      : Draw2D {},

            target_fps                  : 4,
            default_fps                 : 4,

            width, height,

            data                        : GameData::new(),

            // Editor statics
            toolbar_height              : 45,
            toolbar_button_height       : 35,
            toolbar_button_rounding     : (18.0, 18.0, 18.0, 18.0),
            toolbar_button_text_size    : 27.0,

            button_height               : 25,
            button_text_size            : 24.0,
            button_rounding             : (12.0, 12.0, 12.0, 12.0),

            large_button_height         : 30,
            large_button_text_size      : 22.0,
            large_button_rounding       : (14.0, 14.0, 14.0, 14.0),

            node_button_height          : 24,
            node_button_text_size       : 21.0,
            node_button_header_text_size: 19.0,
            node_button_rounding        : (12.0, 12.0, 12.0, 12.0),
            node_connector_color        : [174, 174, 174, 255],

            color_black                 : [25, 25, 25, 255],
            color_white                 : [255, 255, 255, 255],
            color_light_white           : [240, 240, 240, 255],
            color_gray                  : [105, 105, 105, 255],
            color_light_gray            : [155, 155, 155, 255],
            color_orange                : [208, 115, 50, 255],
            color_light_orange          : [208, 156, 112, 255],
            color_green                 : [10, 93, 80, 255],
            color_light_green           : [101, 140, 134, 255],
            color_red                   : [207, 55, 54, 255],
            color_green_conn            : [20, 143, 40, 255],
            color_blue                  : [27, 79, 136, 255],

            color_node_light_gray       : [102, 102, 102, 255],
            color_node_dark_gray        : [48, 48, 48, 255],
            color_node_picker           : [186, 186, 186, 255],

            // Editor state

            drag_context                : None,

            // Tiles
            curr_tileset_index          : 0,
            curr_tile                   : None,
            selection_end               : None,

            // Areas
            curr_area_index             : 0,
            curr_area_tile              : None,

            // Behaviors
            curr_behavior_index         : 0,
            curr_behavior_node_id       : 0,

            dialog_state                : DialogState::Closed,
            dialog_atom_type            : AtomWidgetType::Button,
            dialog_node_behavior_id     : (0, 0, "".to_string()),
            dialog_node_behavior_value  : (0.0, 0.0, 0.0, 0.0),

            // UI Masks
            right_arrow_mask,
            menu_triangle_mask,
            preview_arc_mask
        }
    }

    /// Returns true if the given rect contains the given position
    pub fn contains_pos_for(&self, pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
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
}