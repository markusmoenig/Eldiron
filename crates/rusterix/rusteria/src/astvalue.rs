use crate::Expr;

/// Values in the AST
#[derive(Clone, Debug)]
pub enum ASTValue {
    None,
    Boolean(bool),
    Float(f32),
    Float2(Box<Expr>, Box<Expr>),
    Float3(Box<Expr>, Box<Expr>, Box<Expr>),
    Float4(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
    String(String),
    Function(String, Vec<ASTValue>, Box<ASTValue>),
}

impl ASTValue {
    /// Returns the value as a float if it is one.
    pub fn to_float(&self) -> Option<f32> {
        match self {
            ASTValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The truthiness of the value.
    pub fn is_truthy(&self) -> bool {
        match self {
            ASTValue::Boolean(b) => *b,
            ASTValue::Float(i) => *i != 0.0,
            ASTValue::Float2(_, _) => true,
            ASTValue::Float3(_, _, _) => true,
            ASTValue::Float4(_, _, _, _) => true,
            ASTValue::String(s) => !s.is_empty(),
            _ => false,
        }
    }

    // The components of the value.
    pub fn components(&self) -> usize {
        match self {
            ASTValue::Float(_) => 1,
            ASTValue::Float2(_, _) => 2,
            ASTValue::Float3(_, _, _) => 3,
            ASTValue::Float4(_, _, _, _) => 4,
            _ => 0,
        }
    }
}
