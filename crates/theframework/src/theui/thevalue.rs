use crate::prelude::*;
use std::ops::RangeInclusive;

/// Macro for math operations on TheValue.
macro_rules! impl_arithmetic_op {
    ($fn_name:ident, $op:tt) => {
        pub fn $fn_name(&self, other: &TheValue) -> Option<TheValue> {
            if let TheValue::Int(a) = self {
                match other {
                    TheValue::Int(b) => Some(TheValue::Int(a $op b)),
                    TheValue::Float(b) => Some(TheValue::Float(*a as f32 $op b)),
                    _ => None,
                }
            }
            else if let TheValue::Float(a) = self {
                match other {
                    TheValue::Int(b) => Some(TheValue::Float(a $op *b as f32)),
                    TheValue::Float(b) => Some(TheValue::Float(a $op b)),
                    _ => None,
                }
            }
            else if let TheValue::Position(a) = self {
                match other {
                    TheValue::Int(b) => Some(TheValue::Position(Vec3::new(a.x $op *b as f32, a.y, a.z))),
                    TheValue::Int2(b) => Some(TheValue::Position(Vec3::new(
                        a.x $op b.x as f32,
                        a.z $op b.y as f32,
                        a.z,
                    ))),
                    TheValue::Int3(b) => Some(TheValue::Position(Vec3::new(
                        a.x $op b.x as f32,
                        a.y $op b.y as f32,
                        a.z $op b.z as f32,
                    ))),
                    TheValue::Float(b) => Some(TheValue::Position(Vec3::new(a.x $op *b, a.y, a.z))),
                    TheValue::Float2(b) => Some(TheValue::Position(Vec3::new(a.x $op b.x, a.y, a.z $op b.y))),
                    TheValue::Float3(b) => {
                        Some(TheValue::Position(Vec3::new(a.x $op b.x, a.y $op b.y, a.z $op b.z)))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    };
}

/// Macro for comparison operations on TheValue.
macro_rules! impl_comparison_op {
    ($fn_name:ident, $op:tt) => {
        pub fn $fn_name(&self, other: &TheValue) -> bool {
            let mut rc = false;
            // Compare by converting to f32
            if let Some(self_f) = self.as_f32() {
                if let Some(other_f) = other.as_f32() {
                    rc = self_f $op other_f;
                }
            }
            // Compare by converting to string
            if let Some(self_string) = self.to_string() {
                if let Some(other_string) = other.to_string() {
                    rc = self_string $op other_string;
                }
            }
            rc
        }
    };
}

/// TheValue contains all possible values used by widgets and layouts. Encapsulating them in an enum alllows easy transfer and comparison of values.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TheValue {
    Empty,
    Bool(bool),
    Float(f32),
    FloatRange(f32, RangeInclusive<f32>),
    Int(i32),
    IntRange(i32, RangeInclusive<i32>),
    Text(String),
    TextList(i32, Vec<String>),
    Char(char),
    Int2(Vec2<i32>),
    Float2(Vec2<f32>),
    Int3(Vec3<i32>),
    Float3(Vec3<f32>),
    Int4(Vec4<i32>),
    Float4(Vec4<f32>),
    Position(Vec3<f32>),
    Tile(String, Uuid),
    KeyCode(TheKeyCode),
    RangeI32(RangeInclusive<i32>),
    RangeF32(RangeInclusive<f32>),
    ColorObject(TheColor),
    PaletteIndex(u16),
    Comparison(TheValueComparison),
    Assignment(TheValueAssignment),
    Id(Uuid),
    Direction(Vec3<f32>),
    List(Vec<TheValue>),
    Time(TheTime),
    TimeDuration(TheTime, TheTime),
    TileMask(TheTileMask),
    Image(TheRGBABuffer),
}

use TheValue::*;

impl TheValue {
    pub fn to_vec2i(&self) -> Option<Vec2<i32>> {
        match self {
            Int2(v) => Some(*v),
            _ => None,
        }
    }

    pub fn to_vec2f(&self) -> Option<Vec2<f32>> {
        match self {
            Float2(v) => Some(*v),
            _ => None,
        }
    }

    pub fn to_vec3f(&self) -> Option<Vec3<f32>> {
        match self {
            Float3(v) => Some(*v),
            ColorObject(color) => Some(color.to_vec3()),
            _ => None,
        }
    }

    pub fn to_i32(&self) -> Option<i32> {
        match self {
            Int(v) => Some(*v),
            IntRange(v, _) => Some(*v),
            Text(t) => t.parse::<i32>().ok(),
            TextList(index, _) => Some(*index),
            PaletteIndex(index) => Some(*index as i32),
            _ => None,
        }
    }

    pub fn to_f32(&self) -> Option<f32> {
        match self {
            Float(v) => Some(*v),
            FloatRange(v, _) => Some(*v),
            Text(t) => t.parse::<f32>().ok(),
            _ => None,
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self {
            Text(v) => Some(v.clone()),
            Tile(name, _id) => Some(name.clone()),
            Time(t) => Some(t.to_time24()),
            _ => None,
        }
    }

    pub fn to_char(&self) -> Option<char> {
        match self {
            Char(v) => Some(*v),
            _ => None,
        }
    }

    pub fn to_key_code(&self) -> Option<TheKeyCode> {
        match self {
            KeyCode(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn to_range_i32(&self) -> Option<RangeInclusive<i32>> {
        match self {
            RangeI32(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn to_range_f32(&self) -> Option<RangeInclusive<f32>> {
        match self {
            RangeF32(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub fn to_color(&self) -> Option<TheColor> {
        match self {
            ColorObject(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// Returns the value as f32 if possible. Used for comparison.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Float(v) => Some(*v),
            Int(v) => Some(*v as f32),
            Bool(v) => Some(*v as i32 as f32),
            Text(t) => t.parse::<f32>().ok(),
            _ => None,
        }
    }

    /// Returns the value as i32 if possible.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Float(v) => Some(*v as i32),
            Int(v) => Some(*v),
            Bool(v) => Some(*v as i32),
            Text(t) => t.parse::<i32>().ok(),
            _ => None,
        }
    }

    /// Test if two values are equal.
    pub fn is_equal(&self, other: &TheValue) -> bool {
        // First test if they are exactly the same value.
        let mut equal = *self == *other;

        // Try to convert to f32 and compare.
        if !equal {
            if let Some(self_f) = self.as_f32() {
                if let Some(other_f) = other.as_f32() {
                    equal = self_f == other_f;
                }
            }
        }
        equal
    }

    // Comparison operations on TheValue other than is_equal.
    impl_comparison_op!(is_greater_than, >);
    impl_comparison_op!(is_less_than, <);
    impl_comparison_op!(is_greater_than_or_equal, >=);
    impl_comparison_op!(is_less_than_or_equal, <=);

    // Math operations on TheValue.
    impl_arithmetic_op!(add, +);
    impl_arithmetic_op!(sub, -);
    impl_arithmetic_op!(mul, *);
    impl_arithmetic_op!(div, /);
    impl_arithmetic_op!(modulus, %);

    /// Returns a description of the value as string.
    pub fn to_kind(&self) -> String {
        match self {
            Empty => "Empty".to_string(),
            Bool(_v) => "Bool".to_string(),
            Float(_v) => "Float".to_string(),
            FloatRange(_, _) => "Float".to_string(),
            Int(_i) => "Integer".to_string(),
            IntRange(_, _) => "Integer".to_string(),
            Text(_s) => "Text".to_string(),
            TextList(index, list) => list[*index as usize].clone(),
            Int2(v) => format!("Int2: {:?}", v),
            Float2(v) => format!("Float2: {:?}", v),
            Int3(v) => format!("Int3: {:?}", v),
            Float3(v) => format!("Float3: {:?}", v),
            Int4(v) => format!("Int4: {:?}", v),
            Float4(v) => format!("Float4: {:?}", v),
            Position(v) => format!("Position: {:?}", v),
            Tile(_v, _id) => "Tile".to_string(),
            Char(c) => c.to_string(),
            List(_) => "List".to_string(),
            KeyCode(k) => format!("KeyCode: {:?}", k),
            RangeI32(r) => format!("RangeI32: {:?}", r),
            RangeF32(r) => format!("RangeF32: {:?}", r),
            ColorObject(c) => format!("Color: {:?}", c),
            PaletteIndex(i) => format!("PaletteIndex: {:?}", i),
            Comparison(c) => format!("Comparison: {:?}", c.to_string()),
            Assignment(c) => format!("Assignment: {:?}", c.to_string()),
            Id(c) => format!("Id: {:?}", c.to_string()),
            Direction(v) => format!("Direction: {:?}", v),
            Time(t) => format!("Time: {:?}", t.to_time24()),
            TimeDuration(s, e) => format!("Time Duration: {:?} {:?}", s.to_time24(), e.to_time24()),
            TileMask(_) => str!("Pixels in a tile"),
            Image(b) => format!("Image ({}, {})", b.dim().width, b.dim().height),
        }
    }

    /// Returns a description of the value as string.
    pub fn describe(&self) -> String {
        match self {
            Empty => "Empty".to_string(),
            Bool(v) => {
                if *v {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Float(v) | FloatRange(v, _) => {
                if v.fract() == 0.0 {
                    format!("{:.1}", *v)
                } else {
                    v.to_string()
                }
            }
            Int(i) | IntRange(i, _) => i.to_string(),
            Text(s) => s.clone(),
            TextList(index, list) => list[*index as usize].clone(),
            Int2(v) => format!("({}, {})", v.x, v.y),
            Float2(v) => format!("({}, {})", v.x, v.y),
            Int3(v) => format!("({}, {}, {})", v.x, v.y, v.z),
            Float3(v) => format!("({}, {}, {})", v.x, v.y, v.z),
            Int4(v) => format!("({}, {}, {}, {})", v.x, v.y, v.z, v.w),
            Float4(v) => format!("({}, {}, {}, {})", v.x, v.y, v.z, v.w),
            Position(v) => format!("({}, {})", v.x, v.z),
            Tile(name, _id) => name.clone(),
            Char(c) => c.to_string(),
            List(list) => format!("List ({})", list.len()),
            KeyCode(k) => format!("KeyCode: {:?}", k),
            RangeI32(r) => format!("RangeI32: {:?}", r),
            RangeF32(r) => format!("RangeF32: {:?}", r),
            ColorObject(_) => "Color".to_string(),
            PaletteIndex(i) => format!("PaletteIndex: {:?}", i),
            Comparison(c) => format!("{:?}", c.to_string()),
            Assignment(c) => format!("{:?}", c.to_string()),
            Id(c) => format!("Id: {:?}", c.to_string()),
            Direction(d) => format!("D ({:.2}, {:.2})", d.x, d.z),
            Time(t) => t.to_time24(),
            TimeDuration(s, e) => format!("{} - {}", s.to_time24(), e.to_time24()),
            TileMask(_) => str!("Pixels"),
            Image(b) => format!("Image ({}, {})", b.dim().width, b.dim().height),
        }
    }
}

/// The methods how to compare two values.
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum TheValueComparison {
    Equal,
    Unequal,
    GreaterThanOrEqual,
    LessThanOrEqual,
    GreaterThan,
    LessThan,
}

impl TheValueComparison {
    pub fn to_string(self) -> &'static str {
        match self {
            TheValueComparison::Equal => "==",
            TheValueComparison::Unequal => "!=",
            TheValueComparison::GreaterThanOrEqual => ">=",
            TheValueComparison::LessThanOrEqual => "<=",
            TheValueComparison::GreaterThan => ">",
            TheValueComparison::LessThan => "<",
        }
    }
    pub fn iterator() -> impl Iterator<Item = TheValueComparison> {
        [
            TheValueComparison::Equal,
            TheValueComparison::Unequal,
            TheValueComparison::GreaterThanOrEqual,
            TheValueComparison::LessThanOrEqual,
            TheValueComparison::GreaterThan,
            TheValueComparison::LessThan,
        ]
        .iter()
        .copied()
    }
    pub fn from_index(index: u8) -> Option<TheValueComparison> {
        match index {
            0 => Some(TheValueComparison::Equal),
            1 => Some(TheValueComparison::Unequal),
            2 => Some(TheValueComparison::GreaterThanOrEqual),
            3 => Some(TheValueComparison::LessThanOrEqual),
            4 => Some(TheValueComparison::GreaterThan),
            5 => Some(TheValueComparison::LessThan),
            _ => None,
        }
    }
}

/// The methods of assigning values.
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum TheValueAssignment {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
}

impl TheValueAssignment {
    pub fn to_string(self) -> &'static str {
        match self {
            TheValueAssignment::Assign => "=",
            TheValueAssignment::AddAssign => "+=",
            TheValueAssignment::SubtractAssign => "-=",
            TheValueAssignment::MultiplyAssign => "*=",
            TheValueAssignment::DivideAssign => "/=",
            TheValueAssignment::ModulusAssign => "%=",
        }
    }

    pub fn iterator() -> impl Iterator<Item = TheValueAssignment> {
        [
            TheValueAssignment::Assign,
            TheValueAssignment::AddAssign,
            TheValueAssignment::SubtractAssign,
            TheValueAssignment::MultiplyAssign,
            TheValueAssignment::DivideAssign,
            TheValueAssignment::ModulusAssign,
        ]
        .iter()
        .copied()
    }

    pub fn from_index(index: u8) -> Option<TheValueAssignment> {
        match index {
            0 => Some(TheValueAssignment::Assign),
            1 => Some(TheValueAssignment::AddAssign),
            2 => Some(TheValueAssignment::SubtractAssign),
            3 => Some(TheValueAssignment::MultiplyAssign),
            4 => Some(TheValueAssignment::DivideAssign),
            5 => Some(TheValueAssignment::ModulusAssign),
            _ => None,
        }
    }
}
