use crate::GridCtx;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Cell {
    Empty,
    Variable,
    Value,
    Assign,
    GetAttr,
    SetAttr,
}

use Cell::*;

impl Cell {
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        rect: &TheDim,
        _ctx: &TheContext,
        grid_ctx: &GridCtx,
        is_selected: bool,
    ) {
        match &self {
            Empty => {
                buffer.draw_rect_outline(
                    rect,
                    if is_selected {
                        &grid_ctx.selection_color
                    } else {
                        &grid_ctx.dark_background_color
                    },
                );
            }
            _ => {
                buffer.draw_rect_outline(
                    rect,
                    if is_selected {
                        &grid_ctx.selection_color
                    } else {
                        &grid_ctx.dark_background_color
                    },
                );
            }
        }
    }
}
