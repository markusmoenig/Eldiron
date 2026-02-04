use super::{NodeOp, Program};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

/// The context during script and voxel compilation.
#[derive(Clone)]
pub struct Context {
    /// Holds the global variable names and their indices into the global / flat array.
    pub globals: FxHashMap<String, u32>,

    /// Custom targets needed for recursive nesting.
    pub custom_targets: Vec<Vec<NodeOp>>,

    /// Holds the grid and the programs NodeOps.
    pub program: Program,

    /// All imported paths, collected so that we can watch them.
    pub imported_paths: Vec<PathBuf>,
}

impl Context {
    pub fn new(globals: FxHashMap<String, u32>) -> Self {
        Self {
            program: Program::new(),
            globals,
            imported_paths: vec![],
            custom_targets: vec![],
        }
    }

    pub fn add_custom_target(&mut self) {
        self.custom_targets.push(vec![]);
    }

    pub fn take_last_custom_target(&mut self) -> Option<Vec<NodeOp>> {
        self.custom_targets.pop()
    }

    pub fn emit(&mut self, op: NodeOp) {
        if let Some(custom) = self.custom_targets.last_mut() {
            custom.push(op.clone());
            return;
        }

        self.program.body.push(op);
    }
}
