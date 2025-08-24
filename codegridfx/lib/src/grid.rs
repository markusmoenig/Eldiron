use crate::CellItem;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Grid {
    pub grid: FxHashMap<(u32, u32), CellItem>,
    pub grid_rects: FxHashMap<(u32, u32), TheDim>,
}

impl Grid {
    pub fn insert(&mut self, at: (u32, u32), item: CellItem) {
        self.grid.insert(at, item);
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
