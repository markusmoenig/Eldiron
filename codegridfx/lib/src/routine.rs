use crate::{Cell, CellItem, GridCtx, Group, cellgroup::CellGroup};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: Uuid,
    pub name: String,

    pub module_offset: u32,
    pub visible: bool,
    pub folded: bool,

    pub screen_width: u32,
    pub buffer: TheRGBABuffer,

    pub grid: FxHashMap<(u32, u32), CellItem>,
    pub groups: FxHashMap<Uuid, Group>,
}

impl Routine {
    pub fn new(name: String) -> Self {
        let mut grid = FxHashMap::default();
        grid.insert((0, 0), CellItem::new(Cell::Empty));
        Self {
            id: Uuid::new_v4(),
            name,
            module_offset: 0,
            visible: false,
            folded: false,
            screen_width: 100,
            buffer: TheRGBABuffer::new(TheDim::sized(100, 100)),
            grid,
            groups: FxHashMap::default(),
        }
    }

    pub fn draw(&mut self, ctx: &TheContext, grid_ctx: &GridCtx) {
        // Size check
        let height = self.size(grid_ctx).y;
        if self.buffer.dim().width != self.screen_width as i32
            || self.buffer.dim().height != height as i32
        {
            self.buffer =
                TheRGBABuffer::new(TheDim::sized(self.screen_width as i32, height as i32));
        }

        self.buffer.fill(grid_ctx.background_color);

        let folded_corners = if !self.folded { 0.0 } else { 12.0 };
        let is_selected = Some(self.id) == grid_ctx.selected_routine;

        self.buffer.draw_rounded_rect(
            &TheDim::rect(
                0,
                0,
                self.screen_width as i32,
                grid_ctx.header_height as i32,
            ),
            if is_selected {
                &grid_ctx.selection_color
            } else {
                &grid_ctx.background_color
            },
            &(folded_corners, 12.0, folded_corners, 12.0),
            0.0,
            &WHITE,
        );

        if let Some(font) = &ctx.ui.font {
            ctx.draw.text_rect_blend(
                self.buffer.pixels_mut(),
                &(
                    10,
                    0,
                    self.screen_width as usize,
                    grid_ctx.header_height as usize,
                ),
                ctx.width,
                font,
                15.0,
                &self.name,
                &grid_ctx.text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }

        if !self.folded {
            for (coord, cell) in &mut self.grid {
                let rect = Self::get_rect_for(coord, grid_ctx);
                let is_selected = Some(self.id) == grid_ctx.selected_routine
                    && Some(coord.clone()) == grid_ctx.selected_cell;
                cell.draw(&mut self.buffer, &rect, ctx, grid_ctx, is_selected);
            }
        }
    }

    /// Get the rect for the given cell.
    pub fn get_rect_for(coord: &(u32, u32), grid_ctx: &GridCtx) -> TheDim {
        TheDim::rect(
            (4 + coord.0 * grid_ctx.cell_size.x) as i32,
            (4 + grid_ctx.header_height + coord.1 * grid_ctx.cell_size.y) as i32,
            grid_ctx.cell_size.x as i32,
            grid_ctx.cell_size.y as i32,
        )
    }

    /// Sets the screen width.
    pub fn set_screen_width(&mut self, width: u32, ctx: &TheContext, grid_ctx: &GridCtx) {
        self.screen_width = width;
        self.draw(ctx, grid_ctx);
    }

    /// Returns the number of lines in the grid.
    pub fn lines(&self) -> u32 {
        let mut lines = 1;
        for (c, _) in &self.grid {
            if c.1 > lines {
                lines = c.1;
            }
        }
        lines
    }

    /// Returns the size of the grid.
    pub fn size(&self, grid_ctx: &GridCtx) -> Vec2<u32> {
        if !self.folded {
            let mut col_widths: FxHashMap<u32, u32> = FxHashMap::default();
            let mut row_heights: FxHashMap<u32, u32> = FxHashMap::default();

            for ((col, row), cell) in &self.grid {
                let size = cell.size();
                col_widths
                    .entry(*col)
                    .and_modify(|w| {
                        if size.x > *w {
                            *w = size.x;
                        }
                    })
                    .or_insert(size.x);
                row_heights
                    .entry(*row)
                    .and_modify(|h| {
                        if size.y > *h {
                            *h = size.y;
                        }
                    })
                    .or_insert(size.y);
            }

            let total_width: u32 = col_widths.values().sum::<u32>() + 4;
            let total_height: u32 = row_heights.values().sum::<u32>() + grid_ctx.header_height + 4;

            Vec2::new(total_width, total_height)
        } else {
            // Only header if folded
            Vec2::new(self.screen_width, grid_ctx.header_height + 4)
        }
    }

    /// Add a
    pub fn add_group_at(&mut self, group: Group, coord: (u32, u32)) {
        let cells = group.create_cells();
        let mut group_item = CellGroup::new(group);

        let mut offset = 0;
        for cell in cells {
            group_item.items.insert(cell.id);
            self.grid.insert((coord.0 + offset, coord.1), cell);
            offset += 1;
        }
    }

    /// Handle a click at the given position.
    pub fn click_at(&mut self, loc: Vec2<u32>, ctx: &TheContext, grid_ctx: &mut GridCtx) -> bool {
        let mut handled = false;

        if loc.y < grid_ctx.header_height {
            self.folded = !self.folded;
            grid_ctx.selected_routine = Some(self.id);
            grid_ctx.selected_cell = None;
            self.draw(ctx, grid_ctx);
            handled = true;
        } else {
            for (coord, _) in &self.grid {
                let rect = Self::get_rect_for(coord, grid_ctx);
                if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                    grid_ctx.selected_routine = Some(self.id);
                    grid_ctx.selected_cell = Some(coord.clone());
                    self.draw(ctx, grid_ctx);
                    handled = true;
                    break;
                }
            }
        }

        handled
    }

    /// Handle menu context at the given position.
    pub fn context_at(
        &mut self,
        loc: Vec2<u32>,
        _ctx: &TheContext,
        grid_ctx: &mut GridCtx,
    ) -> Option<TheContextMenu> {
        for (coord, item) in &self.grid {
            let rect = Self::get_rect_for(coord, grid_ctx);
            if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                grid_ctx.selected_routine = Some(self.id);
                grid_ctx.selected_cell = Some(coord.clone());
                return Some(item.generate_context());
            }
        }

        None
    }
}
