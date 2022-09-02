use crate::prelude::*;

use rhai::Scope;

// Server instance of a behavior
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ScopeBuffer {

    pub floats                      : HashMap<String, f64>,
}

impl ScopeBuffer {
    pub fn new() -> Self {
        let floats = HashMap::new();

        Self {
            floats,
        }
    }

    pub fn read_from_scope(&mut self, scope: &rhai::Scope) {

        self.floats = HashMap::new();

        let iter = scope.iter();

        for val in iter {
            if let Some(f) = val.2.as_float().ok() {
                self.floats.insert(val.0.to_string(), f);
            }
        }
    }

    /// Write the contents of this buffer to the scope
    pub fn write_to_scope(&self, _scope: &mut Scope) {

    }
}