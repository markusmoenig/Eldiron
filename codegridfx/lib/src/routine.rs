use crate::{Cell, CellItem, Grid, GridCtx, cell::CellRole};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: Uuid,
    pub name: String,
    pub description: String,

    pub module_offset: u32,
    pub visible: bool,
    pub folded: bool,

    pub screen_width: u32,
    pub buffer: TheRGBABuffer,

    pub grid: Grid,
}

impl Routine {
    pub fn new(name: &str, description: &str) -> Self {
        let mut grid = Grid::default();
        grid.insert((0, 0), CellItem::new(Cell::Empty));
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            module_offset: 0,
            visible: false,
            folded: false,
            screen_width: 100,
            buffer: TheRGBABuffer::new(TheDim::sized(100, 100)),
            grid,
        }
    }

    pub fn draw(&mut self, ctx: &TheContext, grid_ctx: &GridCtx) {
        // Size check
        let height = self.size(ctx, grid_ctx).y;
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
                &grid_ctx.normal_color
            },
            &(folded_corners, 12.0, folded_corners, 12.0),
            0.0,
            &WHITE,
        );

        let stride = self.buffer.dim().width as usize;

        if let Some(font) = &ctx.ui.font {
            ctx.draw.text_rect_blend(
                self.buffer.pixels_mut(),
                &(
                    20,
                    0,
                    self.screen_width as usize,
                    grid_ctx.header_height as usize,
                ),
                stride,
                font,
                15.0,
                &self.name,
                &grid_ctx.text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            ctx.draw.text_rect_blend(
                self.buffer.pixels_mut(),
                &(
                    0,
                    0,
                    self.screen_width as usize - 10,
                    grid_ctx.header_height as usize,
                ),
                stride,
                font,
                13.0,
                &self.description,
                &grid_ctx.text_color,
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );
        }

        if !self.folded {
            for (coord, cell) in &mut self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    let is_selected = Some(self.id) == grid_ctx.selected_routine
                        && Some(coord.clone()) == grid_ctx.current_cell;
                    cell.draw(&mut self.buffer, &rect, ctx, grid_ctx, is_selected, coord);
                }
            }
        }
    }

    /// Sets the screen width.
    pub fn set_screen_width(&mut self, width: u32, ctx: &TheContext, grid_ctx: &GridCtx) {
        self.screen_width = width;
        self.draw(ctx, grid_ctx);
    }

    /// Returns the number of lines in the grid.
    pub fn lines(&self) -> u32 {
        let mut lines = 1;
        for (c, _) in &self.grid.grid {
            if c.1 > lines {
                lines = c.1;
            }
        }
        lines
    }

    /// Returns the size of the grid.
    pub fn size(&mut self, ctx: &TheContext, grid_ctx: &GridCtx) -> Vec2<u32> {
        if !self.folded {
            let mut col_widths: FxHashMap<u32, u32> = FxHashMap::default();
            let mut row_heights: FxHashMap<u32, u32> = FxHashMap::default();

            // Clear grid_rects before filling
            self.grid.grid_rects.clear();

            // First pass: collect sizes
            for ((col, row), cell) in &self.grid.grid {
                let size = cell.size(ctx, grid_ctx);
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

            // Second pass: calculate offsets and fill grid_rects
            for ((col, row), cell) in &self.grid.grid {
                let x_offset = 4 + col_widths
                    .keys()
                    .filter(|&&c| c < *col)
                    .map(|c| col_widths[c])
                    .sum::<u32>();
                let y_offset = 4
                    + grid_ctx.header_height
                    + row_heights
                        .keys()
                        .filter(|&&r| r < *row)
                        .map(|r| row_heights[r])
                        .sum::<u32>();
                let size = cell.size(ctx, grid_ctx);
                self.grid.grid_rects.insert(
                    (*col, *row),
                    TheDim::rect(
                        x_offset as i32,
                        y_offset as i32,
                        size.x as i32,
                        size.y as i32,
                    ),
                );
            }

            let total_width: u32 = col_widths.values().sum::<u32>() + 4;
            let total_height: u32 = row_heights.values().sum::<u32>() + grid_ctx.header_height + 4;

            Vec2::new(total_width, total_height)
        } else {
            // Only header if folded
            Vec2::new(self.screen_width, grid_ctx.header_height + 4)
        }
    }

    /// Handle a click at the given position.
    pub fn drop_at(
        &mut self,
        loc: Vec2<u32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
        drop: &TheDrop,
    ) -> bool {
        let mut handled = false;
        let mut pos: Option<(u32, u32)> = None;
        let mut old_item: CellItem = CellItem::new(Cell::Empty);

        if loc.y > grid_ctx.header_height {
            for (coord, item) in self.grid.grid.iter_mut() {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        if item.replaceable {
                            grid_ctx.selected_routine = Some(self.id);
                            grid_ctx.current_cell = Some(coord.clone());
                            pos = Some(coord.clone());
                            old_item = item.clone();
                        }
                        handled = true;
                        break;
                    }
                }
            }
        }

        if let Some(pos) = pos {
            if let Some(cell) = Cell::from_str(&drop.title) {
                let mut item = CellItem::new(cell);
                let mut insert = true;

                if old_item.cell.role() != item.cell.role() && old_item.cell != Cell::Empty {
                    insert = false;
                }

                if insert {
                    if item.cell.role() == CellRole::Value {
                        item.description = old_item.description.clone();
                        item.replaceable = old_item.replaceable.clone();
                        item.dependend_on = old_item.dependend_on.clone();
                    }

                    self.grid.remove_dependencies_for(old_item.id);
                    item.insert_at(pos, &mut self.grid, old_item);
                }
            }
        }

        if let Some(pos) = pos {
            if let Some(item) = self.grid.grid.get(&pos) {
                let nodeui: TheNodeUI = item.create_settings();
                if let Some(layout) = ui.get_text_layout("Node Settings") {
                    nodeui.apply_to_text_layout(layout);
                    ctx.ui.relayout = true;
                }
            }

            self.grid.insert_empty();
            self.draw(ctx, grid_ctx);
        }

        handled
    }

    /// Handle a click at the given position.
    pub fn click_at(
        &mut self,
        loc: Vec2<u32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
    ) -> bool {
        let mut handled = false;

        if loc.y < grid_ctx.header_height {
            self.folded = !self.folded;
            grid_ctx.selected_routine = Some(self.id);
            grid_ctx.current_cell = None;
            self.draw(ctx, grid_ctx);
            handled = true;
        } else {
            for (coord, cell) in &self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        grid_ctx.selected_routine = Some(self.id);
                        if grid_ctx.current_cell != Some(coord.clone()) {
                            grid_ctx.current_cell = Some(coord.clone());

                            let nodeui: TheNodeUI = cell.create_settings();
                            if let Some(layout) = ui.get_text_layout("Node Settings") {
                                nodeui.apply_to_text_layout(layout);
                                ctx.ui.relayout = true;
                            }

                            self.draw(ctx, grid_ctx);
                        }
                        handled = true;
                        break;
                    }
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
        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord) {
                if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                    grid_ctx.selected_routine = Some(self.id);
                    grid_ctx.current_cell = Some(coord.clone());
                    return Some(item.generate_context());
                }
            }
        }

        None
    }

    /// Build the routine into Python source
    pub fn build(&self, out: &mut String, indent: usize) {
        let mut indent = indent;

        *out += &format!("{:indent$}if event == \"{}\":\n", "", self.name);
        indent += 4;

        let rows = self.grid.grid_by_rows();
        for row in rows {
            let mut row_code = String::new();
            for (item, _pos) in row {
                row_code += &item.code();
                row_code += " ";
            }
            *out += &format!("{:indent$}{}\n", "", row_code);
        }
    }
}
