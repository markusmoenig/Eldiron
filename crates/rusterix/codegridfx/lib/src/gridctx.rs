use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GridCtx {
    pub selected_routine: Option<Uuid>,
    pub current_cell: Option<(u32, u32)>,

    pub zoom: f32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Default for GridCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl GridCtx {
    pub fn new() -> Self {
        Self {
            selected_routine: None,
            current_cell: None,
            zoom: 1.0,

            offset_x: 0,
            offset_y: 0,
        }
    }
}
