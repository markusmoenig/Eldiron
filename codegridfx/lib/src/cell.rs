// use crate::GridCtx;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Cell {
    Empty,
    Variable(String),
    Value(String),
    Assignment,
    GetAttr,
    SetAttr,

    Comma,
    LeftParent,
    RightParent,
}

// use Cell::*;

impl Cell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Empty" => Some(Cell::Empty),
            "Variable" => Some(Cell::Variable("Unnamed".into())),
            "Value" => Some(Cell::Value("0".into())),
            "Assignment" => Some(Cell::Assignment),
            "GetAttr" => Some(Cell::GetAttr),
            "SetAttr" => Some(Cell::SetAttr),
            _ => None,
        }
    }
}
