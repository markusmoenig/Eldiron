// use crate::GridCtx;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Variable(String),
    Integer(String),
    Float(String),
    Str(String),
    Boolean(bool),
    Assignment,
    GetAttr,
    SetAttr,

    LeftParent,
    RightParent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CellRole {
    None,
    Operator,
    Value,
    Function,
}

impl CellRole {
    pub fn to_color(&self) -> [u8; 4] {
        match self {
            CellRole::None => [180, 180, 180, 255],
            CellRole::Operator => [200, 195, 150, 255],
            // CellRole::Operator => [195, 170, 150, 255],
            CellRole::Value => [160, 185, 160, 255],
            CellRole::Function => [160, 175, 190, 255],
        }
    }
}

use Cell::*;

impl Cell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Empty" => Some(Cell::Empty),
            "Variable" => Some(Cell::Variable("Unnamed".into())),
            "Integer" => Some(Cell::Integer("0".into())),
            "Float" => Some(Cell::Float("0.0".into())),
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
            Integer(value) | Float(value) => value.clone(),
            Boolean(value) => {
                if *value {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            Str(value) => format!("\"{}\"", value),
            GetAttr => "get_attr".into(),
            SetAttr => "set_attr".into(),

            LeftParent => "(".into(),
            RightParent => ")".into(),
            Assignment => "=".into(),
            _ => "".into(),
        }
    }

    pub fn role(&self) -> CellRole {
        match &self {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) => CellRole::Value,
            Assignment => CellRole::Operator,
            GetAttr | SetAttr => CellRole::Function,

            _ => CellRole::None,
        }
    }
}
