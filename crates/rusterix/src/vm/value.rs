use crate::value::Value;
use std::ops::{Add, Div, Mul, Neg, Sub};
use vek::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct VMValue {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub string: Option<String>,
}

impl VMValue {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            string: None,
        }
    }

    /// Construct with numeric components and an optional string payload.
    pub fn new_with_string<S: Into<String>>(x: f32, y: f32, z: f32, s: S) -> Self {
        Self {
            x,
            y,
            z,
            string: Some(s.into()),
        }
    }

    pub fn broadcast(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            string: None,
        }
    }

    pub fn zero() -> Self {
        Self::broadcast(0.0)
    }

    pub fn from_bool(v: bool) -> Self {
        Self::broadcast(if v { 1.0 } else { 0.0 })
    }

    pub fn from_i32(v: i32) -> Self {
        Self::broadcast(v as f32)
    }

    pub fn from_f32(v: f32) -> Self {
        Self::broadcast(v)
    }

    pub fn from_u32(v: u32) -> Self {
        Self::broadcast(v as f32)
    }

    /// Generic helper leveraging `Into<VMValue>` implementations.
    pub fn from<T: Into<VMValue>>(v: T) -> Self {
        v.into()
    }

    pub fn from_vec3(v: Vec3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            string: None,
        }
    }

    pub fn to_vec3(&self) -> Vec3<f32> {
        Vec3::new(self.x, self.y, self.z)
    }

    pub fn from_string<S: Into<String>>(s: S) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            string: Some(s.into()),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        self.string.as_deref()
    }

    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::NoValue => VMValue::zero(),
            Value::Bool(b) => VMValue::broadcast(if *b { 1.0 } else { 0.0 }),
            Value::Int(i) => VMValue::broadcast(*i as f32),
            Value::UInt(i) => VMValue::broadcast(*i as f32),
            Value::Int64(i) => VMValue::broadcast(*i as f32),
            Value::Float(f) => VMValue::broadcast(*f),
            Value::Vec2(v) => VMValue::new(v[0], v[1], 0.0),
            Value::Vec3(v) => VMValue::new(v[0], v[1], v[2]),
            Value::Vec4(v) => VMValue::new(v[0], v[1], v[2]),
            Value::Str(s) => VMValue::from_string(s.clone()),
            _ => VMValue::zero(),
        }
    }

    /// Convert into a generic runtime Value.
    pub fn to_value(&self) -> Value {
        if let Some(s) = self.as_string() {
            Value::Str(s.to_string())
        } else if self.x == self.y && self.x == self.z {
            Value::Float(self.x)
        } else {
            Value::Vec3([self.x, self.y, self.z])
        }
    }

    /// Convert into a Value using an optional type hint and/or inline string tag (e.g. "bool").
    pub fn to_value_with_hint(&self, hint: Option<&Value>) -> Value {
        // String payload can act as an explicit type hint.
        if let Some(s) = self.as_string() {
            let s_trim = s.trim();
            // Support legacy tagged strings like "bool:true"
            if let Some(tagged) = Self::from_type_tagged_str(s_trim) {
                return tagged;
            }
            match s_trim.to_ascii_lowercase().as_str() {
                "bool" => return Value::Bool(self.to_bool()),
                "int" => return Value::Int(self.x as i32),
                "uint" => return Value::UInt(self.x.max(0.0) as u32),
                "i64" | "int64" => return Value::Int64(self.x as i64),
                "float" | "f32" => return Value::Float(self.x),
                "vec2" => return Value::Vec2([self.x, self.y]),
                "vec3" => return Value::Vec3([self.x, self.y, self.z]),
                "str" | "string" => {
                    return Value::Str(Self::to_string_lossy_components(self.x, self.y, self.z));
                }
                _ => {}
            }
        }

        match hint {
            Some(Value::Bool(_)) => Value::Bool(self.to_bool()),
            Some(Value::Int(_)) => Value::Int(self.x as i32),
            Some(Value::UInt(_)) => Value::UInt(self.x.max(0.0) as u32),
            Some(Value::Int64(_)) => Value::Int64(self.x as i64),
            Some(Value::Float(_)) => Value::Float(self.x),
            Some(Value::Vec2(_)) => Value::Vec2([self.x, self.y]),
            Some(Value::Vec3(_)) => Value::Vec3([self.x, self.y, self.z]),
            Some(Value::Vec4(_)) => Value::Vec4([self.x, self.y, self.z, 0.0]),
            Some(Value::Str(_)) => Value::Str(
                self.as_string()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}", self.x)),
            ),
            Some(Value::StrArray(_)) => Value::StrArray(vec![
                self.as_string()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}", self.x)),
            ]),
            _ => {
                // Fallback: infer from string payload, then numbers.
                if let Some(s) = self.as_string() {
                    if let Some(b) = Self::parse_bool_str(s) {
                        return Value::Bool(b);
                    }
                    if let Ok(i) = s.parse::<i32>() {
                        return Value::Int(i);
                    }
                    if let Ok(f) = s.parse::<f32>() {
                        return Value::Float(f);
                    }
                    return Value::Str(s.to_string());
                }
                if self.x == self.y && self.x == self.z {
                    Value::Float(self.x)
                } else {
                    Value::Vec3([self.x, self.y, self.z])
                }
            }
        }
    }

    pub fn to_bool(&self) -> bool {
        if let Some(s) = self.as_string() {
            if let Some(b) = Self::parse_bool_str(s) {
                return b;
            }
        }
        // Numeric fallback: nonzero -> true
        self.x != 0.0 || self.y != 0.0 || self.z != 0.0
    }

    pub fn is_truthy(&self) -> bool {
        if let Some(s) = &self.string {
            !s.is_empty()
        } else {
            self.x != 0.0 || self.y != 0.0 || self.z != 0.0
        }
    }

    fn parse_bool_str(s: &str) -> Option<bool> {
        match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        }
    }

    fn from_type_tagged_str(s: &str) -> Option<Value> {
        let (tag, rest) = s.split_once(':')?;
        let tag = tag.trim().to_ascii_lowercase();
        let rest = rest.trim();

        match tag.as_str() {
            "bool" => Self::parse_bool_str(rest).map(Value::Bool),
            "int" => rest.parse::<i32>().ok().map(Value::Int),
            "uint" => rest.parse::<u32>().ok().map(Value::UInt),
            "i64" | "int64" => rest.parse::<i64>().ok().map(Value::Int64),
            "float" | "f32" => rest.parse::<f32>().ok().map(Value::Float),
            "vec2" => parse_vec(rest, 2).map(|v| Value::Vec2([v[0], v[1]])),
            "vec3" => parse_vec(rest, 3).map(|v| Value::Vec3([v[0], v[1], v[2]])),
            "str" | "string" => Some(Value::Str(rest.to_string())),
            _ => None,
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.to_vec3().magnitude()
    }

    pub fn map<F: Fn(f32) -> f32>(&self, f: F) -> Self {
        VMValue::new(f(self.x), f(self.y), f(self.z))
    }

    pub fn map2<F: Fn(f32, f32) -> f32>(&self, other: VMValue, f: F) -> Self {
        VMValue::new(f(self.x, other.x), f(self.y, other.y), f(self.z, other.z))
    }

    pub fn dot(&self, other: VMValue) -> f32 {
        self.to_vec3().dot(other.to_vec3())
    }

    pub fn cross(&self, other: VMValue) -> Self {
        VMValue::from_vec3(self.to_vec3().cross(other.to_vec3()))
    }

    fn format_scalar(v: f32) -> String {
        if v.fract() == 0.0 {
            format!("{:.0}", v)
        } else {
            v.to_string()
        }
    }

    fn to_string_lossy_components(x: f32, y: f32, z: f32) -> String {
        if x == y && y == z {
            Self::format_scalar(x)
        } else {
            format!("{},{},{}", x, y, z)
        }
    }

    fn _to_string_lossy(&self) -> String {
        Self::to_string_lossy_components(self.x, self.y, self.z)
    }
}

