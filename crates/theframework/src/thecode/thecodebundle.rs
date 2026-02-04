use crate::prelude::*;

/// TheCodeBundle is a collections of codegrids which make up the behavior of an entity.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TheCodeBundle {
    pub name: String,
    pub id: Uuid,
    pub selected_grid_id: Option<Uuid>,
    pub grids: FxHashMap<Uuid, TheCodeGrid>,
}

impl Default for TheCodeBundle {
    fn default() -> Self {
        TheCodeBundle::new()
    }
}

impl TheCodeBundle {
    pub fn new() -> Self {
        let grids = FxHashMap::default();
        //let def = TheCodeGrid::default();
        //grids.insert(def.uuid, def);

        Self {
            name: "Unnamed".to_string(),
            id: Uuid::new_v4(),
            selected_grid_id: None,
            grids,
        }
    }

    /// Insert a codegrid into the bundle.
    pub fn insert_grid(&mut self, grid: TheCodeGrid) {
        self.grids.insert(grid.id, grid);
    }

    /// Get a grid from the module.
    pub fn get_grid(&self, id: &Uuid) -> Option<&TheCodeGrid> {
        self.grids.get(id)
    }

    /// Get a mutable grid from the module.
    pub fn get_grid_mut(&mut self, id: &Uuid) -> Option<&mut TheCodeGrid> {
        self.grids.get_mut(id)
    }

    /// Returns a list of all codegrid keys in the bundle sorted by their name.
    pub fn sorted(&self) -> Vec<Uuid> {
        let mut entries: Vec<(Uuid, String)> = self
            .grids
            .iter()
            .map(|(uuid, data)| (*uuid, data.name.clone()))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));
        entries.into_iter().map(|(uuid, _)| uuid).collect()
    }

    /// Move all positions by the given amount.
    pub fn move_positions_by(&mut self, move_by: Vec2<i32>) {
        for grid in self.grids.values_mut() {
            grid.move_positions_by(move_by);
        }
    }
}
