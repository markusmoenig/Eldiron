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
    NotEqual,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
}
impl ComparisonOp {
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(ComparisonOp::Equal),
            1 => Some(ComparisonOp::NotEqual),
            2 => Some(ComparisonOp::LessEqual),
            3 => Some(ComparisonOp::GreaterEqual),
            4 => Some(ComparisonOp::Less),
            5 => Some(ComparisonOp::Greater),
            _ => None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            ComparisonOp::Equal => 0,
            ComparisonOp::NotEqual => 1,
            ComparisonOp::LessEqual => 2,
            ComparisonOp::GreaterEqual => 3,
            ComparisonOp::Less => 4,
            ComparisonOp::Greater => 5,
        }
    }
    pub fn to_string(&self) -> &'static str {
        match self {
            ComparisonOp::Equal => "==",
            ComparisonOp::NotEqual => "!=",
            ComparisonOp::LessEqual => "<=",
            ComparisonOp::GreaterEqual => ">=",
            ComparisonOp::Less => "<",
            ComparisonOp::Greater => ">",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Cell {
    ConstructAssignBlock,
    ConstructIfBlock,

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
    Else,

    Action,
    AddItem,
    BlockEvents,
    CloseIn,
    DealDamage,
    Drop,
    DropItems,
    EntitiesInRadius,
    Equip,
    GetAttr,
    GetAttrOf,
    GetEntityAttr,
    Goto,
    Id,
    Intent,
    InventoryItems,
    InventoryItemsOf,
    Message,
    NotifyIn,
    OfferInventory,
    Random,
    RandomWalk,
    RandomWalkInSector,
    SetAttr,
    SetEmitLight,
    SetProximityTracking,
    SetTile,
    Take,
    Teleport,
    ToggleAttr,
    TookDamage,

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
            Cell::ConstructAssignBlock => "Var = ..",
            Cell::ConstructIfBlock => "If .. == ..",

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
            Cell::Else => "Else",

            Cell::Action => "Action",
            Cell::AddItem => "Add Item",
            Cell::BlockEvents => "Block Events",
            Cell::CloseIn => "Close In",
            Cell::DealDamage => "Deal Damage",
            Cell::Drop => "Drop",
            Cell::DropItems => "Drop Items",
            Cell::EntitiesInRadius => "Entities in Radius",
            Cell::Equip => "Equip",
            Cell::GetAttr => "Get Attribute",
            Cell::GetAttrOf => "Get Attribute Of",
            Cell::GetEntityAttr => "Get Attribute Of",
            Cell::Goto => "Goto",
            Cell::Id => "Id",
            Cell::Intent => "Intent",
            Cell::InventoryItems => "Inventory Items",
            Cell::InventoryItemsOf => "Inventory Items Of",
            Cell::Message => "Message",
            Cell::NotifyIn => "Notify In",
            Cell::OfferInventory => "Offer Inventory",
            Cell::Random => "Random Number",
            Cell::RandomWalk => "Random Walk",
            Cell::RandomWalkInSector => "Random Walk In Sector",
            Cell::SetAttr => "Set Attribute",
            Cell::SetEmitLight => "Set Emit Light",
            Cell::SetProximityTracking => "Set Proximity Tracking",
            Cell::SetTile => "Set Tile",
            Cell::Take => "Take",
            Cell::Teleport => "Teleport",
            Cell::ToggleAttr => "toggle_attr",
            Cell::TookDamage => "took_damage",

            Cell::LeftParent => "Left Parenthesis",
            Cell::RightParent => "Right Parenthesis",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Var = .." => Some(Cell::ConstructAssignBlock),
            "If .. == .." => Some(Cell::ConstructIfBlock),

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
            "Else" => Some(Cell::Else),

            "action" => Some(Cell::Action),
            "add_item" => Some(Cell::AddItem),
            "block_events" => Some(Cell::BlockEvents),
            "close_in" => Some(Cell::CloseIn),
            "deal_damage" => Some(Cell::DealDamage),
            "drop" => Some(Cell::Drop),
            "drop_items" => Some(Cell::DropItems),
            "entities_in_radius" => Some(Cell::EntitiesInRadius),
            "equip" => Some(Cell::Equip),
            "get_attr" => Some(Cell::GetAttr),
            "get_attr_of" => Some(Cell::GetAttrOf),
            "goto" => Some(Cell::Goto),
            "id" => Some(Cell::Id),
            "intent" => Some(Cell::Intent),
            "inventory_items" => Some(Cell::InventoryItems),
            "inventory_items_of" => Some(Cell::InventoryItemsOf),
            "message" => Some(Cell::Message),
            "notify_in" => Some(Cell::NotifyIn),
            "offer_inventory" => Some(Cell::OfferInventory),
            "random" => Some(Cell::Random),
            "random_walk" => Some(Cell::RandomWalk),
            "random_walk_in_sector" => Some(Cell::RandomWalkInSector),
            "set_attr" => Some(Cell::SetAttr),
            "set_emit_light" => Some(Cell::SetEmitLight),
            "set_proximity_tracking" => Some(Cell::SetProximityTracking),
            "set_tile" => Some(Cell::SetTile),
            "take" => Some(Cell::Take),
            "teleport" => Some(Cell::Teleport),
            "toggle_attr" => Some(Cell::ToggleAttr),
            "took_damage" => Some(Cell::TookDamage),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match &self {
            Variable(var_name) => {
                if var_name == "myself" {
                    "id()".to_string()
                } else {
                    var_name.clone()
                }
            }
            Integer(value) | Float(value) => value.clone(),
            Boolean(value) => {
                if *value {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            Str(value) => {
                if value.contains("\"") {
                    value.clone()
                } else {
                    format!("\"{}\"", value)
                }
            }

            Assignment => "=".into(),
            Comparison(op) => op.to_string().to_string(),
            Arithmetic(op) => op.to_string().to_string(),
            If => "if".into(),
            Else => "else".into(),

            Action => "action".into(),
            AddItem => "add_item".into(),
            BlockEvents => "block_events".into(),
            CloseIn => "close_in".into(),
            DealDamage => "deal_damage".into(),
            Drop => "drop".into(),
            DropItems => "drop_items".into(),
            EntitiesInRadius => "entities_in_radius".into(),
            Equip => "equip".into(),
            GetAttr => "get_attr".into(),
            GetAttrOf => "get_attr_of".into(),
            Goto => "goto".into(),
            Id => "id".into(),
            Intent => "intent".into(),
            InventoryItems => "inventory_items".into(),
            InventoryItemsOf => "inventory_items_of".into(),
            Message => "message".into(),
            NotifyIn => "notify_in".into(),
            OfferInventory => "offer_inventory".into(),
            Random => "random".into(),
            RandomWalk => "random_walk".into(),
            RandomWalkInSector => "random_walk_in_sector".into(),
            SetAttr => "set_attr".into(),
            SetEmitLight => "set_emit_light".into(),
            SetProximityTracking => "set_proximity_tracking".into(),
            SetTile => "set_tile".into(),
            Take => "take".into(),
            Teleport => "teleport".into(),
            ToggleAttr => "toggle_attr".into(),
            TookDamage => "took_damage".into(),

            LeftParent => "(".into(),
            RightParent => ")".into(),
            _ => "".into(),
        }
    }

    pub fn status(&self) -> String {
        match &self {
            Action => "Player based action.".into(),
            AddItem => "Add an item to the inventory of the current entity.".into(),
            BlockEvents => "Block specific events for a period of in-game minutes for the current entity or item.".into(),
            CloseIn => "Close in to the target entities within the given radius and speed.".into(),
            DealDamage => "Deal damage to the given entity or item.".into(),
            Drop => "Drop the item of the given ID for the current entity.".into(),
            DropItems => "Drop all or filtered items for the current entity.".into(),
            EntitiesInRadius => {
                "Returns a list of entity IDs in the radius of the current entity or item.".into()
            }
            GetAttr => "Get an attribute of the current entity or item.".into(),
            GetAttrOf => "Get an attribute of the given entity or item.".into(),
            Goto => "Go to a sector using pathfinding.".into(),
            Id => "Returns the ID of the current entity or item.".into(),
            Intent=> "Player intent. Only applicable during for user events.".into(),
            InventoryItems => {
                "Returns a list of item IDs of the inventory of the current entity.".into()
            }
            InventoryItemsOf => "Returns a list of item IDs of the item with the given ID.".into(),
            Message => "Send a message to an entity.".into(),
            NotifyIn => "Send the given event after the given amount of in-game minutes.".into(),
            OfferInventory => "Offer the inventory for sale to the given entity.".into(),
            Random => "Generate a random number within an open range.".into(),
            RandomWalk => "Randomly walk.".into(),
            RandomWalkInSector => "Randomly walk in the entities current sector.".into(),
            SetAttr => "Set an attribute of the current entity or item.".into(),
            SetEmitLight => "Set the light emission state of the current entity or item.".into(),
            SetProximityTracking => {
                "Enable / disable tracking of entities for the current entity or item.".into()
            }
            SetTile => "Set the tile ID for the current entity or item.".into(),
            Take => "Take the item with the given ID.".into(),
            Teleport => "Teleport to a sector. Optionally in another region.".into(),
            ToggleAttr => "Toggles a boolean attribute of the current entity or item.".into(),
            TookDamage => "Takes damage.".into(),
            _ => "".into(),
        }
    }

    pub fn role(&self) -> CellRole {
        match &self {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) => CellRole::Value,
            Assignment | Comparison(_) | If | Else => CellRole::Operator,
            Empty => CellRole::None,

            _ => CellRole::Function,
        }
    }
}
