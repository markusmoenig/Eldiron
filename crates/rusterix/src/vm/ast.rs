use super::objectd::FunctionD;
use super::{ASTValue, CompileVisitor, Context, Module, RuntimeError};
use std::path::PathBuf;
use vek::Vec3;

#[macro_export]
macro_rules! empty_expr {
    () => {
        Box::new(Expr::Value(
            ASTValue::None,
            vec![],
            vec![],
            Location::default(),
        ))
    };
}

#[macro_export]
macro_rules! zero_expr_int {
    () => {
        Box::new(Expr::Value(
            ASTValue::Int(0),
            vec![],
            vec![],
            Location::default(),
        ))
    };
}

#[macro_export]
macro_rules! zero_expr_float {
    () => {
        Box::new(Expr::Value(
            ASTValue::Float(0.0),
            vec![],
            vec![],
            Location::default(),
        ))
    };
}

#[macro_export]
macro_rules! expr_float {
    ($val:expr) => {
        Box::new(Expr::Value(
            ASTValue::Float($val),
            vec![],
            vec![],
            Location::default(),
        ))
    };
}

/// Statements in the AST
#[derive(Clone, Debug)]
pub enum Stmt {
    If(Box<Expr>, Box<Stmt>, Option<Box<Stmt>>, Location),
    While(Box<Expr>, Box<Stmt>, Location),
    For(
        Vec<Box<Stmt>>,
        Vec<Box<Expr>>,
        Vec<Box<Expr>>,
        Box<Stmt>,
        Location,
    ),
    Import(Option<Module>, Location),
    FunctionDeclaration(FunctionD, Location),
    Print(Box<Expr>, Location),
    Block(Vec<Box<Stmt>>, Location),
    Expression(Box<Expr>, Location),
    VarDeclaration(String, ASTValue, Box<Expr>, Location),
    StructDeclaration(String, Vec<(String, ASTValue)>, Location),
    Return(Box<Expr>, Location),
    Break(Location),
    Empty,
}

/// Expressions in the AST
#[derive(Clone, Debug)]
pub enum Expr {
    Value(ASTValue, Vec<u8>, Vec<String>, Location),
    Logical(Box<Expr>, LogicalOperator, Box<Expr>, Location),
    Unary(UnaryOperator, Box<Expr>, Location),
    Equality(Box<Expr>, EqualityOperator, Box<Expr>, Location),
    Comparison(Box<Expr>, ComparisonOperator, Box<Expr>, Location),
    Binary(Box<Expr>, BinaryOperator, Box<Expr>, Location),
    Grouping(Box<Expr>, Location),
    Variable(String, Vec<u8>, Vec<String>, Location),
    VariableAssignment(
        String,
        AssignmentOperator,
        Vec<u8>,
        Vec<String>,
        Box<Expr>,
        Location,
    ),
    FunctionCall(Box<Expr>, Vec<u8>, Vec<String>, Vec<Box<Expr>>, Location),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>, Location),
}

/// Assignment operators in the AST
#[derive(Clone, PartialEq, Debug)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
}

impl AssignmentOperator {
    pub fn describe(&self) -> &str {
        match self {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::AddAssign => "+=",
            AssignmentOperator::SubtractAssign => "-=",
            AssignmentOperator::MultiplyAssign => "*=",
            AssignmentOperator::DivideAssign => "/=",
        }
    }
}

/// Logical operators in the AST
#[derive(Clone, PartialEq, Debug)]
pub enum LogicalOperator {
    And,
    Or,
}

impl LogicalOperator {
    pub fn describe(&self) -> &str {
        match self {
            LogicalOperator::And => "&&",
            LogicalOperator::Or => "||",
        }
    }
}

/// Unary operators in the AST
#[derive(Clone, Debug)]
pub enum UnaryOperator {
    Negate,
    Minus,
}

/// Binary operators in the AST
#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Mod,
}

impl BinaryOperator {
    pub fn describe(&self) -> &str {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Mod => "%",
        }
    }
}

/// Comparison operators in the AST
#[derive(Clone, Debug)]
pub enum ComparisonOperator {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

impl ComparisonOperator {
    pub fn describe(&self) -> &str {
        match self {
            ComparisonOperator::Greater => ">",
            ComparisonOperator::GreaterEqual => ">=",
            ComparisonOperator::Less => "<",
            ComparisonOperator::LessEqual => "<=",
        }
    }
}

/// Equality operators in the AST
#[derive(Clone, Debug)]
pub enum EqualityOperator {
    NotEqual,
    Equal,
}

impl EqualityOperator {
    pub fn describe(&self) -> &str {
        match self {
            EqualityOperator::NotEqual => "!=",
            EqualityOperator::Equal => "==",
        }
    }
}

/// Visitor trait
pub trait Visitor {
    fn new() -> Self
    where
        Self: Sized;

