// use crate::GridCtx;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Cell {
    Empty,
    Variable(String),
    Number(String),
    Str(String),
    Boolean(bool),
    Assignment,
    GetAttr,
    SetAttr,

    Comma,
    LeftParent,
    RightParent,
}

use Cell::*;

impl Cell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Empty" => Some(Cell::Empty),
            "Variable" => Some(Cell::Variable("Unnamed".into())),
            "Number" => Some(Cell::Number("0".into())),
            "String" => Some(Cell::Str("".into())),
            "Boolean" => Some(Cell::Boolean(true)),
            "Assignment" => Some(Cell::Assignment),
            "get_attr" => Some(Cell::GetAttr),
            "set_attr" => Some(Cell::SetAttr),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match &self {
            Variable(var_name) => var_name.clone(),
            Number(value_name) => value_name.clone(),
            Str(value) => format!("\"{}\"", value),
            GetAttr => "get_attr".into(),
            SetAttr => "set_attr".into(),

            Comma => ", ".into(),
            LeftParent => "(".into(),
            RightParent => ")".into(),
            Assignment => "=".into(),
            _ => "".into(),
        }
    }
}
