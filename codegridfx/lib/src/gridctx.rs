use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GridCtx {
    pub header_height: u32,
    pub cell_size: Vec2<u32>,

    pub selected_routine: Option<Uuid>,
    pub current_cell: Option<(u32, u32)>,

    pub font_size: f32,
    pub large_font_size: f32,
    pub zoom: f32,

    pub offset_y: i32,

    // Grid Colors
    pub background_color: [u8; 4],
    pub normal_color: [u8; 4],
    pub dark_color: [u8; 4],
    pub selection_color: [u8; 4],
    pub text_color: [u8; 4],
    pub highlight_text_color: [u8; 4],
    pub error_color: [u8; 4],
}

impl Default for GridCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl GridCtx {
    pub fn new() -> Self {
        Self {
            header_height: 35,
            cell_size: Vec2::new(100, 50),

            selected_routine: None,
            current_cell: None,

            font_size: 12.5,
            large_font_size: 14.0,
            zoom: 1.0,

            offset_y: 0,

            background_color: [0; 4],
            normal_color: [0; 4],
            dark_color: [0; 4],
            selection_color: [0; 4],
            text_color: [0; 4],
            highlight_text_color: [0; 4],
            error_color: [0; 4],
        }
    }
}
