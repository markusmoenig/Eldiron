use crate::{Cell, GridCtx};
use theframework::prelude::*;

use Cell::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellItem {
    pub id: Uuid,
    pub cell: Cell,
}

impl CellItem {
    pub fn new(cell: Cell) -> Self {
        Self {
            id: Uuid::new_v4(),
            cell,
        }
    }

    /// Draw the cell
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        rect: &TheDim,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        is_selected: bool,
    ) {
        self.cell.draw(buffer, rect, ctx, grid_ctx, is_selected);
    }

    pub fn size(&self) -> Vec2<u32> {
        match &self.cell {
            Empty => Vec2::new(100, 60),
            _ => Vec2::new(100, 60),
        }
    }

    pub fn generate_context(&self) -> TheContextMenu {
        let mut context_menu = TheContextMenu::named(str!("CGFContext"));

        match &self.cell {
            _ => {
                context_menu.add(TheContextMenuItem::new(
                    str!("Assignment"),
                    TheId::named("CGFAssignment"),
                ));
            }
        }

        context_menu
    }
}
