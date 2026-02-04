use rustc_hash::FxHashMap;

use crate::vm::NodeOp;
use std::sync::Arc;

#[derive(Clone)]
pub struct Program {
    /// Number of global variables
    pub globals: usize,

    /// The program body
    pub body: Vec<NodeOp>,

    /// Code of all user defined functions.
    pub user_functions: Vec<Arc<[NodeOp]>>,

    /// Locals count per user function (parameters + locals).
    pub user_functions_locals: Vec<usize>,

    /// Map of user function names to their indices.
    pub user_functions_name_map: FxHashMap<String, usize>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            user_functions: vec![],
            user_functions_name_map: FxHashMap::default(),
            globals: 0,
            user_functions_locals: vec![],
        }
    }
}