impl std::fmt::Display for VMValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.string {
            let tag = s.trim();
            let tag_l = tag.to_ascii_lowercase();

            if let Some(val) = Self::from_type_tagged_str(tag) {
                return write!(f, "{}", format_value_brief(&val));
            }

            return match tag_l.as_str() {
                "bool" => write!(f, "{}", self.to_bool()),
                "int" => write!(f, "{}", self.x as i32),
                "uint" => write!(f, "{}", self.x.max(0.0) as u32),
                "i64" | "int64" => write!(f, "{}", self.x as i64),
                "float" | "f32" => write!(f, "{}", self.x),
                "vec2" => write!(f, "[{}, {}]", self.x, self.y),
                "vec3" => write!(f, "[{}, {}, {}]", self.x, self.y, self.z),
                "vec4" => write!(f, "[{}, {}, {}, 0]", self.x, self.y, self.z),
                "str" | "string" => {
                    write!(
                        f,
                        "{}",
                        Self::to_string_lossy_components(self.x, self.y, self.z)
                    )
                }
                _ => write!(f, "{}", s),
            };
        }

        if self.x == self.y && self.x == self.z {
            write!(f, "{}", self.x)
        } else {
            write!(f, "[{}, {}, {}]", self.x, self.y, self.z)
        }
    }
}

