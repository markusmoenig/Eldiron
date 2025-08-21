// use crate::GridCtx;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Cell {
    Empty,
    Variable(String),
    Value,
    Assign,
    GetAttr,
    SetAttr,
}

// use Cell::*;

impl Cell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Empty" => Some(Cell::Empty),
            "Variable" => Some(Cell::Variable("Unnamed".into())),
            "Value" => Some(Cell::Value),
            "Assign" => Some(Cell::Assign),
            "GetAttr" => Some(Cell::GetAttr),
            "SetAttr" => Some(Cell::SetAttr),
            _ => None,
        }
    }
}