    fn print(
        &mut self,
        expression: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn block(
        &mut self,
        list: &[Box<Stmt>],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn expression(
        &mut self,
        expression: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn import(
        &mut self,
        id: &Option<Module>,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn function_declaration(
        &mut self,
        objectd: &FunctionD,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn var_declaration(
        &mut self,
        name: &str,
        static_type: &ASTValue,
        expression: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn value(
        &mut self,
        value: ASTValue,
        swizzle: &[u8],
        field_path: &[String],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn unary(
        &mut self,
        op: &UnaryOperator,
        expr: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn equality(
        &mut self,
        left: &Expr,
        op: &EqualityOperator,
        right: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn comparison(
        &mut self,
        left: &Expr,
        op: &ComparisonOperator,
        right: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn binary(
        &mut self,
        left: &Expr,
        op: &BinaryOperator,
        right: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn grouping(
        &mut self,
        expression: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn variable(
        &mut self,
        name: String,
        swizzle: &[u8],
        field_path: &[String],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    #[allow(clippy::too_many_arguments)]
    fn variable_assignment(
        &mut self,
        name: String,
        op: &AssignmentOperator,
        swizzle: &[u8],
        field_path: &[String],
        expression: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn func_call(
        &mut self,
        callee: &Expr,
        swizzle: &[u8],
        field_path: &[String],
        args: &[Box<Expr>],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn struct_declaration(
        &mut self,
        name: &str,
        field: &[(String, ASTValue)],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn return_stmt(
        &mut self,
        expr: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn break_stmt(&mut self, loc: &Location, ctx: &mut Context) -> Result<ASTValue, RuntimeError>;
    fn empty_stmt(&mut self, ctx: &mut Context) -> Result<ASTValue, RuntimeError>;

    fn if_stmt(
        &mut self,
        cond: &Expr,
        then_stmt: &Stmt,
        else_stmt: &Option<Box<Stmt>>,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn while_stmt(
        &mut self,
        cond: &Expr,
        body_stmt: &Stmt,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn for_stmt(
        &mut self,
        init: &[Box<Stmt>],
        cond: &[Box<Expr>],
        incr: &[Box<Expr>],
        body_stmt: &Stmt,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn logical_expr(
        &mut self,
        left: &Expr,
        op: &LogicalOperator,
        right: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;

    fn ternary(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError>;
}

impl Stmt {
    pub fn accept(
        &self,
        visitor: &mut dyn Visitor,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        match self {
            Stmt::If(cond, then_stmt, else_stmt, loc) => {
                visitor.if_stmt(cond, then_stmt, else_stmt, loc, ctx)
            }
            Stmt::While(cond, body, loc) => visitor.while_stmt(cond, body, loc, ctx),
            Stmt::For(init, cond, incr, body, loc) => {
                visitor.for_stmt(init, cond, incr, body, loc, ctx)
            }
            Stmt::Print(expression, loc) => visitor.print(expression, loc, ctx),
            Stmt::Block(list, loc) => visitor.block(list, loc, ctx),
            Stmt::Expression(expression, loc) => visitor.expression(expression, loc, ctx),
            Stmt::Import(module, loc) => visitor.import(module, loc, ctx),
            Stmt::FunctionDeclaration(objectd, loc) => {
                visitor.function_declaration(objectd, loc, ctx)
            }
            Stmt::VarDeclaration(name, static_type, initializer, loc) => {
                visitor.var_declaration(name, static_type, initializer, loc, ctx)
            }
            Stmt::StructDeclaration(name, fields, loc) => {
                visitor.struct_declaration(name, fields, loc, ctx)
            }
            Stmt::Break(loc) => visitor.break_stmt(loc, ctx),
            Stmt::Empty => visitor.empty_stmt(ctx),
            Stmt::Return(expr, loc) => visitor.return_stmt(expr, loc, ctx),
        }
    }
}

impl Expr {
    pub fn accept(
        &self,
        visitor: &mut dyn Visitor,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        match self {
            Expr::Value(value, swizzle, field_path, loc) => {
                visitor.value(value.clone(), swizzle, field_path, loc, ctx)
            }
            Expr::Logical(left, op, right, loc) => visitor.logical_expr(left, op, right, loc, ctx),
            Expr::Unary(op, expr, loc) => visitor.unary(op, expr, loc, ctx),
            Expr::Equality(left, op, right, loc) => visitor.equality(left, op, right, loc, ctx),
            Expr::Comparison(left, op, right, loc) => visitor.comparison(left, op, right, loc, ctx),
            Expr::Binary(left, op, right, loc) => visitor.binary(left, op, right, loc, ctx),
            Expr::Grouping(expr, loc) => visitor.grouping(expr, loc, ctx),
            Expr::Variable(name, swizzle, field_path, loc) => {
                visitor.variable(name.clone(), swizzle, field_path, loc, ctx)
            }
            Expr::VariableAssignment(name, op, swizzle, field_path, expr, loc) => {
                visitor.variable_assignment(name.clone(), op, swizzle, field_path, expr, loc, ctx)
            }
            Expr::FunctionCall(callee, args, swizzle, field_path, loc) => {
                visitor.func_call(callee, args, swizzle, field_path, loc, ctx)
            }
            Expr::Ternary(cond, then_expr, else_expr, loc) => {
                visitor.ternary(cond, then_expr, else_expr, loc, ctx)
            }
        }
    }

    /// Converts a Float3 expression to a Vec3<f32>
    pub fn to_vec3(&self, visitor: &mut CompileVisitor, ctx: &mut Context) -> Option<Vec3<f32>> {
        if let Expr::Value(ASTValue::Float3(x, y, z), _, _, _) = self {
            let x_val = x.accept(visitor, ctx).ok()?;
            let y_val = y.accept(visitor, ctx).ok()?;
            let z_val = z.accept(visitor, ctx).ok()?;

            if let (ASTValue::Float(x_f), ASTValue::Float(y_f), ASTValue::Float(z_f)) =
                (x_val, y_val, z_val)
            {
                return Some(Vec3::new(x_f, y_f, z_f));
            }
        }
        None
    }
}

/// Location in the source code
#[derive(Clone, Debug)]
pub struct Location {
    pub line: usize,
    pub path: PathBuf,
}

impl Default for Location {
    fn default() -> Self {
        Self::new(0, PathBuf::default())
    }
}

impl Location {
    pub fn new(line: usize, path: PathBuf) -> Self {
        Location { line, path }
    }

    pub fn describe(&self) -> String {
        // format!("in '{}' at line {}.", self.file, self.line)
        format!("at line {}.", self.line)
    }
}
