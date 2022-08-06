use crate::prelude::*;

// Server instance of a behavior
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ScopeBuffer {

    pub floats                  : HashMap<String, f64>,
}

impl ScopeBuffer {
    pub fn new(scope: &rhai::Scope) -> Self {

        let mut floats = HashMap::new();

        let iter = scope.iter();

        for val in iter {
            //println!("Got: {:?}", val);
        }

        Self {
            floats,
        }
    }
}