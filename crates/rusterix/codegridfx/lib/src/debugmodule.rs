use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct DebugModule {
    #[serde(with = "vectorize")]
    pub ids: FxHashMap<u32, Vec<DebugGrid>>,
}

impl DebugModule {
    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn clear_execution(&mut self) {
        for grids in self.ids.values_mut() {
            for grid in grids {
                grid.executed.clear();
                grid.executed_rows.clear();
                grid.not_taken_rows.clear();
                grid.header_executed = false;
            }
        }
    }

    /// Merge the content of another Debug into this one.
    /// Values and errors from `other` will be inserted into or overwrite existing entries.
    pub fn merge(&mut self, other: &DebugModule) {
        for (id, other_grids) in &other.ids {
            let grids = self.ids.entry(*id).or_default();
            for other_grid in other_grids {
                let grid = if let Some(pos) = grids.iter().position(|g| g.name == other_grid.name) {
                    grids.get_mut(pos).unwrap()
                } else {
                    grids.push(DebugGrid::new(&other_grid.name));
                    grids.last_mut().unwrap()
                };
                // Merge errors
                for err in &other_grid.errors {
                    grid.errors.insert(*err);
                }
                // Merge execution markers
                for pos in &other_grid.executed {
                    grid.executed.insert(*pos);
                }
                for row in &other_grid.executed_rows {
                    grid.executed_rows.insert(*row);
                }
                for row in &other_grid.not_taken_rows {
                    grid.not_taken_rows.insert(*row);
                }
                grid.header_executed |= other_grid.header_executed;
                // Merge results
                for (pos, val) in &other_grid.result {
                    grid.result.insert(*pos, val.clone());
                }
            }
        }
    }

    /// Add or overwrite a value in the specified routine (by name) at (x, y).
    /// Creates the routine if it does not yet exist. Returns true if the value was written.
    pub fn add_value(
        &mut self,
        id: u32,
        routine_name: &str,
        x: u32,
        y: u32,
        value: TheValue,
    ) -> bool {
        let grid = self.ensure_grid(id, routine_name);
        grid.executed.insert((x, y));
        grid.executed_rows.insert(y);
        grid.result.insert((x, y), value);
        true
    }

    /// Record an error coordinate for a routine (by name). Creates the routine if needed.
    pub fn add_error(&mut self, id: u32, routine_name: &str, x: u32, y: u32) {
        let grid = self.ensure_grid(id, routine_name);
        grid.executed.insert((x, y));
        grid.executed_rows.insert(y);
        grid.errors.insert((x, y));
    }

    /// Remove an error coordinate for a routine (by name). Creates the routine if needed.
    pub fn remove_error(&mut self, id: u32, routine_name: &str, x: u32, y: u32) {
        let grid = self.ensure_grid(id, routine_name);
        grid.executed.insert((x, y));
        grid.executed_rows.insert(y);
        grid.errors.remove(&(x, y));
    }

    /// Record that a cell was executed in the current debug snapshot.
    pub fn mark_executed(&mut self, id: u32, routine_name: &str, x: u32, y: u32) {
        let grid = self.ensure_grid(id, routine_name);
        grid.executed.insert((x, y));
        grid.executed_rows.insert(y);
    }

    pub fn mark_condition(
        &mut self,
        id: u32,
        routine_name: &str,
        x: u32,
        y: u32,
        taken: bool,
        value: TheValue,
    ) {
        let grid = self.ensure_grid(id, routine_name);
        grid.executed.insert((x, y));
        grid.executed_rows.insert(y);
        grid.result.insert((x, y), value);
        grid.errors.remove(&(x, y));
        if taken {
            grid.not_taken_rows.remove(&y);
        } else {
            grid.not_taken_rows.insert(y);
        }
    }

    /// Record that a routine header was executed in the current debug snapshot.
    pub fn mark_header_executed(&mut self, id: u32, routine_name: &str) {
        let grid = self.ensure_grid(id, routine_name);
        grid.header_executed = true;
    }

    /// Get an immutable reference to a routine (DebugGrid) by id and name.
    pub fn get_routine(&self, id: u32, routine_name: &str) -> Option<&DebugGrid> {
        self.ids.get(&id)?.iter().find(|g| g.name == routine_name)
    }

    /// Get a value at (x, y) from a routine by id and name.
    pub fn get_value(&self, id: u32, routine_name: &str, x: u32, y: u32) -> Option<&TheValue> {
        self.get_routine(id, routine_name)?.result.get(&(x, y))
    }

    /// Get a reference to the error set of a routine by id and name.
    pub fn get_errors(&self, id: u32, routine_name: &str) -> Option<&FxHashSet<(u32, u32)>> {
        self.get_routine(id, routine_name).map(|g| &g.errors)
    }

    /// Check if a routine has an error at the given (x, y) position.
    pub fn has_error(&self, id: u32, routine_name: &str, x: u32, y: u32) -> bool {
        if let Some(grid) = self.get_routine(id, routine_name) {
            grid.errors.contains(&(x, y))
        } else {
            false
        }
    }

    /// Check if a routine cell executed in the current debug snapshot.
    pub fn was_executed(&self, id: u32, routine_name: &str, x: u32, y: u32) -> bool {
        if let Some(grid) = self.get_routine(id, routine_name) {
            grid.executed.contains(&(x, y))
        } else {
            false
        }
    }

    /// Check if a routine header executed in the current debug snapshot.
    pub fn header_was_executed(&self, id: u32, routine_name: &str) -> bool {
        if let Some(grid) = self.get_routine(id, routine_name) {
            grid.header_executed
        } else {
            false
        }
    }

    /// Check if a routine row executed in the current debug snapshot.
    pub fn row_was_executed(&self, id: u32, routine_name: &str, y: u32) -> bool {
        if let Some(grid) = self.get_routine(id, routine_name) {
            grid.executed_rows.contains(&y)
        } else {
            false
        }
    }

    pub fn row_was_not_taken(&self, id: u32, routine_name: &str, y: u32) -> bool {
        if let Some(grid) = self.get_routine(id, routine_name) {
            grid.not_taken_rows.contains(&y)
        } else {
            false
        }
    }

    fn ensure_grid<'a>(&'a mut self, id: u32, routine_name: &str) -> &'a mut DebugGrid {
        let grids = self.ids.entry(id).or_default();
        if let Some(pos) = grids.iter().position(|g| g.name == routine_name) {
            // Safe due to bounds check above
            return grids.get_mut(pos).unwrap();
        }
        grids.push(DebugGrid::new(routine_name));
        let len = grids.len();
        grids.get_mut(len - 1).unwrap()
    }
}
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct DebugGrid {
    pub name: String,

    pub errors: FxHashSet<(u32, u32)>,
    pub executed: FxHashSet<(u32, u32)>,
    pub executed_rows: FxHashSet<u32>,
    pub not_taken_rows: FxHashSet<u32>,
    pub header_executed: bool,
    pub result: FxHashMap<(u32, u32), TheValue>,
}

impl DebugGrid {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}
