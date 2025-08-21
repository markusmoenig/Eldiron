use crate::{Cell, CellItem};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Group {
    Assignment,
}

use Group::*;

impl Group {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "CGFAssignment" => Some(Group::Assignment),
            _ => None,
        }
    }

    /// Create the cells for this group
    pub fn create_cells(&self) -> Vec<CellItem> {
        let mut cells = vec![];

        match &self {
            Assignment => {
                cells.push(CellItem::new(Cell::Variable("Unnamed".into())));
                cells.push(CellItem::new(Cell::Assign));
                cells.push(CellItem::new(Cell::Value));
            }
        }

        cells
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellGroup {
    pub id: Uuid,
    pub group: Group,
    pub items: FxHashSet<Uuid>,
}

impl CellGroup {
    pub fn new(group: Group) -> Self {
        Self {
            id: Uuid::new_v4(),
            group,
            items: FxHashSet::default(),
        }
    }
}
