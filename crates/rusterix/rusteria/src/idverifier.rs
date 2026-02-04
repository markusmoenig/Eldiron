use crate::ParseError;
use rustc_hash::FxHashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Var {
    name: String,
    original_name: String,

    is_function: bool,
    is_const: bool,
    is_global: bool,
}

/// Verifies variable and identifier names in scope and handles shadowing
pub struct IdVerifier {
    scopes: Vec<FxHashMap<String, Var>>,
    var_counter: i32,
}

impl Default for IdVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl IdVerifier {
    pub fn new() -> Self {
        let mut inbuilt = FxHashMap::default();

        let inbuilt_functions = vec![
            "rotate2d",
            "dot",
            "dot2",
            "dot3",
            "cross",
            "mix",
            "smoothstep",
            "length",
            "length2",
            "length3",
            "normalize",
            "sin",
            "sin1",
            "sin2",
            "cos",
            "cos1",
            "cos2",
            "sqrt",
            "ceil",
            "floor",
            "fract",
            "abs",
            "tan",
            "degrees",
            "radians",
            "min",
            "max",
            "pow",
            "rand",
            "clamp",
            "sign",
            "atan",
            "atan2",
            "mod",
            "step",
            "exp",
            "log",
            "print",
            "sample",
            "sample_normal",
            "alloc",
            "iterate",
            "save",
            "rotate2d",
            "palette",
            "round",
        ];

        for func in inbuilt_functions {
            inbuilt.insert(
                func.to_string(),
                Var {
                    name: func.to_string(),
                    original_name: func.to_string(),
                    is_function: true,
                    is_const: true,
                    is_global: true,
                },
            );
        }

        IdVerifier {
            scopes: vec![inbuilt],
            var_counter: 0,
        }
    }

    /// Defines a new variable name. If the name already exists in the current scope, a new name is created.
    pub fn define_var(
        &mut self,
        original_name: &str,
        is_function: bool,
    ) -> Result<String, ParseError> {
        let name = if !is_function {
            self.create_var_name(original_name)
        } else {
            // TODO check if function name already exists
            original_name.to_string()
        };
        let var = Var {
            name: name.clone(),
            original_name: original_name.to_string(),
            is_function,
            is_const: false,
            is_global: self.is_global_scope(),
        };

        // let scope_ = self.current_scope();
        if let Some(scope) = self.scopes.last_mut() {
            // println!("insert {} at {}", name, scope_);
            scope.insert(name.clone(), var);
        }

        Ok(name)
    }

    /// Checks if a variable with the original name exists, and if yes, returns the new name.
    pub fn get_var_name(&mut self, original_name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            for (name, var) in scope.iter() {
                if var.original_name == original_name {
                    return Some(name.clone());
                }
            }
        }
        None
    }

    /// Gets the original name of a variable.
    pub fn get_original_var_name(&mut self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var.original_name.clone());
            }
        }
        None
    }

    /// Creates a new var name based on the original name.
    pub fn create_var_name(&mut self, name: &str) -> String {
        // let mut new_name: String = name.to_string();
        // let mut i = 0;
        // while self.var_exists(&new_name) {
        //     i += 1;
        //     new_name = format!("{}_{}", name, i);
        // }
        // new_name

        let new_name = format!("{}_{}", name, self.var_counter);
        self.var_counter += 1;
        new_name
    }

    pub fn var_exists(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }

    /// Begin a new scope.
    pub fn begin_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }

    /// End the current scope.
    pub fn end_scope(&mut self) {
        self.scopes.pop();
    }

    /// Returns true if the current scope is the global scope.
    pub fn is_global_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    pub fn current_scope(&self) -> i32 {
        self.scopes.len() as i32
    }
}
