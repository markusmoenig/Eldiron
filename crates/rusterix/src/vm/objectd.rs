use super::{Expr, NodeOp, Stmt};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct FunctionD {
    pub name: String,
    pub arity: usize,
    pub locals: IndexMap<String, Option<Box<Expr>>>,
    pub block: Box<Stmt>,
    pub body: Vec<NodeOp>,
}

impl FunctionD {
    pub fn new(
        name: String,
        arity: usize,
        locals: IndexMap<String, Option<Box<Expr>>>,
        block: Box<Stmt>,
    ) -> Self {
        Self {
            name,
            arity,
            locals,
            block: block,

            body: vec![],
        }
    }
}
