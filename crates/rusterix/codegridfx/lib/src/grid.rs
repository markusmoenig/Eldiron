use crate::{Cell, CellItem, CellRole, DebugModule, GridCtx};
use theframework::prelude::*;

const INDENT_WIDTH: u32 = 60;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Grid {
    #[serde(with = "vectorize")]
    pub grid: FxHashMap<(u32, u32), CellItem>,
    #[serde(with = "vectorize")]
    pub grid_rects: FxHashMap<(u32, u32), TheDim>,
    #[serde(with = "vectorize")]
    pub row_indents: FxHashMap<u32, u32>,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Counts the amount of non empty cells in the grid
    pub fn count(&self) -> i32 {
        let mut count = 0;
        for item in self.grid.values() {
            if !matches!(item.cell, Cell::Empty) {
                count += 1;
            }
        }
        count
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

    /// Return the effective indent for a given row, walking upward if absent.
    fn effective_indent(&self, row: u32) -> u32 {
        if let Some(&ind) = self.row_indents.get(&row) {
            return ind;
        }
        let mut r = row;
        while r > 0 {
            let prev = r - 1;
            if let Some(&i) = self.row_indents.get(&prev) {
                return i;
            }
            r -= 1;
        }
        0
    }

    /// Shift all rows with index >= start_row down by `count`.
    pub fn shift_rows_down_from(&mut self, start_row: u32, count: u32) {
        // Collect and remove impacted cells first
        let mut to_shift: Vec<((u32, u32), CellItem)> = Vec::new();
        for (&(col, r), cell) in &self.grid {
            if r >= start_row {
                to_shift.push(((col, r), cell.clone()));
            }
        }
        for ((col, r), _) in &to_shift {
            self.grid.remove(&(*col, *r));
            self.grid_rects.remove(&(*col, *r));
        }
        for ((col, r), cell) in to_shift {
            self.grid.insert((col, r + count), cell);
        }

        // Update indents
        let mut new_indents = FxHashMap::default();
        for (&r, &ind) in &self.row_indents {
            if r >= start_row {
                new_indents.insert(r + count, ind);
            } else {
                new_indents.insert(r, ind);
            }
        }
        self.row_indents = new_indents;
    }

    /// Ensure invariants:
    /// 1) Every existing row ends with a trailing `Cell::Empty`.
    /// 2) After the last **non-empty** row, create a suffix of rows — one per
    ///    indentation level from that row's indent down to 0 — each containing at
    ///    least one `Cell::Empty`. This guarantees there's always a drop target at
    ///    every indentation level.
    pub fn insert_empty(&mut self) {
        // --- (1) Make sure each existing row ends with an Empty cell ---
        let mut rows: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for (&(col, row), _) in &self.grid {
            rows.entry(row).or_default().push(col);
        }
        for (row, cols) in rows {
            // Determine the left-most **non-empty** cell for this row
            let mut leading_is_else = false;
            if let Some(&lead_col) = cols
                .iter()
                .filter(|&&c| {
                    if let Some(ci) = self.grid.get(&(c, row)) {
                        !matches!(ci.cell, Cell::Empty)
                    } else {
                        false
                    }
                })
                .min()
            {
                if let Some(ci) = self.grid.get(&(lead_col, row)) {
                    if matches!(ci.cell, Cell::Else) {
                        leading_is_else = true;
                    }
                }
            }

            if leading_is_else {
                // For an Else row, do NOT append a trailing empty cell.
                continue;
            }

            if let Some(&max_col) = cols.iter().max() {
                let needs_empty = match self.grid.get(&(max_col, row)) {
                    Some(cell_item) => !matches!(cell_item.cell, Cell::Empty),
                    None => true,
                };
                if needs_empty {
                    self.grid
                        .insert((max_col + 1, row), CellItem::new(Cell::Empty));
                }
            } else {
                // No cells recorded for this row key; ensure at least one empty cell exists.
                self.grid.insert((0, row), CellItem::new(Cell::Empty));
            }
        }

        // --- (2) Find the bottom-most row that has ANY non-empty cell ---
        let mut bottom_nonempty_row: Option<u32> = None;
        for (&(_, row), cell) in &self.grid {
            if !matches!(cell.cell, Cell::Empty) {
                bottom_nonempty_row = match bottom_nonempty_row {
                    Some(r) if row > r => Some(row),
                    None => Some(row),
                    other => other,
                };
            }
        }

        // If there are no non-empty rows at all, treat row 0 as the base.
        let base_row = bottom_nonempty_row.unwrap_or(0);

        // Determine the indentation level of the base row. If it's not explicitly in
        // row_indents, walk upwards to find the nearest defined indent, defaulting to 0.
        let base_indent = if let Some(&ind) = self.row_indents.get(&base_row) {
            ind
        } else {
            let mut r = base_row;
            let mut ind = 0;
            while r > 0 {
                r -= 1;
                if let Some(&i) = self.row_indents.get(&r) {
                    ind = i;
                    break;
                }
            }
            ind
        };

        // --- (3) Ensure a suffix of rows without downgrading existing indents ---
        // If there is already an (all-empty) row directly after the last non-empty row
        // with indent = base_indent + 1 (typical after inserting an `If`), keep it.
        let first_row = base_row + 1;

        // Check whether `first_row` exists and is all-empty
        let mut first_row_exists = false;
        let mut first_row_all_empty = true;
        for (&(_, r), cell) in &self.grid {
            if r == first_row {
                first_row_exists = true;
                if !matches!(cell.cell, Cell::Empty) {
                    first_row_all_empty = false;
                    break;
                }
            }
        }

        // Detect whether the base row is a control opener (If / For / While, etc.)
        let mut base_is_control = false;
        for (&(_, r), cell) in &self.grid {
            if r == base_row {
                match cell.cell {
                    Cell::If | Cell::Else /* | Cell::For | Cell::While */ => {
                        base_is_control = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Determine the starting indent level for the suffix sequence.
        // Default is the base indent; allow base_indent+1 only if base row opens a block.
        let existing_first_indent = self.row_indents.get(&first_row).copied();
        let mut start_level = base_indent;

        if base_is_control && first_row_exists && first_row_all_empty {
            let desired = base_indent.saturating_add(1);
            let effective = existing_first_indent.unwrap_or(desired);
            // Keep the higher one (never downgrade an existing higher indent)
            start_level = effective.max(desired);
        } else {
            // No block opener: do not elevate indent. Normalize back to base_indent if needed.
            if first_row_exists {
                if let Some(cur) = existing_first_indent {
                    if cur > base_indent {
                        self.row_indents.insert(first_row, base_indent);
                    }
                }
            }
        }

        // Now create/ensure a continuous suffix from `start_level` down to 0
        // mapping levels to rows: row = base_row + 1 + offset, level = start_level - offset
        let mut next_row = base_row + 1;
        for level in (0..=start_level).rev() {
            // Ensure row exists
            let row_exists = self.grid.keys().any(|&(_, r)| r == next_row);
            if !row_exists {
                self.grid.insert((0, next_row), CellItem::new(Cell::Empty));
            } else {
                // Ensure this row ends with an Empty cell (in case it already has content)
                let mut max_col: Option<u32> = None;
                for (&(c, r), _) in &self.grid {
                    if r == next_row {
                        max_col = Some(max_col.map_or(c, |m| m.max(c)));
                    }
                }
                if let Some(mc) = max_col {
                    let needs_empty = match self.grid.get(&(mc, next_row)) {
                        Some(cell_item) => !matches!(cell_item.cell, Cell::Empty),
                        None => true,
                    };
                    if needs_empty {
                        self.grid
                            .insert((mc + 1, next_row), CellItem::new(Cell::Empty));
                    }
                } else {
                    self.grid.insert((0, next_row), CellItem::new(Cell::Empty));
                }
            }

            // Set the indent for this suffix row, but do not downgrade if a higher indent exists
            match self.row_indents.get(&next_row).copied() {
                Some(existing) => {
                    if existing < level {
                        self.row_indents.insert(next_row, level);
                    }
                }
                None => {
                    self.row_indents.insert(next_row, level);
                }
            }

            next_row += 1;
        }

        // --- (4) Normalize mid-grid indent gaps (ensure drop target at each level)
        self.fill_indent_gaps();

        // --- (5) Always ensure one empty row after the last non-empty row ---
        let mut last_nonempty: Option<u32> = None;
        for (&(_, r), cell) in &self.grid {
            if !matches!(cell.cell, Cell::Empty) {
                last_nonempty = Some(last_nonempty.map_or(r, |m| m.max(r)));
            }
        }

        if let Some(last) = last_nonempty {
            // Is there any row strictly below the last non-empty row?
            let has_below = self.grid.keys().any(|&(_, r)| r > last);
            if !has_below {
                let new_row = last + 1;
                // Insert a single Empty cell for the new bottom row
                self.grid.insert((0, new_row), CellItem::new(Cell::Empty));
                // Inherit the effective indent of the last non-empty row (usually 0 here)
                let ind = self.effective_indent(last);
                self.row_indents.insert(new_row, ind);
            }
        }
    }

    /// Ensure that between any two adjacent existing rows, if the indent drops by
    /// more than 1, we insert intermediate empty rows so there is always a drop
    /// target at each missing indent level.
    pub fn fill_indent_gaps(&mut self) {
        loop {
            let mut changed = false;
            // Build sorted unique list of existing row indices
            let mut row_keys: Vec<u32> = self.grid.keys().map(|&(_, r)| r).collect();
            row_keys.sort();
            row_keys.dedup();

            for w in row_keys.windows(2) {
                let r = w[0];
                let next = w[1];
                let ind_r = self.effective_indent(r);
                let ind_next = self.effective_indent(next);

                if ind_r > ind_next + 1 {
                    // Insert one intermediate row just before `next` with indent ind_r-1
                    self.shift_rows_down_from(next, 1);
                    self.grid.insert((0, next), CellItem::new(Cell::Empty));
                    self.row_indents.insert(next, ind_r - 1);
                    changed = true;
                    break; // restart scan since indices changed
                }
            }

            if !changed {
                break;
            }
        }

        // After structural inserts, ensure each row ends with an empty cell
        let mut rows: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for (&(col, row), _) in &self.grid {
            rows.entry(row).or_default().push(col);
        }
        for (row, cols) in rows {
            // Determine the left-most **non-empty** cell for this row
            let mut leading_is_else = false;
            if let Some(&lead_col) = cols
                .iter()
                .filter(|&&c| {
                    if let Some(ci) = self.grid.get(&(c, row)) {
                        !matches!(ci.cell, Cell::Empty)
                    } else {
                        false
                    }
                })
                .min()
            {
                if let Some(ci) = self.grid.get(&(lead_col, row)) {
                    if matches!(ci.cell, Cell::Else) {
                        leading_is_else = true;
                    }
                }
            }

            if leading_is_else {
                // For an Else row, do NOT append a trailing empty cell.
                continue;
            }

            if let Some(&max_col) = cols.iter().max() {
                let needs_empty = match self.grid.get(&(max_col, row)) {
                    Some(cell_item) => !matches!(cell_item.cell, Cell::Empty),
                    None => true,
                };
                if needs_empty {
                    self.grid
                        .insert((max_col + 1, row), CellItem::new(Cell::Empty));
                }
            } else {
                self.grid.insert((0, row), CellItem::new(Cell::Empty));
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

    /// Insert a sibling empty row adjacent to `row` with the same indent.
    /// For control rows (If/Else), insert **above** so the line below stays reserved for the block body.
    /// For non-control rows, insert **below**.
    pub fn return_sibling_at(&mut self, row: u32) {
        let indent = self.effective_indent(row);

        // Detect if the leading non-empty cell is a control opener (If/Else)
        let mut lead_col: Option<u32> = None;
        let mut is_control = false;
        for (&(c, r), cell) in &self.grid {
            if r == row && !matches!(cell.cell, Cell::Empty) {
                match lead_col {
                    Some(prev) if c >= prev => {}
                    _ => lead_col = Some(c),
                }
            }
        }
        if let Some(c0) = lead_col {
            if let Some(ci) = self.grid.get(&(c0, row)) {
                is_control = matches!(ci.cell, Cell::If /* | Cell::For | Cell::While */);
            }
        }

        if is_control {
            // Insert sibling **above** current row
            self.shift_rows_down_from(row, 1);
            self.grid.insert((0, row), CellItem::new(Cell::Empty));
            self.row_indents.insert(row, indent);
        } else {
            // Insert sibling **below** current row
            self.shift_rows_down_from(row + 1, 1);
            let new_row = row + 1;
            self.grid.insert((0, new_row), CellItem::new(Cell::Empty));
            self.row_indents.insert(new_row, indent);
        }

        // Keep invariants (trailing empties, gap filling, etc.)
        self.insert_empty();
    }

    /// Handles a return at the given row, i.e. pushes the current row and all rows below down
    /// and inserts an empty row at the current position with the same indent.
    pub fn return_at(&mut self, row: u32) {
        // Get the indent of the current row before shifting
        let current_indent = self.effective_indent(row);

        // Shift rows >= `row` down by 1 (including the current row)
        let mut to_shift: Vec<((u32, u32), CellItem)> = Vec::new();
        for (&(col, r), cell) in &self.grid {
            if r >= row {
                to_shift.push(((col, r), cell.clone()));
            }
        }
        for ((col, r), _) in &to_shift {
            self.grid.remove(&(*col, *r));
            self.grid_rects.remove(&(*col, *r));
        }
        for ((col, r), cell) in to_shift {
            self.grid.insert((col, r + 1), cell);
        }

        // Update the indent map for shifted rows
        let mut new_indents = FxHashMap::default();
        for (&r, &ind) in &self.row_indents {
            if r >= row {
                new_indents.insert(r + 1, ind);
            } else {
                new_indents.insert(r, ind);
            }
        }

        // Insert a new empty row at the current position with the same indent
        self.grid.insert((0, row), CellItem::new(Cell::Empty));
        new_indents.insert(row, current_indent);

        self.row_indents = new_indents;
    }

    /// Handles deletion/backspace at the given row.
    /// If the current row is empty, delete it. Otherwise, if the previous row exists and is empty,
    /// delete that previous row. Otherwise, delete the current row. Then shift rows below up by one
    /// and update indents accordingly. Finally, restore grid invariants via `insert_empty()`.
    pub fn delete_at(&mut self, row: u32) {
        // Helper: does a row exist and is it all empty?
        let row_exists = |rr: u32| -> bool { self.grid.keys().any(|&(_, r)| r == rr) };
        let row_all_empty = |rr: u32| -> bool {
            let mut any = false;
            for ((_, r), cell) in &self.grid {
                if *r == rr {
                    any = true;
                    if !matches!(cell.cell, Cell::Empty) {
                        return false;
                    }
                }
            }
            // If the row has no cells recorded at all, treat it as empty-only if it truly doesn't exist.
            // We use row_exists separately to decide whether it's a candidate for deletion.
            any && true
        };

        // Decide which row to remove.
        let remove_row: u32 = {
            let current_is_empty = row_exists(row) && row_all_empty(row);
            if current_is_empty {
                row
            } else if row > 0 {
                let prev = row - 1;
                if row_exists(prev) && row_all_empty(prev) {
                    prev
                } else {
                    row
                }
            } else {
                row
            }
        };

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

        // Remove and reinsert all cells in lower rows, shifting them up by one.
        // IMPORTANT: process in ASCENDING row order to avoid overwriting freshly shifted rows.
        to_shift.sort_by_key(|&((_, r), _)| r);
        for ((col, r), cell) in to_shift {
            // remove original position (if still present) and insert at r-1
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

    /// Move all rows at or below the given row index one line down (shift by 1).
    pub fn move_down_from(&mut self, row: u32) {
        // Collect and remove impacted cells first
        let mut to_shift: Vec<((u32, u32), CellItem)> = Vec::new();
        for (&(col, r), cell) in &self.grid {
            if r >= row {
                to_shift.push(((col, r), cell.clone()));
            }
        }

        // Remove them from current positions
        for ((col, r), _) in &to_shift {
            self.grid.remove(&(*col, *r));
            self.grid_rects.remove(&(*col, *r));
        }

        // Reinsert them shifted down by one
        for ((col, r), cell) in to_shift {
            self.grid.insert((col, r + 1), cell);
        }

        // Update row indents accordingly
        let mut new_indents = FxHashMap::default();
        for (&r, &ind) in &self.row_indents {
            if r >= row {
                new_indents.insert(r + 1, ind);
            } else {
                new_indents.insert(r, ind);
            }
        }
        self.row_indents = new_indents;
    }

    /// Returns the size of the grid.
    pub fn size(
        &mut self,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        folded: bool,
        screen_width: u32,
        event: &str,
        id: u32,
        debug: Option<&DebugModule>,
    ) -> Vec2<u32> {
        let header_height = 35;
        if !folded {
            // Track widths per row and column and heights per row
            let mut row_col_widths: FxHashMap<u32, FxHashMap<u32, u32>> = FxHashMap::default();
            let mut row_heights: FxHashMap<u32, u32> = FxHashMap::default();

            self.grid_rects.clear();

            // First pass: collect individual cell sizes
            for ((col, row), cell) in &self.grid {
                let size = cell.size(ctx, grid_ctx, &(*col, *row), event, id, debug);
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
                let indent_offset = indent * INDENT_WIDTH;

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
                    + header_height
                    + row_heights
                        .keys()
                        .filter(|&&r| r < *row)
                        .map(|r| row_heights[&r])
                        .sum::<u32>();

                // Store the rectangle for this cell
                let size = cell.size(ctx, grid_ctx, &(*col, *row), event, id, debug);
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
                    let indent_offset = indent * INDENT_WIDTH;
                    indent_offset + cols.values().sum::<u32>()
                })
                .max()
                .unwrap_or(0)
                + 4;
            let total_height = row_heights.values().sum::<u32>() + header_height + 4;

            Vec2::new(total_width, total_height)
        } else {
            // When folded, only show header
            Vec2::new(screen_width, header_height + 4)
        }
    }
}
