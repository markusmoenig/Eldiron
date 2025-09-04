use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl ArithmeticOp {
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(ArithmeticOp::Add),
            1 => Some(ArithmeticOp::Subtract),
            2 => Some(ArithmeticOp::Multiply),
            3 => Some(ArithmeticOp::Divide),
            _ => None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            ArithmeticOp::Add => 0,
            ArithmeticOp::Subtract => 1,
            ArithmeticOp::Multiply => 2,
            ArithmeticOp::Divide => 3,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            ArithmeticOp::Add => "+",
            ArithmeticOp::Subtract => "-",
            ArithmeticOp::Multiply => "*",
            ArithmeticOp::Divide => "/",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ComparisonOp {
    Equal,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
}
impl ComparisonOp {
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(ComparisonOp::Equal),
            1 => Some(ComparisonOp::LessEqual),
            2 => Some(ComparisonOp::GreaterEqual),
            3 => Some(ComparisonOp::Less),
            4 => Some(ComparisonOp::Greater),
            _ => None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            ComparisonOp::Equal => 0,
            ComparisonOp::LessEqual => 1,
            ComparisonOp::GreaterEqual => 2,
            ComparisonOp::Less => 3,
            ComparisonOp::Greater => 4,
        }
    }
    pub fn to_string(&self) -> &'static str {
        match self {
            ComparisonOp::Equal => "==",
            ComparisonOp::LessEqual => "<=",
            ComparisonOp::GreaterEqual => ">=",
            ComparisonOp::Less => "<",
            ComparisonOp::Greater => ">",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Variable(String),
    Integer(String),
    Float(String),
    Str(String),
    Boolean(bool),
    Assignment,
    Comparison(ComparisonOp),
    Arithmetic(ArithmeticOp),
    If,

    AddItem,
    GetAttr,
    GetEntityAttr,
    GetItemAttr,
    Random,
    RandomWalk,
    RandomWalkInSector,

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
    Event,
}

impl CellRole {
    pub fn to_color(&self) -> [u8; 4] {
        match self {
            CellRole::None => [180, 180, 180, 255],
            CellRole::Operator => [200, 195, 150, 255],
            CellRole::Value => [160, 185, 160, 255],
            CellRole::Function => [160, 175, 190, 255],
            CellRole::Event => [195, 170, 150, 255],
        }
    }
}

use Cell::*;

impl Cell {
    pub fn description(&self) -> &'static str {
        match self {
            Cell::Empty => "Empty",
            Cell::Variable(_) => "Variable",
            Cell::Integer(_) => "Integer",
            Cell::Float(_) => "Float",
            Cell::Str(_) => "String",
            Cell::Boolean(_) => "Boolean",
            Cell::Assignment => "Assignment",
            Cell::Comparison(_) => "Comparison",
            Cell::Arithmetic(_) => "Arithmetic",
            Cell::If => "If",
            Cell::AddItem => "Add Item",
            Cell::GetAttr => "Get Attribute",
            Cell::GetEntityAttr => "Get Entity Attribute",
            Cell::GetItemAttr => "Get Item Attribute",
            Cell::Random => "Random Number",
            Cell::RandomWalk => "Random Walk",
            Cell::RandomWalkInSector => "Random Walk In Sector",
            Cell::SetAttr => "Set Attribute",
            Cell::LeftParent => "Left Parenthesis",
            Cell::RightParent => "Right Parenthesis",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Empty" => Some(Cell::Empty),
            "Variable" => Some(Cell::Variable("Unnamed".into())),
            "Integer" => Some(Cell::Integer("0".into())),
            "Float" => Some(Cell::Float("0.0".into())),
            "String" => Some(Cell::Str("".into())),
            "Boolean" => Some(Cell::Boolean(true)),
            "Assignment" => Some(Cell::Assignment),
            "Comparison" => Some(Cell::Comparison(ComparisonOp::Equal)),
            "Arithmetic" => Some(Cell::Arithmetic(ArithmeticOp::Add)),
            "If" => Some(Cell::If),

            "add_item" => Some(Cell::AddItem),
            "get_attr" => Some(Cell::GetAttr),
            "get_entity_attr" => Some(Cell::GetEntityAttr),
            "get_item_attr" => Some(Cell::GetItemAttr),

            "random" => Some(Cell::Random),
            "random_walk" => Some(Cell::RandomWalk),
            "random_walk_in_sector" => Some(Cell::RandomWalkInSector),
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

            Assignment => "=".into(),
            Comparison(op) => op.to_string().to_string(),
            Arithmetic(op) => op.to_string().to_string(),
            If => "if".into(),

            AddItem => "add_item".into(),
            GetAttr => "get_attr".into(),
            GetEntityAttr => "get_entity_attr".into(),
            GetItemAttr => "get_item_attr".into(),
            Random => "random".into(),
            RandomWalk => "random_walk".into(),
            RandomWalkInSector => "random_walk_in_sector".into(),
            SetAttr => "set_attr".into(),

            LeftParent => "(".into(),
            RightParent => ")".into(),
            _ => "".into(),
        }
    }

    pub fn status(&self) -> String {
        match &self {
            AddItem => "Add an item to the inventory of the current entity.".into(),
            GetAttr => "Get an attribute of the current entity or item.".into(),
            GetEntityAttr => "Get an attribute of the given entity id.".into(),
            GetItemAttr => "Get an attribute of the given item id.".into(),
            Random => "Generate a random number within an open range.".into(),
            RandomWalk => "Randomly walk.".into(),
            RandomWalkInSector => "Randomly walk in the entities current sector.".into(),
            SetAttr => "Set an attribute of the current entity or item.".into(),
            _ => "".into(),
        }
    }

    pub fn role(&self) -> CellRole {
        match &self {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) => CellRole::Value,
            Assignment | Comparison(_) | If => CellRole::Operator,
            Empty => CellRole::None,

            _ => CellRole::Function,
        }
    }
}