fn format_value_brief(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Int64(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Vec2(v) => format!("[{}, {}]", v[0], v[1]),
        Value::Vec3(v) => format!("[{}, {}, {}]", v[0], v[1], v[2]),
        Value::Vec4(v) => format!("[{}, {}, {}, {}]", v[0], v[1], v[2], v[3]),
        Value::Str(s) => s.clone(),
        _ => format!("{:?}", v),
    }
}

impl Add for VMValue {
    type Output = VMValue;

    fn add(self, rhs: VMValue) -> Self::Output {
        let (ax, ay, az) = (self.x, self.y, self.z);
        let (bx, by, bz) = (rhs.x, rhs.y, rhs.z);
        match (self.string, rhs.string) {
            (Some(a), Some(b)) => VMValue::from_string(format!("{a}{b}")),
            (Some(a), None) => {
                let b_str = VMValue::to_string_lossy_components(bx, by, bz);
                VMValue::from_string(format!("{a}{b_str}"))
            }
            (None, Some(b)) => {
                let a_str = VMValue::to_string_lossy_components(ax, ay, az);
                VMValue::from_string(format!("{a_str}{b}"))
            }
            _ => VMValue::new(ax + bx, ay + by, az + bz),
        }
    }
}

impl Sub for VMValue {
    type Output = VMValue;

    fn sub(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul for VMValue {
    type Output = VMValue;

    fn mul(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Div for VMValue {
    type Output = VMValue;

    fn div(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }
}

impl Neg for VMValue {
    type Output = VMValue;

    fn neg(self) -> Self::Output {
        VMValue::new(-self.x, -self.y, -self.z)
    }
}

impl From<bool> for VMValue {
    fn from(v: bool) -> Self {
        VMValue::from_bool(v)
    }
}

impl From<i32> for VMValue {
    fn from(v: i32) -> Self {
        VMValue::from_i32(v)
    }
}

impl From<u32> for VMValue {
    fn from(v: u32) -> Self {
        VMValue::from_u32(v)
    }
}

impl From<f32> for VMValue {
    fn from(v: f32) -> Self {
        VMValue::from_f32(v)
    }
}

impl From<String> for VMValue {
    fn from(s: String) -> Self {
        VMValue::from_string(s)
    }
}

impl From<&str> for VMValue {
    fn from(s: &str) -> Self {
        VMValue::from_string(s)
    }
}

impl From<Value> for VMValue {
    fn from(v: Value) -> Self {
        VMValue::from_value(&v)
    }
}

impl From<Vec3<f32>> for VMValue {
    fn from(v: Vec3<f32>) -> Self {
        VMValue::from_vec3(v)
    }
}

fn parse_vec(s: &str, expected: usize) -> Option<Vec<f32>> {
    let vals: Vec<f32> = s
        .split(',')
        .filter_map(|p| p.trim().parse::<f32>().ok())
        .collect();
    if vals.len() == expected {
        Some(vals)
    } else {
        None
    }
}
