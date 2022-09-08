use crate::prelude::*;

use rhai::Scope;

// Server instance of a behavior
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ScopeBuffer {

    pub values                      : FxHashMap<String, Value>,
}

impl ScopeBuffer {
    pub fn new() -> Self {
        let values = FxHashMap::default();

        Self {
            values,
        }
    }

    pub fn read_from_scope(&mut self, scope: &rhai::Scope) {

        self.values = FxHashMap::default();

        let iter = scope.iter();

        for val in iter {
            if let Some(value) = val.2.as_float().ok() {
                self.values.insert(val.0.to_string(), Value::Float(value));
            } else
            if let Some(value) = val.2.as_int().ok() {
                self.values.insert(val.0.to_string(), Value::Integer(value));
            } else
            if let Some(value) = val.2.into_string().ok() {
                self.values.insert(val.0.to_string(), Value::String(value));
            }
        }
    }

    /// Write the contents of this buffer to the scope
    pub fn write_to_scope(&self, _scope: &mut Scope) {

    }
}