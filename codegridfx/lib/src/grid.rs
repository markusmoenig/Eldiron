use crate::{Cell, CellItem};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Grid {
    #[serde(with = "vectorize")]
    pub grid: FxHashMap<(u32, u32), CellItem>,
    #[serde(with = "vectorize")]
    pub grid_rects: FxHashMap<(u32, u32), TheDim>,
}

impl Grid {
    pub fn insert(&mut self, at: (u32, u32), item: CellItem) {
        self.grid.insert(at, item);
    }

    /// Make sure there is an empty cell at the end of each row and at the bottom row.
    pub fn insert_empty(&mut self) {
        // Add an empty cell at the end of each row only if not already empty
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

        // Only add a new bottom row if the current bottom row contains any non-empty cells
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
        if !all_empty {
            self.grid
                .insert((0, max_row + 1), CellItem::new(Cell::Empty));
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
}
