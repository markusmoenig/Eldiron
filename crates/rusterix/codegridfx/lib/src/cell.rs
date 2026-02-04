use theframework::prelude::*;

/// Assignment operators in the AST
#[derive(Clone, PartialEq, Debug)]
pub enum AssignmentOp {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
}

impl AssignmentOp {
    pub fn describe(&self) -> &str {
        match self {
            AssignmentOp::Assign => "=",
            AssignmentOp::AddAssign => "+=",
            AssignmentOp::SubtractAssign => "-=",
            AssignmentOp::MultiplyAssign => "*=",
            AssignmentOp::DivideAssign => "/=",
        }
    }

    pub fn to_index(&self) -> i32 {
        match self {
            AssignmentOp::Assign => 0,
            AssignmentOp::AddAssign => 1,
            AssignmentOp::SubtractAssign => 2,
            AssignmentOp::MultiplyAssign => 3,
            AssignmentOp::DivideAssign => 4,
        }
    }

    pub fn from_index(idx: i32) -> Option<Self> {
        match idx {
            0 => Some(AssignmentOp::Assign),
            1 => Some(AssignmentOp::AddAssign),
            2 => Some(AssignmentOp::SubtractAssign),
            3 => Some(AssignmentOp::MultiplyAssign),
            4 => Some(AssignmentOp::DivideAssign),
            _ => None,
        }
    }
}

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
    ConstructColorAssignBlock,
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
    PaletteColor(u8),
    Value(String),

    // Python based
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
    SetPlayerCamera,
    SetProximityTracking,
    SetTile,
    Take,
    Teleport,
    ToggleAttr,
    TookDamage,

    // Shader based (sorted)
    Abs,
    Atan,
    Atan2,
    Ceil,
    Clamp,
    Cos,
    Cross,
    Degrees,
    Dot,
    Exp,
    Floor,
    Fract,
    Length,
    Log,
    Max,
    Min,
    Mix,
    Mod,
    Normalize,
    Pow,
    Radians,
    Rand,
    Rotate2d,
    Sign,
    Sin,
    Smoothstep,
    Sample,
    SampleNormal,
    Textures(String),
    Sqrt,
    Step,
    Tan,

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
            Cell::ConstructColorAssignBlock => "Color = ..",
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
            Cell::PaletteColor(_) => "Palette Color",
            Cell::Value(_) => "Value",

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
            Cell::SetPlayerCamera => "Set Player Camera",
            Cell::SetProximityTracking => "Set Proximity Tracking",
            Cell::SetTile => "Set Tile",
            Cell::Take => "Take",
            Cell::Teleport => "Teleport",
            Cell::ToggleAttr => "Toggle Attr",
            Cell::TookDamage => "Took Damage",

            Cell::Abs => "Abs",
            Cell::Atan => "Atan",
            Cell::Atan2 => "Atan2",
            Cell::Ceil => "Ceil",
            Cell::Clamp => "Clamp",
            Cell::Cos => "Cos",
            Cell::Cross => "Cross",
            Cell::Degrees => "Degrees",
            Cell::Dot => "Dot",
            Cell::Exp => "Exp",
            Cell::Floor => "Floor",
            Cell::Fract => "Fract",
            Cell::Length => "Length",
            Cell::Log => "Log",
            Cell::Max => "Max",
            Cell::Min => "Min",
            Cell::Mix => "Mix",
            Cell::Mod => "Mod",
            Cell::Normalize => "Normalize",
            Cell::Pow => "Pow",
            Cell::Radians => "Radians",
            Cell::Rand => "Rand",
            Cell::Rotate2d => "Rotate2d",
            Cell::Sign => "Sign",
            Cell::Sin => "Sin",
            Cell::Smoothstep => "Smoothstep",
            Cell::Sample => "Sample",
            Cell::SampleNormal => "Sample Normal",
            Cell::Textures(_) => "Texture",
            Cell::Sqrt => "Sqrt",
            Cell::Step => "Step",
            Cell::Tan => "Tan",

            Cell::LeftParent => "Left Parenthesis",
            Cell::RightParent => "Right Parenthesis",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Var = .." => Some(Cell::ConstructAssignBlock),
            "Color = .." => Some(Cell::ConstructColorAssignBlock),
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
            "Palette Color" => Some(Cell::PaletteColor(0)),
            "Value" => Some(Cell::Value("1".to_string())),

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
            "set_player_camera" => Some(Cell::SetPlayerCamera),
            "set_proximity_tracking" => Some(Cell::SetProximityTracking),
            "set_tile" => Some(Cell::SetTile),
            "take" => Some(Cell::Take),
            "teleport" => Some(Cell::Teleport),
            "toggle_attr" => Some(Cell::ToggleAttr),
            "took_damage" => Some(Cell::TookDamage),

            "abs" => Some(Cell::Abs),
            "atan" => Some(Cell::Atan),
            "atan2" => Some(Cell::Atan2),
            "ceil" => Some(Cell::Ceil),
            "clamp" => Some(Cell::Clamp),
            "cos" => Some(Cell::Cos),
            "cross" => Some(Cell::Cross),
            "degrees" => Some(Cell::Degrees),
            "dot" => Some(Cell::Dot),
            "exp" => Some(Cell::Exp),
            "floor" => Some(Cell::Floor),
            "fract" => Some(Cell::Fract),
            "length" => Some(Cell::Length),
            "log" => Some(Cell::Log),
            "max" => Some(Cell::Max),
            "min" => Some(Cell::Min),
            "mix" => Some(Cell::Mix),
            "mod" => Some(Cell::Mod),
            "normalize" => Some(Cell::Normalize),
            "pow" => Some(Cell::Pow),
            "radians" => Some(Cell::Radians),
            "rand" => Some(Cell::Rand),
            "rotate2d" => Some(Cell::Rotate2d),
            "sample" => Some(Cell::Sample),
            "sample_normal" => Some(Cell::SampleNormal),
            "textures" => Some(Cell::Textures("value".into())),
            "sign" => Some(Cell::Sign),
            "sin" => Some(Cell::Sin),
            "smoothstep" => Some(Cell::Smoothstep),
            "sqrt" => Some(Cell::Sqrt),
            "step" => Some(Cell::Step),
            "tan" => Some(Cell::Tan),

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
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Str(value) => {
                if value.contains("\"") {
                    value.clone()
                } else {
                    format!("\"{}\"", value)
                }
            }
            PaletteColor(idx) => {
                format!("palette({})", idx)
            }
            Value(value) => value.clone(),

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
            SetPlayerCamera => "set_player_camera".into(),
            SetProximityTracking => "set_proximity_tracking".into(),
            SetTile => "set_tile".into(),
            Take => "take".into(),
            Teleport => "teleport".into(),
            ToggleAttr => "toggle_attr".into(),
            TookDamage => "took_damage".into(),

            Abs => "abs".into(),
            Atan => "atan".into(),
            Atan2 => "atan2".into(),
            Ceil => "ceil".into(),
            Clamp => "clamp".into(),
            Cos => "cos".into(),
            Cross => "cross".into(),
            Degrees => "degrees".into(),
            Dot => "dot".into(),
            Exp => "exp".into(),
            Floor => "floor".into(),
            Fract => "fract".into(),
            Length => "length".into(),
            Log => "log".into(),
            Max => "max".into(),
            Min => "min".into(),
            Mix => "mix".into(),
            Mod => "mod".into(),
            Normalize => "normalize".into(),
            Pow => "pow".into(),
            Radians => "radians".into(),
            Rand => "rand".into(),
            Rotate2d => "rotate2d".into(),
            Sample => "sample".into(),
            SampleNormal => "sample_normal".into(),
            Textures(name) => format!("\"{}\"", name),
            Sign => "sign".into(),
            Sin => "sin".into(),
            Smoothstep => "smoothstep".into(),
            Sqrt => "sqrt".into(),
            Step => "step".into(),
            Tan => "tan".into(),

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
            SetPlayerCamera => {
                "Sets the player camera: '2d', 'iso' or 'firstp'.".into()
            }
            SetProximityTracking => {
                "Enable / disable tracking of entities for the current entity or item.".into()
            }
            SetTile => "Set the tile ID for the current entity or item.".into(),
            Take => "Take the item with the given ID.".into(),
            Teleport => "Teleport to a sector. Optionally in another region.".into(),
            ToggleAttr => "Toggles a boolean attribute of the current entity or item.".into(),
            TookDamage => "Takes damage.".into(),

            Abs => "Absolute value of x.".into(),
            Atan => "Arc tangent of y/x (single-arg).".into(),
            Atan2 => "Arc tangent of y/x using signs of both to determine quadrant.".into(),
            Ceil => "Ceiling: round x up to the next integer.".into(),
            Clamp => "Clamp x to the range [min, max].".into(),
            Cos => "Cosine of angle (radians).".into(),
            Cross => "3D cross product.".into(),
            Degrees => "Convert radians to degrees.".into(),
            Dot => "Dot product.".into(),
            Exp => "Exponential e^x.".into(),
            Floor => "Floor: round x down to the previous integer.".into(),
            Fract => "Fractional part of x.".into(),
            Length => "Vector length / magnitude.".into(),
            Log => "Natural logarithm.".into(),
            Max => "Component-wise maximum of x and y.".into(),
            Min => "Component-wise minimum of x and y.".into(),
            Mix => "Linear interpolation: mix(a, b, t).".into(),
            Mod => "Remainder of x/y with sign of x.".into(),
            Normalize => "Normalize a vector to unit length.".into(),
            Pow => "Power: x^y.".into(),
            Radians => "Convert degrees to radians.".into(),
            Rand => "Random number in [0,1).".into(),
            Rotate2d => "Rotate a 2D vector by an angle (in degrees).".into(),
            Sign => "Sign of x (-1, 0, or 1) component-wise.".into(),
            Sin => "Sine of angle (radians).".into(),
            Smoothstep => "Hermite smooth interpolation between edge0 and edge1.".into(),
            Sqrt => "Square root.".into(),
            Step => "Step function: 0 if x < edge, else 1.".into(),
            Tan => "Tangent of angle (radians).".into(),
            Sample => "Sample a noise or pattern texture.".into(),
            SampleNormal => "Sample the normal of a noise or pattern texture.".into(),

            _ => "".into(),
        }
    }

    pub fn role(&self) -> CellRole {
        match &self {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) | Textures(_) | Value(_)
            | PaletteColor(_) => CellRole::Value,
            Assignment | Comparison(_) | If | Else | Arithmetic(_) => CellRole::Operator,
            Empty => CellRole::None,

            _ => CellRole::Function,
        }
    }
}
