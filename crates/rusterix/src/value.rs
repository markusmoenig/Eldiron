use crate::{
    Light, MaterialProfile, ParticleEmitter, Pixel, PixelSource, PlayerCamera, SampleMode, Texture,
    VertexBlendPreset,
};
// use rustpython::vm::*;
use std::fmt;
use theframework::prelude::*;

/// A single height control point with position and height
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct HeightControlPoint {
    pub position: [f32; 2], // UV position (x, y)
    pub height: f32,        // Height value
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Value {
    NoValue,
    Bool(bool),
    Int(i32),
    UInt(u32),
    Int64(i64),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Str(String),
    StrArray(Vec<String>),
    Id(Uuid),
    Source(PixelSource),
    Texture(Texture),
    SampleMode(SampleMode),
    PlayerCamera(PlayerCamera),
    Light(Light),
    Pixel(Pixel),
    Color(TheColor),
    ParticleEmitter(ParticleEmitter),
    MaterialProfile(MaterialProfile),
    HeightPoints(Vec<HeightControlPoint>),
    #[serde(with = "vectorize")]
    TileOverrides(FxHashMap<(i32, i32), PixelSource>),
    #[serde(with = "vectorize")]
    BlendOverrides(FxHashMap<(i32, i32), (VertexBlendPreset, PixelSource)>),
}

impl Value {
    pub fn to_source(&self) -> Option<&PixelSource> {
        match self {
            Value::Source(source) => Some(source),
            _ => None,
        }
    }

    pub fn to_f32(&self) -> Option<f32> {
        match self {
            Value::Int(f) => Some(*f as f32),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn to_i32(&self) -> Option<i32> {
        match self {
            Value::Int(f) => Some(*f),
            Value::Float(f) => Some(*f as i32),
            _ => None,
        }
    }

    /*
    /// Convert from a Python object
    pub fn from_pyobject(value: PyObjectRef, vm: &VirtualMachine) -> Option<Self> {
        if value.class().is(vm.ctx.types.bool_type) {
            let val: bool = value.try_into_value(vm).ok()?;
            Some(Value::Bool(val))
        } else if value.class().is(vm.ctx.types.int_type) {
            // let val: i32 = value.try_into_value(vm).ok()?;
            // Some(Value::Int(val))
            // Try signed first
            if let Ok(val) = i32::try_from_object(vm, value.clone()) {
                Some(Value::Int(val))
            }
            // Then try unsigned if it doesn't fit as i32
            else if let Ok(val) = u32::try_from_object(vm, value) {
                Some(Value::UInt(val))
            } else {
                None // Doesn't fit in either
            }
        } else if value.class().is(vm.ctx.types.float_type) {
            let val: f32 = value.try_into_value(vm).ok()?;
            Some(Value::Float(val))
        } else if value.class().is(vm.ctx.types.str_type) {
            let val: String = value.try_into_value(vm).ok()?;
            Some(Value::Str(val))
        } else if value.class().is(vm.ctx.types.tuple_type) {
            let tuple: Vec<PyObjectRef> = value.try_into_value(vm).ok()?;
            match tuple.len() {
                2 => {
                    let x: f32 = tuple[0].clone().try_into_value(vm).ok()?;
                    let y: f32 = tuple[1].clone().try_into_value(vm).ok()?;
                    Some(Value::Vec2([x, y]))
                }
                3 => {
                    let x: f32 = tuple[0].clone().try_into_value(vm).ok()?;
                    let y: f32 = tuple[1].clone().try_into_value(vm).ok()?;
                    let z: f32 = tuple[2].clone().try_into_value(vm).ok()?;
                    Some(Value::Vec3([x, y, z]))
                }
                4 => {
                    let x: f32 = tuple[0].clone().try_into_value(vm).ok()?;
                    let y: f32 = tuple[1].clone().try_into_value(vm).ok()?;
                    let z: f32 = tuple[2].clone().try_into_value(vm).ok()?;
                    let w: f32 = tuple[3].clone().try_into_value(vm).ok()?;
                    Some(Value::Vec4([x, y, z, w]))
                }
                _ => None,
            }
        } else {
            None
        }
    }*/

    /*
    /// Convert to a Python object
    pub fn to_pyobject(&self, vm: &VirtualMachine) -> PyObjectRef {
        match self {
            Value::Bool(val) => vm.ctx.new_bool(*val).into(),
            Value::Int(val) => vm.ctx.new_int(*val).into(),
            Value::Float(val) => vm.ctx.new_float(*val as f64).into(),
            Value::Str(val) => vm.ctx.new_str(val.clone()).into(),
            Value::Vec2(val) => vm
                .ctx
                .new_tuple(vec![
                    vm.ctx.new_float(val[0] as f64).into(),
                    vm.ctx.new_float(val[1] as f64).into(),
                ])
                .into(),
            Value::Vec3(val) => vm
                .ctx
                .new_tuple(vec![
                    vm.ctx.new_float(val[0] as f64).into(),
                    vm.ctx.new_float(val[1] as f64).into(),
                    vm.ctx.new_float(val[2] as f64).into(),
                ])
                .into(),
            Value::Vec4(val) => vm
                .ctx
                .new_tuple(vec![
                    vm.ctx.new_float(val[0] as f64).into(),
                    vm.ctx.new_float(val[1] as f64).into(),
                    vm.ctx.new_float(val[2] as f64).into(),
                    vm.ctx.new_float(val[3] as f64).into(),
                ])
                .into(),
            Value::Id(uuid) => vm.ctx.new_str(uuid.to_string()).into(), // Convert UUID to string
            _ => vm.ctx.none(),
        }
    }*/
}

// Implement Display for Python-compatible string representation
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::NoValue => write!(f, "NoValue"),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Int(val) => write!(f, "{}", val),
            Value::UInt(val) => write!(f, "{}", val),
            Value::Int64(val) => write!(f, "{}", val),
            Value::Float(val) => write!(f, "{:.2}", val),
            Value::Vec2(val) => write!(f, "[{}, {}]", val[0], val[1]),
            Value::Vec3(val) => write!(f, "[{}, {}, {}]", val[0], val[1], val[2]),
            Value::Vec4(val) => write!(f, "[{}, {}, {}, {}]", val[0], val[1], val[2], val[3]),
            Value::Str(val) => write!(f, "{}", val),
            Value::StrArray(val) => write!(f, "{:?}", val),
            Value::Id(val) => write!(f, "{}", val),
            Value::Source(val) => write!(f, "{:?}", val),
            Value::Texture(val) => {
                write!(f, "Texture: {}, {}", val.width, val.height)
            }
            Value::SampleMode(_) => write!(f, "SampleMode"),
            Value::PlayerCamera(_) => write!(f, "PlayerCamera"),
            Value::Light(_) => write!(f, "Light"),
            Value::Pixel(_) => write!(f, "Pixel"),
            Value::Color(_) => write!(f, "Color"),
            Value::ParticleEmitter(_) => write!(f, "ParticleEmitter"),
            Value::MaterialProfile(_) => write!(f, "MaterialProfile"),
            Value::HeightPoints(points) => write!(f, "HeightPoints({})", points.len()),
            Value::TileOverrides(_) => write!(f, "TileOverrides"),
            Value::BlendOverrides(_) => write!(f, "BlendOverrides"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ValueContainer {
    values: FxHashMap<String, Value>,
}

impl Default for ValueContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueContainer {
    // Create a new, empty ValueContainer
    pub fn new() -> Self {
        ValueContainer {
            values: FxHashMap::default(),
        }
    }

    // Add or update a value
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    // Get a value by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    // Toggle a boolean value
    pub fn toggle(&mut self, key: &str) {
        if let Some(Value::Bool(current)) = self.values.get_mut(key) {
            *current = !*current;
        }
    }

    // Get a mutable reference to a value by key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.values.get_mut(key)
    }

    // Getters for specific value types by key
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| {
            if let Value::Bool(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_bool_default(&self, key: &str, def: bool) -> bool {
        self.values
            .get(key)
            .map(|v| if let Value::Bool(val) = v { *val } else { def })
            .unwrap_or(def)
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.values.get(key).and_then(|v| {
            if let Value::Int(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_int_default(&self, key: &str, def: i32) -> i32 {
        self.values
            .get(key)
            .map(|v| if let Value::Int(val) = v { *val } else { def })
            .unwrap_or(def)
    }

    pub fn get_float(&self, key: &str) -> Option<f32> {
        self.values.get(key).and_then(|v| {
            if let Value::Float(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_float_default(&self, key: &str, def: f32) -> f32 {
        self.values
            .get(key)
            .map(|v| if let Value::Float(val) = v { *val } else { def })
            .unwrap_or(def)
    }

    pub fn get_vec2(&self, key: &str) -> Option<[f32; 2]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec2(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_vec3(&self, key: &str) -> Option<[f32; 3]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec3(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_vec3_default(&self, key: &str, def: [f32; 3]) -> [f32; 3] {
        self.get_vec3(key).unwrap_or(def)
    }

    pub fn get_vec4(&self, key: &str) -> Option<[f32; 4]> {
        self.values.get(key).and_then(|v| {
            if let Value::Vec4(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(|v| {
            if let Value::Str(val) = v {
                Some(val.as_str())
            } else {
                None
            }
        })
    }

    pub fn get_str_default(&self, key: &str, def: String) -> String {
        self.values
            .get(key)
            .map(|v| {
                if let Value::Str(val) = v {
                    val.clone()
                } else {
                    def.clone()
                }
            })
            .unwrap_or(def)
    }

    pub fn get_color_default(&self, key: &str, def: TheColor) -> TheColor {
        self.values
            .get(key)
            .map(|v| {
                if let Value::Color(val) = v {
                    val.clone()
                } else {
                    def.clone()
                }
            })
            .unwrap_or(def)
    }

    pub fn get_id(&self, key: &str) -> Option<Uuid> {
        self.values.get(key).and_then(|v| {
            if let Value::Id(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    /// Get a source of the given key.
    pub fn get_source(&self, key: &str) -> Option<&PixelSource> {
        self.values.get(key).and_then(|v| {
            if let Value::Source(val) = v {
                Some(val)
            } else {
                None
            }
        })
    }

    /// Get the default source. "floor_source" is just for compatibility.
    pub fn get_default_source(&self) -> Option<&PixelSource> {
        self.values.get("source").and_then(|v| {
            if let Value::Source(val) = v {
                Some(val)
            } else {
                None
            }
        })
    }

    /// Get a material profile.
    pub fn get_material_profile(&self, key: &str) -> Option<MaterialProfile> {
        self.values.get(key).and_then(|v| {
            if let Value::MaterialProfile(val) = v {
                Some(*val)
            } else {
                None
            }
        })
    }

    /// Get height control points for terrain generation.
    pub fn get_height_points(&self, key: &str) -> Option<&Vec<HeightControlPoint>> {
        self.values.get(key).and_then(|v| {
            if let Value::HeightPoints(val) = v {
                Some(val)
            } else {
                None
            }
        })
    }

    /// Get height control points with a default empty vector.
    pub fn get_height_points_default(&self, key: &str) -> Vec<HeightControlPoint> {
        self.values
            .get(key)
            .map(|v| {
                if let Value::HeightPoints(val) = v {
                    val.clone()
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default()
    }

    // Checks if the value exists
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    // Remove a value by key
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.values.remove(key)
    }

    // Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    // Get all keys sorted
    pub fn keys_sorted(&self) -> Vec<&String> {
        let mut keys: Vec<&String> = self.values.keys().collect();

        keys.sort_by(|a, b| {
            let type_a = self.get_type_order(a);
            let type_b = self.get_type_order(b);

            // First, sort by Value type order
            match type_a.cmp(&type_b) {
                std::cmp::Ordering::Equal => a.cmp(b), // If same type, sort by key name
                other => other,
            }
        });

        keys
    }

    /// Helper function to assign sorting order for Value types
    fn get_type_order(&self, key: &String) -> usize {
        match self.values.get(key) {
            Some(Value::Str(_)) => 0, // Strings come first
            Some(Value::StrArray(_)) => 0,
            Some(Value::Bool(_)) => 1,
            Some(Value::Int(_)) => 2,
            Some(Value::UInt(_)) => 2,
            Some(Value::Int64(_)) => 2,
            Some(Value::Float(_)) => 3,
            Some(Value::Vec2(_)) => 4,
            Some(Value::Vec3(_)) => 5,
            Some(Value::Vec4(_)) => 6,
            Some(Value::Id(_)) => 7,
            Some(Value::Source(_)) => 8,
            Some(Value::Texture(_)) => 9,
            Some(Value::SampleMode(_)) => 10,
            Some(Value::PlayerCamera(_)) => 11,
            Some(Value::Light(_)) => 12,
            Some(Value::Pixel(_)) => 13,
            Some(Value::Color(_)) => 14,
            Some(Value::ParticleEmitter(_)) => 14,
            Some(Value::MaterialProfile(_)) => 15,
            Some(Value::HeightPoints(_)) => 16,
            Some(Value::TileOverrides(_)) => 17,
            Some(Value::BlendOverrides(_)) => 17,
            Some(Value::NoValue) => 18,
            None => 99, // If key is missing, push to the end
        }
    }

    // Get all values
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.values.values()
    }

    /// Convert the container into a Python dict string representation
    pub fn to_python_dict_string(&self) -> String {
        let mut items = Vec::new();

        for (key, value) in &self.values {
            let key_str = format!("{:?}", key);
            let value_str = match value {
                Value::NoValue => "None".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Int(i) => i.to_string(),
                Value::UInt(i) => i.to_string(),
                Value::Int64(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Vec2(v) => format!("[{}, {}]", v[0], v[1]),
                Value::Vec3(v) => format!("[{}, {}, {}]", v[0], v[1], v[2]),
                Value::Vec4(v) => format!("[{}, {}, {}, {}]", v[0], v[1], v[2], v[3]),
                Value::Str(s) => format!("{:?}", s),
                Value::Id(u) => format!("{:?}", u.to_string()),
                _ => continue, // Skip unsupported types
            };

            // Always end with a comma
            items.push(format!("    {}: {},", key_str, value_str));
        }

        format!("{{\n{}\n}}", items.join("\n"))
    }
}

// Implement Display for ValueContainer
impl fmt::Display for ValueContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.values {
            writeln!(f, "{}: {}", key, value)?;
        }
        Ok(())
    }
}
