use rustc_hash::FxHashMap;

use crate::NodeOp;
use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct Program {
    /// Number of global variables
    pub globals: usize,

    /// The program body
    pub body: Vec<NodeOp>,

    /// Code of all user defined functions.
    pub user_functions: Vec<Arc<[NodeOp]>>,

    /// Map of user function names to their indices.
    pub user_functions_name_map: FxHashMap<String, usize>,

    /// Index of the shape function
    pub shade_index: Option<usize>,

    /// Amount of local variables in the shade function
    pub shade_locals: usize,

    /// Strings,
    pub strings: Vec<String>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            user_functions: vec![],
            user_functions_name_map: FxHashMap::default(),
            shade_index: None,
            globals: 0,
            shade_locals: 0,
            strings: vec![],
        }
    }

    /// Returns true if the shader changes opacity
    pub fn shader_supports_opacity(&self) -> bool {
        if let Some(index) = self.user_functions_name_map.get("shade") {
            if let Some(func) = self.user_functions.get(*index) {
                for n in func.deref() {
                    if matches!(n, NodeOp::SetOpacity) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
