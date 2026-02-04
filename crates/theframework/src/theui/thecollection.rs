use indexmap::IndexMap;

pub use crate::prelude::*;

/// Represents a collection of TheValues.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TheCollection {
    pub name: String,
    pub keys: indexmap::IndexMap<String, TheValue>,
}

impl Default for TheCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl TheCollection {
    pub fn named(name: String) -> Self {
        Self {
            name,
            keys: IndexMap::default(),
        }
    }
    pub fn new() -> Self {
        Self {
            name: str!("Unnamed"),
            keys: IndexMap::default(),
        }
    }

    /// Clears the keys.
    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// Returns the given key.
    pub fn get(&self, key: &str) -> Option<&TheValue> {
        self.keys.get(key)
    }

    /// Returns the given key, if not found return the default.
    pub fn get_default(&self, key: &str, default: TheValue) -> TheValue {
        if let Some(v) = self.keys.get(key) {
            v.clone()
        } else {
            default
        }
    }

    /// Get an f32 value, if not found return the default.
    pub fn get_f32_default(&self, key: &str, default: f32) -> f32 {
        if let Some(v) = self.keys.get(key) {
            if let Some(v) = v.to_f32() {
                return v;
            }
        }
        default
    }

    /// Get an Float3 value, if not found return the default.
    pub fn get_float3_default(&self, key: &str, default: Vec3<f32>) -> Vec3<f32> {
        if let Some(v) = self.keys.get(key) {
            if let Some(v) = v.to_vec3f() {
                return v;
            }
        }
        default
    }

    /// Get an i32 value, if not found return the default.
    pub fn get_i32_default(&self, key: &str, default: i32) -> i32 {
        if let Some(v) = self.keys.get(key) {
            if let Some(v) = v.to_i32() {
                return v;
            }
        }
        default
    }

    /// Get a bool value, if not found return the default.
    pub fn get_bool_default(&self, key: &str, default: bool) -> bool {
        if let Some(TheValue::Bool(v)) = self.keys.get(key) {
            return *v;
        }
        default
    }

    /// Sets the given key with the given value.
    pub fn set(&mut self, key: &str, value: TheValue) {
        self.keys.insert(key.to_string(), value);
    }

    /// Checks if the collection contains the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.keys.contains_key(key)
    }
}
