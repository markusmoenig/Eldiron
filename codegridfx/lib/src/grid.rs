use crate::{Cell, CellItem, CellRole, GridCtx};
use rusterix::Debug;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Grid {
    #[serde(with = "vectorize")]
    pub grid: FxHashMap<(u32, u32), CellItem>,
    #[serde(with = "vectorize")]
    pub grid_rects: FxHashMap<(u32, u32), TheDim>,
    #[serde(with = "vectorize")]
    pub row_indents: FxHashMap<u32, u32>,
    pub indent_width: u32,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            indent_width: 40,
            ..Default::default()
        }
    }

    /// Insert a cell item at the given location.
    pub fn insert(&mut self, at: (u32, u32), item: CellItem) {
        self.grid.insert(at, item);
    }

    /// Checks the cell role at the given position / offset
    pub fn is_role_at(&self, at: (u32, u32), x_offset: i32, role: CellRole) -> bool {
        if at.0 as i32 + x_offset < 0 {
            return false;
        }

        if let Some(item) = self.grid.get(&((at.0 as i32 + x_offset) as u32, at.1)) {
            if item.cell.role() == role {
                return true;
            }
        }
        false
    }

    /// Make sure there is an empty cell at the end of each row and at the bottom row.
    pub fn insert_empty(&mut self) {
        // Add an empty cell at the end of each row only if not already empty (unchanged)
        let mut rows: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for (&(col, row), _) in &self.grid {
            rows.entry(row).or_default().push(col);
        }
        for (&row, cols) in &rows {
            if let Some(&max_col) = cols.iter().max() {
                let last_cell = self.grid.get(&(max_col, row));
                let needs_empty = match last_cell {
                    Some(cell_item) => !matches!(cell_item.cell, Cell::Empty),
                    None => true,
                };
                if needs_empty {
                    self.grid
                        .insert((max_col + 1, row), CellItem::new(Cell::Empty));
                }
            }
        }

        // Determine the current bottom row and whether it’s all empty (unchanged)
        let max_row = self.grid.keys().map(|&(_, row)| row).max().unwrap_or(0);
        let bottom_row_cells: Vec<&CellItem> = self
            .grid
            .iter()
            .filter(|&(&(_, row), _)| row == max_row)
            .map(|(_, cell_item)| cell_item)
            .collect();
        let all_empty = !bottom_row_cells.is_empty()
            && bottom_row_cells
                .iter()
                .all(|cell_item| matches!(cell_item.cell, Cell::Empty));

        // If the bottom row contains any non-empty cell, add one or two new rows based on indent
        if !all_empty {
            let bottom_indent = *self.row_indents.get(&max_row).unwrap_or(&0);
            let first_new_row = max_row + 1;

            // Insert the row inside the current indent (if it doesn’t already exist)
            if !self.grid.contains_key(&(0, first_new_row)) {
                self.grid
                    .insert((0, first_new_row), CellItem::new(Cell::Empty));
                self.row_indents.insert(first_new_row, bottom_indent);
            }

            // If there is indentation, insert an extra row with one level less indent
            if bottom_indent > 0 {
                let second_new_row = first_new_row + 1;
                if !self.grid.contains_key(&(0, second_new_row)) {
                    self.grid
                        .insert((0, second_new_row), CellItem::new(Cell::Empty));
                    // The “outside” row should have indent bottom_indent - 1; clamp at 0 if needed.
                    self.row_indents.insert(second_new_row, bottom_indent - 1);
                }
            }
        }
    }

    /// Removes all dependencies for a given id.
    pub fn remove_dependencies_for(&mut self, id: Uuid) {
        let mut to_remove = vec![];
        for (coord, item) in self.grid.iter_mut() {
            if item.dependend_on == Some(id) {
                to_remove.push(coord.clone());
            }
        }
        for id in to_remove {
            self.grid.remove(&id);
            self.grid_rects.remove(&id);
        }
    }

    /// Returns the grid sorted in rows / columns
    pub fn grid_by_rows(&self) -> Vec<Vec<(&CellItem, (u32, u32))>> {
        let mut rows: FxHashMap<u32, Vec<(u32, &CellItem)>> = FxHashMap::default();
        for (&(col, row), cell) in &self.grid {
            rows.entry(row).or_default().push((col, cell));
        }
        let mut result = Vec::new();
        let mut row_keys: Vec<u32> = rows.keys().cloned().collect();
        row_keys.sort();
        for row in row_keys {
            let mut cols = rows.remove(&row).unwrap();
            cols.sort_by_key(|(col, _)| *col);
            result.push(
                cols.into_iter()
                    .map(|(col, cell)| (cell, (col, row)))
                    .collect(),
            );
        }
        result
    }

    /// Handles a return at the given row, i.e. pushes all rows down and inserts an empty row.
    /// The new row inherits the indent level of the line at `row`.
    pub fn return_at(&mut self, row: u32) {
        // Determine the current row’s indent, looking up the nearest preceding entry
        let current_indent = match self.row_indents.get(&row) {
            Some(&ind) => ind,
            None => {
                let mut r = row;
                let mut ind = 0;
                while r > 0 {
                    r -= 1;
                    if let Some(&i) = self.row_indents.get(&r) {
                        ind = i;
                        break;
                    }
                }
                ind
            }
        };

        // Check whether this row ends with a control statement (e.g. 'If')
        let mut is_control = false;
        for ((_, r), cell) in &self.grid {
            if *r == row {
                match cell.cell {
                Cell::If /* | Cell::For | Cell::While */ => {
                    is_control = true;
                    break;
                }
                _ => {}
            }
            }
        }

        // Decide how many rows to insert and what indent to use for the first new row
        let (insert_count, first_indent, _second_indent) = if current_indent == 0 && is_control {
            // At an 'if' line: insert one row with indent=1
            (1, current_indent + 1, None)
        } else if current_indent > 0 {
            // Inside a block: insert two rows, one with indent=current_indent and one with indent-1
            (2, current_indent, Some(current_indent - 1))
        } else {
            // Top-level, non-control line: insert one unindented row
            (1, current_indent, None)
        };

        // Shift rows > `row` down by insert_count
        let mut to_shift: Vec<((u32, u32), CellItem)> = Vec::new();
        for (&(col, r), cell) in &self.grid {
            if r > row {
                to_shift.push(((col, r), cell.clone()));
            }
        }
        for ((col, r), _) in &to_shift {
            self.grid.remove(&(*col, *r));
            self.grid_rects.remove(&(*col, *r));
        }
        for ((col, r), cell) in to_shift {
            self.grid.insert((col, r + insert_count), cell);
        }

        // Update the indent map for shifted rows
        let mut new_indents = FxHashMap::default();
        for (&r, &ind) in &self.row_indents {
            if r > row {
                new_indents.insert(r + insert_count, ind);
            } else {
                new_indents.insert(r, ind);
            }
        }

        // Insert the first new row and record its indent
        let inner_row = row + 1;
        self.grid.insert((0, inner_row), CellItem::new(Cell::Empty));
        new_indents.insert(inner_row, first_indent);

        // If a second row is needed (inside a block), insert it with indent one level less
        // if let Some(outdent) = second_indent {
        //     let outer_row = row + 2;
        //     self.grid.insert((0, outer_row), CellItem::new(Cell::Empty));
        //     new_indents.insert(outer_row, outdent);
        // }

        self.row_indents = new_indents;

        // Only append a trailing blank line when at top level and not handling a control statement
        if current_indent == 0 && !is_control {
            self.insert_empty();
        }
    }

    /// Handles deletion/backspace at the given row.
    /// If the previous row exists and is empty, delete that previous row; otherwise delete the current row.
    /// All rows below the removed row are shifted up and the indent map is updated accordingly.
    pub fn delete_at(&mut self, row: u32) {
        // Decide which row to remove: if the previous row is all empty cells, remove it instead.
        let mut remove_row = row;
        if row > 0 {
            let prev = row - 1;
            // Check whether the previous row exists and whether it contains only empty cells
            let mut has_prev = false;
            let mut prev_all_empty = true;
            for ((_, r), cell) in &self.grid {
                if *r == prev {
                    has_prev = true;
                    if !matches!(cell.cell, Cell::Empty) {
                        prev_all_empty = false;
                        break;
                    }
                }
            }
            if !has_prev || prev_all_empty {
                remove_row = prev;
            }
        }

        // Gather cells to remove (the row we’re deleting) and cells to shift (rows below)
        let mut to_shift: Vec<((u32, u32), CellItem)> = Vec::new();
        let mut to_remove: Vec<(u32, u32)> = Vec::new();
        for (&(col, r), cell) in &self.grid {
            if r == remove_row {
                to_remove.push((col, r));
            } else if r > remove_row {
                to_shift.push(((col, r), cell.clone()));
            }
        }

        // Remove all cells in the row being deleted
        for (col, r) in &to_remove {
            self.grid.remove(&(*col, *r));
            self.grid_rects.remove(&(*col, *r));
        }

        // Remove and reinsert all cells in lower rows, shifting them up by one
        for ((col, r), cell) in to_shift {
            self.grid.remove(&(col, r));
            self.grid_rects.remove(&(col, r));
            self.grid.insert((col, r - 1), cell);
        }

        // Update the indent map: drop the indent for the removed row, and shift indents for rows below it up by one
        let mut new_indents: FxHashMap<u32, u32> = FxHashMap::default();
        for (&r, &ind) in &self.row_indents {
            if r == remove_row {
                // skip the removed row
                continue;
            } else if r > remove_row {
                new_indents.insert(r - 1, ind);
            } else {
                new_indents.insert(r, ind);
            }
        }
        self.row_indents = new_indents;

        // Restore grid invariants: ensure each row ends with an empty cell and a new bottom row if needed
        self.insert_empty();
    }

    /// Returns the size of the grid.
    pub fn size(
        &mut self,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        folded: bool,
        screen_width: u32,
        id: u32,
        debug: Option<&Debug>,
    ) -> Vec2<u32> {
        if !folded {
            // Track widths per row and column and heights per row
            let mut row_col_widths: FxHashMap<u32, FxHashMap<u32, u32>> = FxHashMap::default();
            let mut row_heights: FxHashMap<u32, u32> = FxHashMap::default();

            self.grid_rects.clear();

            // First pass: collect individual cell sizes
            for ((col, row), cell) in &self.grid {
                let size = cell.size(ctx, grid_ctx, id, debug);
                // Update width for this row/column
                row_col_widths
                    .entry(*row)
                    .or_default()
                    .entry(*col)
                    .and_modify(|w| {
                        if size.x > *w {
                            *w = size.x;
                        }
                    })
                    .or_insert(size.x);
                // Track max height per row (unchanged from your original code)
                row_heights
                    .entry(*row)
                    .and_modify(|h| {
                        if size.y > *h {
                            *h = size.y;
                        }
                    })
                    .or_insert(size.y);
            }

            // Second pass: compute offsets and fill grid_rects per row
            for ((col, row), cell) in &self.grid {
                let cols_map = row_col_widths.get(row).unwrap();
                let mut sorted_cols: Vec<u32> = cols_map.keys().cloned().collect();
                sorted_cols.sort();

                // Determine the indent offset for this row (default to 0 if missing)
                let indent = *self.row_indents.get(row).unwrap_or(&0);
                let indent_offset = indent * self.indent_width;

                // x offset includes the indent plus the widths of earlier columns
                let x_offset = indent_offset
                    + 4
                    + sorted_cols
                        .iter()
                        .filter(|&&c| c < *col)
                        .map(|c| cols_map[c])
                        .sum::<u32>();

                // y offset remains the same
                let y_offset = 4
                    + grid_ctx.header_height
                    + row_heights
                        .keys()
                        .filter(|&&r| r < *row)
                        .map(|r| row_heights[&r])
                        .sum::<u32>();

                // Store the rectangle for this cell
                let size = cell.size(ctx, grid_ctx, id, debug);
                self.grid_rects.insert(
                    (*col, *row),
                    TheDim::rect(
                        x_offset as i32,
                        y_offset as i32,
                        size.x as i32,
                        size.y as i32,
                    ),
                );
            }

            // Overall width now accounts for each row’s indent
            let total_width = row_col_widths
                .iter()
                .map(|(&row, cols)| {
                    let indent = *self.row_indents.get(&row).unwrap_or(&0);
                    let indent_offset = indent * self.indent_width;
                    indent_offset + cols.values().sum::<u32>()
                })
                .max()
                .unwrap_or(0)
                + 4;
            let total_height = row_heights.values().sum::<u32>() + grid_ctx.header_height + 4;

            Vec2::new(total_width, total_height)
        } else {
            // When folded, only show header
            Vec2::new(screen_width, grid_ctx.header_height + 4)
        }
    }
}
