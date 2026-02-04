use super::ASTValue;
use rustc_hash::FxHashMap;

pub enum ScopeTypes {
    Function,
    Block,
}

// Define a struct to represent the environment
pub struct Environment {
    scopes: Vec<FxHashMap<String, ASTValue>>,
    scopes_types: Vec<ScopeTypes>,
    scoped_returns: Vec<ASTValue>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            scopes: vec![FxHashMap::default()],
            scopes_types: vec![],
            scoped_returns: vec![],
        }
    }

    /// Define a variable in the current scope
    pub fn define(&mut self, name: String, value: ASTValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Assign a value to an existing variable in the current scope or any outer scope
    pub fn assign(&mut self, name: &str, value: ASTValue) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }

    /// Get the value of a variable in the current scope or any outer scope
    pub fn get(&self, name: &str) -> Option<ASTValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Begin a new scope.
    pub fn begin_scope(&mut self, returns: ASTValue, func: bool) {
        self.scopes.push(FxHashMap::default());
        if func {
            self.scopes_types.push(ScopeTypes::Function);
        } else {
            self.scopes_types.push(ScopeTypes::Block);
        }
        self.scoped_returns.push(returns);
    }

    /// End the current scope.
    pub fn end_scope(&mut self) {
        self.scopes.pop();
        self.scopes_types.pop();
        self.scoped_returns.pop();
    }

    /// Returns the return value of the current scope.
    pub fn get_return(&self) -> Option<ASTValue> {
        // Unwind until we find the last function return type
        for (i, scope) in self.scopes_types.iter().enumerate().rev() {
            if let ScopeTypes::Function = scope {
                return self.scoped_returns.get(i).cloned();
            }
        }
        None
    }

    /// Returns true if the current scope is the global scope.
    pub fn is_global_scope(&self) -> bool {
        self.scopes.len() == 1
    }
}
