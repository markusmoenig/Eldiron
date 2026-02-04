use super::objectd::FunctionD;
use super::{
    ASTValue, AssignmentOperator, BinaryOperator, ComparisonOperator, Context, Environment,
    EqualityOperator, Expr, Location, LogicalOperator, Module, NodeOp, RuntimeError, Stmt,
    UnaryOperator, VMValue, Visitor, optimize,
};
use crate::vm::builtin::Builtins;
use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ASTFunction {
    pub name: String,
    pub arguments: i32,
    pub op: NodeOp,
}

/// ExecuteVisitor
pub struct CompileVisitor {
    pub environment: Environment,
    functions: FxHashMap<String, ASTFunction>,

    user_functions: IndexMap<String, (usize, IndexMap<String, Option<Vec<NodeOp>>>, usize, usize)>,

    /// List of local variables which are in scope (inside functions)
    locals: IndexSet<String>,
}

impl CompileVisitor {
    /// Map friendly field aliases to component indices.
    fn component_alias(field: &str) -> Option<u8> {
        match field {
            "distance" => Some(1),
            "amount" => Some(1),
            "subject_id" => Some(0),
            "count" => Some(2),
            _ => None,
        }
    }

    #[inline]
    fn is_string_alias(field: &str) -> bool {
        matches!(field, "string" | "text")
    }
}

impl Visitor for CompileVisitor {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut functions: FxHashMap<String, ASTFunction> = FxHashMap::default();
        for (name, (arity, op)) in Builtins::default().entries() {
            functions.insert(
                name.clone(),
                ASTFunction {
                    name: name.clone(),
                    arguments: *arity as i32,
                    op: op.clone(),
                },
            );
        }

        Self {
            environment: Environment::default(),
            functions,
            user_functions: IndexMap::default(),
            locals: IndexSet::default(),
        }
    }

    fn print(
        &mut self,
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        print!("-- Print ");
        expression.accept(self, ctx)?;
        println!(" --");

        Ok(ASTValue::None)
    }

    fn block(
        &mut self,
        list: &[Box<Stmt>],
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let mut value = ASTValue::None;

        self.environment.begin_scope(ASTValue::None, false);
        for stmt in list {
            value = stmt.accept(self, ctx)?;
        }
        self.environment.end_scope();

        Ok(value)
    }

    fn expression(
        &mut self,
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        expression.accept(self, ctx)
    }

    fn import(
        &mut self,
        module: &Option<Module>,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        // Execute the statements in the imported module
        if let Some(module) = module {
            ctx.imported_paths.push(module.path.clone());
            let mut visitor = CompileVisitor::new();
            for statement in module.stmts.clone() {
                _ = statement.accept(&mut visitor, ctx);
            }
        }

        Ok(ASTValue::None)
    }

    fn var_declaration(
        &mut self,
        name: &str,
        _static_type: &ASTValue,
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = expression.accept(self, ctx)?;

        if self.locals.is_empty() {
            // Global scope
            if let Some(index) = ctx.globals.get(name) {
                ctx.emit(NodeOp::StoreGlobal(*index as usize));
            }
        } else {
            // Function scope: ensure the local exists, then store.
            let index = if let Some(idx) = self.locals.get_index_of(name) {
                idx
            } else {
                self.locals.insert(name.to_string());
                self.locals.get_index_of(name).unwrap()
            };
            ctx.emit(NodeOp::StoreLocal(index));
        }

        // self.environment.define(name.to_string(), v);

        Ok(ASTValue::None)
    }

    fn variable_assignment(
        &mut self,
        name: String,
        op: &AssignmentOperator,
        swizzle: &[u8],
        field_path: &[String],
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        // Treat built-ins as pseudo-variables with custom load/store ops
        #[allow(dead_code)]
        enum Target {
            Builtin { load: NodeOp, store: NodeOp },
            Local { index: usize },
            Global { index: usize },
        }

        /*
        let target = if name == "color" {
            Some(Target::Builtin {
                load: NodeOp::Color,
                store: NodeOp::SetColor,
            })
        } else if name == "uv" {
            Some(Target::Builtin {
                load: NodeOp::UV,
                store: NodeOp::SetUV,
            })
        } else if name == "normal" {
            Some(Target::Builtin {
                load: NodeOp::Normal,
                store: NodeOp::SetNormal,
            })
        } else if name == "roughness" {
            Some(Target::Builtin {
                load: NodeOp::Roughness,
                store: NodeOp::SetRoughness,
            })
        } else if name == "metallic" {
            Some(Target::Builtin {
                load: NodeOp::Metallic,
                store: NodeOp::SetMetallic,
            })
        } else if name == "emissive" {
            Some(Target::Builtin {
                load: NodeOp::Emissive,
                store: NodeOp::SetEmissive,
            })
        } else if name == "opacity" {
            Some(Target::Builtin {
                load: NodeOp::Opacity,
                store: NodeOp::SetOpacity,
            })
        } else*/

        let target = if let Some(index) = self.locals.get_index_of(&name) {
            Some(Target::Local { index })
        } else if let Some(&g) = ctx.globals.get(&name) {
            Some(Target::Global { index: g as usize })
        } else {
            None
        };

        if let Some(target) = target {
            // Small helpers to emit load/store for any target kind
            let load_target = |ctx: &mut Context| match target {
                Target::Builtin { ref load, .. } => ctx.emit(load.clone()),
                Target::Local { index } => ctx.emit(NodeOp::LoadLocal(index)),
                Target::Global { index } => ctx.emit(NodeOp::LoadGlobal(index)),
            };
            let store_target = |ctx: &mut Context| match target {
                Target::Builtin { ref store, .. } => ctx.emit(store.clone()),
                Target::Local { index } => ctx.emit(NodeOp::StoreLocal(index)),
                Target::Global { index } => ctx.emit(NodeOp::StoreGlobal(index)),
            };

            // Helper to emit arithmetic op for compound assignments
            let emit_comp = |ctx: &mut Context| match op {
                AssignmentOperator::AddAssign => ctx.emit(NodeOp::Add),
                AssignmentOperator::SubtractAssign => ctx.emit(NodeOp::Sub),
                AssignmentOperator::MultiplyAssign => ctx.emit(NodeOp::Mul),
                AssignmentOperator::DivideAssign => ctx.emit(NodeOp::Div),
                AssignmentOperator::Assign => unreachable!(),
            };

            if swizzle.is_empty() && field_path.is_empty() {
                // Non-swizzled path
                match op {
                    AssignmentOperator::Assign => {
                        _ = expression.accept(self, ctx)?; // RHS
                        store_target(ctx);
                    }
                    _ => {
                        // t = t (op) rhs
                        load_target(ctx); // t
                        _ = expression.accept(self, ctx)?; // t, rhs
                        emit_comp(ctx); // t (op) rhs
                        store_target(ctx); // store back
                    }
                }
            } else if !swizzle.is_empty() {
                // Swizzled path (handle both plain and compound)
                match op {
                    AssignmentOperator::Assign => {
                        _ = expression.accept(self, ctx)?; // rhs
                        load_target(ctx); // rhs, t
                        ctx.emit(NodeOp::Swap); // t, rhs
                        ctx.emit(NodeOp::SetComponents(swizzle.to_vec())); // t'
                        store_target(ctx);
                    }
                    _ => {
                        // t.swz = t.swz (op) rhs
                        load_target(ctx); // t
                        ctx.emit(NodeOp::Dup); // t, t
                        ctx.emit(NodeOp::GetComponents(swizzle.to_vec())); // t, a
                        _ = expression.accept(self, ctx)?; // t, a, rhs
                        emit_comp(ctx); // t, (a op rhs)
                        ctx.emit(NodeOp::SetComponents(swizzle.to_vec())); // t'
                        store_target(ctx);
                    }
                }
            } else if field_path.len() == 1 {
                if let Some(idx) = Self::component_alias(&field_path[0]) {
                    let swz = vec![idx];
                    match op {
                        AssignmentOperator::Assign => {
                            _ = expression.accept(self, ctx)?; // rhs
                            load_target(ctx); // rhs, t
                            ctx.emit(NodeOp::Swap); // t, rhs
                            ctx.emit(NodeOp::SetComponents(swz)); // t'
                            store_target(ctx);
                        }
                        _ => {
                            load_target(ctx); // t
                            ctx.emit(NodeOp::Dup); // t,t
                            ctx.emit(NodeOp::GetComponents(swz.clone())); // t,a
                            _ = expression.accept(self, ctx)?; // t,a,rhs
                            emit_comp(ctx); // t,(a op rhs)
                            ctx.emit(NodeOp::SetComponents(swz)); // t'
                            store_target(ctx);
                        }
                    }
                } else if field_path[0] == "string" {
                    match op {
                        AssignmentOperator::Assign => {
                            _ = expression.accept(self, ctx)?; // rhs
                            load_target(ctx); // rhs, t
                            ctx.emit(NodeOp::Swap); // t, rhs
                            ctx.emit(NodeOp::SetString); // t'
                            store_target(ctx);
                        }
                        _ => {
                            load_target(ctx); // t
                            ctx.emit(NodeOp::Dup); // t,t
                            ctx.emit(NodeOp::GetString); // t, s
                            _ = expression.accept(self, ctx)?; // t, s, rhs
                            emit_comp(ctx); // t, combined
                            ctx.emit(NodeOp::SetString);
                            store_target(ctx);
                        }
                    }
                }
            } else if field_path.len() == 1 && field_path[0] == "string" {
                match op {
                    AssignmentOperator::Assign => {
                        _ = expression.accept(self, ctx)?; // rhs
                        load_target(ctx); // rhs, t
                        ctx.emit(NodeOp::Swap); // t, rhs
                        ctx.emit(NodeOp::SetString); // t'
                        store_target(ctx);
                    }
                    _ => {
                        load_target(ctx); // t
                        ctx.emit(NodeOp::Dup); // t,t
                        ctx.emit(NodeOp::GetString); // t, s
                        _ = expression.accept(self, ctx)?; // t, s, rhs
                        emit_comp(ctx); // t, combined (numeric op on strings makes little sense, but follow pattern)
                        ctx.emit(NodeOp::SetString);
                        store_target(ctx);
                    }
                }
            }
        }

        Ok(ASTValue::None)
    }

    fn variable(
        &mut self,
        name: String,
        swizzle: &[u8],
        field_path: &[String],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let mut rc = ASTValue::None;

        /*
        if name == "uv" {
            ctx.emit(NodeOp::UV);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "normal" {
            ctx.emit(NodeOp::Normal);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "color" {
            ctx.emit(NodeOp::Color);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "roughness" {
            ctx.emit(NodeOp::Roughness);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "metallic" {
            ctx.emit(NodeOp::Metallic);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "emissive" {
            ctx.emit(NodeOp::Emissive);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "opacity" {
            ctx.emit(NodeOp::Opacity);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "bump" {
            ctx.emit(NodeOp::Bump);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if name == "hitpoint" {
            ctx.emit(NodeOp::Hitpoint);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else */

        if name == "time" {
            ctx.emit(NodeOp::Time);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            } else if field_path.len() == 1 && Self::is_string_alias(&field_path[0]) {
                ctx.emit(NodeOp::GetString);
            } else if field_path.len() == 1 {
                if let Some(idx) = Self::component_alias(&field_path[0]) {
                    ctx.emit(NodeOp::GetComponents(vec![idx]));
                }
            }
        } else if let Some(index) = self.locals.get_index_of(&name) {
            ctx.emit(NodeOp::LoadLocal(index));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            } else if field_path.len() == 1 && Self::is_string_alias(&field_path[0]) {
                ctx.emit(NodeOp::GetString);
            } else if field_path.len() == 1 {
                if let Some(idx) = Self::component_alias(&field_path[0]) {
                    ctx.emit(NodeOp::GetComponents(vec![idx]));
                }
            }
        } else if let Some(index) = ctx.globals.get(&name) {
            ctx.emit(NodeOp::LoadGlobal(*index as usize));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            } else if field_path.len() == 1 && Self::is_string_alias(&field_path[0]) {
                ctx.emit(NodeOp::GetString);
            } else if field_path.len() == 1 {
                if let Some(idx) = Self::component_alias(&field_path[0]) {
                    ctx.emit(NodeOp::GetComponents(vec![idx]));
                }
            }
        } else if self.functions.contains_key(&name) || self.user_functions.contains_key(&name) {
            rc = ASTValue::Function(name.clone(), vec![], Box::new(ASTValue::None));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            } else if field_path.len() == 1 && Self::is_string_alias(&field_path[0]) {
                ctx.emit(NodeOp::GetString);
            } else if field_path.len() == 1 {
                if let Some(idx) = Self::component_alias(&field_path[0]) {
                    ctx.emit(NodeOp::GetComponents(vec![idx]));
                }
            }
        } else {
            return Err(RuntimeError::new(
                format!("Unknown identifier '{}'", name),
                loc,
            ));
        }
        // else if let Some(vv) = self.environment.get(&name) {
        //     rc = vv;

        Ok(rc)
    }

    fn value(
        &mut self,
        value: ASTValue,
        _swizzle: &[u8],
        _field_path: &[String],
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        match &value {
            ASTValue::Boolean(b) => {
                ctx.emit(NodeOp::Push(if *b {
                    VMValue::new_with_string(1.0, 1.0, 1.0, "bool")
                } else {
                    VMValue::new_with_string(0.0, 0.0, 0.0, "bool")
                }));
            }
            ASTValue::Float(f) => {
                ctx.emit(NodeOp::Push(VMValue::new_with_string(*f, *f, *f, "float")));
            }
            ASTValue::Float2(x, y) => {
                _ = x.accept(self, ctx)?.to_float().unwrap_or_default();
                _ = y.accept(self, ctx)?.to_float().unwrap_or_default();
                ctx.emit(NodeOp::Pack2);
            }
            ASTValue::Float3(x, y, z) => {
                _ = x.accept(self, ctx)?.to_float().unwrap_or_default();
                _ = y.accept(self, ctx)?.to_float().unwrap_or_default();
                _ = z.accept(self, ctx)?.to_float().unwrap_or_default();

                ctx.emit(NodeOp::Pack3);
            }
            ASTValue::String(s) => {
                ctx.emit(NodeOp::Push(VMValue::from_string(s.clone())));
            }
            _ => {}
        };

        Ok(ASTValue::None)
    }

    fn unary(
        &mut self,
        op: &UnaryOperator,
        expr: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = expr.accept(self, ctx)?;

        match op {
            UnaryOperator::Negate => ctx.emit(NodeOp::Not),
            UnaryOperator::Minus => ctx.emit(NodeOp::Neg),
        }

        Ok(ASTValue::None)
    }

    fn equality(
        &mut self,
        left: &Expr,
        op: &EqualityOperator,
        right: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = left.accept(self, ctx)?;
        _ = right.accept(self, ctx)?;

        match op {
            EqualityOperator::NotEqual => ctx.emit(NodeOp::Ne),
            EqualityOperator::Equal => ctx.emit(NodeOp::Eq),
        }

        Ok(ASTValue::None)
    }

    fn comparison(
        &mut self,
        left: &Expr,
        op: &ComparisonOperator,
        right: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = left.accept(self, ctx)?;
        _ = right.accept(self, ctx)?;

        match op {
            ComparisonOperator::Greater => ctx.emit(NodeOp::Gt),
            ComparisonOperator::GreaterEqual => ctx.emit(NodeOp::Ge),
            ComparisonOperator::Less => ctx.emit(NodeOp::Lt),
            ComparisonOperator::LessEqual => ctx.emit(NodeOp::Le),
        }

        Ok(ASTValue::None)
    }

    fn binary(
        &mut self,
        left: &Expr,
        op: &BinaryOperator,
        right: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = left.accept(self, ctx)?;
        _ = right.accept(self, ctx)?;

        match op {
            BinaryOperator::Add => {
                ctx.emit(NodeOp::Add);
            }
            BinaryOperator::Subtract => {
                ctx.emit(NodeOp::Sub);
            }
            BinaryOperator::Multiply => {
                ctx.emit(NodeOp::Mul);
            }
            BinaryOperator::Divide => {
                ctx.emit(NodeOp::Div);
            }
            BinaryOperator::Mod => {
                ctx.emit(NodeOp::Mod);
            }
        }

        Ok(ASTValue::None)
    }

    fn grouping(
        &mut self,
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        expression.accept(self, ctx)
    }

    fn func_call(
        &mut self,
        callee: &Expr,
        swizzle: &[u8],
        _field_path: &[String],
        args: &[Box<Expr>],
        loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let callee = callee.accept(self, ctx)?;

        if let ASTValue::Function(name, _func_args, _returns) = callee {
            if let Some(func) = &self.functions.get(&name).cloned() {
                if name == "format" {
                    for arg in args {
                        _ = arg.accept(self, ctx)?;
                    }
                    ctx.emit(NodeOp::Format(args.len() as u8));
                    if !swizzle.is_empty() {
                        ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
                    }
                } else if name == "print" {
                    for arg in args {
                        _ = arg.accept(self, ctx)?;
                    }
                    ctx.emit(NodeOp::Print(args.len() as u8));
                } else {
                    if func.arguments as usize == args.len() {
                        for arg in args {
                            _ = arg.accept(self, ctx)?;
                        }
                        ctx.emit(func.op.clone());
                        if !swizzle.is_empty() {
                            ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
                        }
                    } else {
                        return Err(RuntimeError::new(
                            format!(
                                "Wrong amount of arguments for '{}', expected '{}' got '{}'",
                                name,
                                func.arguments as usize,
                                args.len(),
                            ),
                            loc,
                        ));
                    }
                }
            } else if let Some((arity, _params, locals_len, index)) = self.user_functions.get(&name)
            {
                let func_index = *index;
                let total_locals = *locals_len;
                if *arity != args.len() {
                    return Err(RuntimeError::new(
                        format!(
                            "Wrong amount of arguments for '{}', expected '{}' got '{}'",
                            name,
                            arity,
                            args.len()
                        ),
                        loc,
                    ));
                }

                for arg in args {
                    _ = arg.accept(self, ctx)?;
                }
                ctx.emit(NodeOp::FunctionCall(
                    args.len() as u8,
                    total_locals as u8,
                    func_index,
                ));
                if !swizzle.is_empty() {
                    ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
                }
            } else {
                return Err(RuntimeError::new(
                    format!("Unknown function '{}'", name),
                    loc,
                ));
            }
        } else {
            return Err(RuntimeError::new(format!("Unknown function ''"), loc));
        }

        Ok(ASTValue::None)
    }

    fn struct_declaration(
        &mut self,
        _name: &str,
        _fields: &[(String, ASTValue)],
        _loc: &Location,
        _ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        /*
        let mut size: usize = 0;

        for (_, field) in fields {
            size += field.components() * ctx.precision.size();
        }

        ctx.structs
            .insert(name.to_string(), fields.to_vec().clone());

        ctx.struct_sizes.insert(name.to_string(), size);

        Ok(ASTValue::Struct("".to_string(), None, vec![]))
        */
        Ok(ASTValue::None)
    }

    /// Create a voxel box
    fn function_declaration(
        &mut self,
        objectd: &FunctionD,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        self.locals.clear();

        // Compile locals (and their optional default values)
        let mut cp: IndexMap<String, Option<Vec<NodeOp>>> = IndexMap::default();
        for (name, ast) in &objectd.locals {
            let mut def: Option<Vec<NodeOp>> = None;
            if let Some(ast) = ast {
                ctx.add_custom_target();
                _ = ast.accept(self, ctx)?;
                if let Some(code) = ctx.take_last_custom_target() {
                    def = Some(code);
                }
            }
            cp.insert(name.clone(), def);
            self.locals.insert(name.clone());
        }

        ctx.add_custom_target();

        let index = ctx.program.user_functions.len();

        // locals_len counts parameters + any locals declared in the body.
        let locals_len = cp.len();
        self.user_functions.insert(
            objectd.name.clone(),
            (objectd.arity, cp.clone(), locals_len, index),
        );

        objectd.block.accept(self, ctx)?;
        if let Some(mut codes) = ctx.take_last_custom_target() {
            optimize(&mut codes);
            ctx.program
                .user_functions
                .push(Arc::from(codes.into_boxed_slice()));
            ctx.program.user_functions_locals.push(locals_len);
            ctx.program
                .user_functions_name_map
                .insert(objectd.name.clone(), index);
        }

        self.locals.clear();

        Ok(ASTValue::None)
    }

    fn return_stmt(
        &mut self,
        expr: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        _ = expr.accept(self, ctx)?;
        ctx.emit(NodeOp::Return);

        Ok(ASTValue::None)
    }

    fn if_stmt(
        &mut self,
        cond: &Expr,
        then_stmt: &Stmt,
        else_stmt: &Option<Box<Stmt>>,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        ctx.add_custom_target();
        _ = then_stmt.accept(self, ctx)?;
        let mut then_code = vec![];
        if let Some(code) = ctx.take_last_custom_target() {
            then_code = code;
        }

        let mut else_code = None;

        if let Some(else_stmt) = else_stmt {
            ctx.add_custom_target();
            _ = else_stmt.accept(self, ctx)?;
            if let Some(code) = ctx.take_last_custom_target() {
                else_code = Some(code);
            }
        }

        _ = cond.accept(self, ctx)?;
        ctx.emit(NodeOp::If(then_code, else_code));

        Ok(ASTValue::None)
    }

    fn ternary(
        &mut self,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        // Build "then" branch code
        ctx.add_custom_target();
        let then_returns = then_expr.accept(self, ctx)?;
        let mut then_code = Vec::new();
        if let Some(code) = ctx.take_last_custom_target() {
            then_code = code;
        }

        // Build "else" branch code
        ctx.add_custom_target();
        let else_returns = else_expr.accept(self, ctx)?;
        let mut else_code = Vec::new();
        if let Some(code) = ctx.take_last_custom_target() {
            else_code = code;
        }

        // Emit condition and branch
        _ = cond.accept(self, ctx)?;
        ctx.emit(NodeOp::If(then_code, Some(else_code)));

        // Return a best-effort type hint; prefer then branch, else fallback.
        if let ASTValue::None = then_returns {
            Ok(else_returns)
        } else {
            Ok(then_returns)
        }
    }

    fn for_stmt(
        &mut self,
        init: &[Box<Stmt>],
        conditions: &[Box<Expr>],
        incr: &[Box<Expr>],
        body_stmt: &Stmt,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let mut init_code = vec![];
        ctx.add_custom_target();
        for stmt in init {
            _ = stmt.accept(self, ctx)?;
        }
        if let Some(code) = ctx.take_last_custom_target() {
            init_code = code;
        }

        let mut cond_code = vec![];
        ctx.add_custom_target();
        for stmt in conditions {
            _ = stmt.accept(self, ctx)?;
        }
        if let Some(code) = ctx.take_last_custom_target() {
            cond_code = code;
        }

        let mut incr_code = vec![];
        ctx.add_custom_target();
        for stmt in incr {
            _ = stmt.accept(self, ctx)?;
        }
        if let Some(code) = ctx.take_last_custom_target() {
            incr_code = code;
        }

        let mut body_code = vec![];
        ctx.add_custom_target();
        body_stmt.accept(self, ctx)?;
        if let Some(code) = ctx.take_last_custom_target() {
            body_code = code;
        }

        ctx.emit(NodeOp::For(init_code, cond_code, incr_code, body_code));

        Ok(ASTValue::None)
    }

    fn while_stmt(
        &mut self,
        _cond: &Expr,
        _body_stmt: &Stmt,
        _loc: &Location,
        _ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        /*
                ctx.add_line();

                let instr = "(block".to_string();
                ctx.add_wat(&instr);
                ctx.add_indention();

                let instr = "(loop".to_string();
                ctx.add_wat(&instr);
                ctx.add_indention();

                self.break_depth.push(0);

                let _rc = cond.accept(self, ctx)?;

                let instr = "(i32.eqz)".to_string();
                ctx.add_wat(&instr);

                let instr = "(br_if 1)".to_string();
                ctx.add_wat(&instr);

                let _rc = body_stmt.accept(self, ctx)?;

                let instr = "(br 0)".to_string();
                ctx.add_wat(&instr);

                self.break_depth.pop();

                ctx.remove_indention();
                ctx.add_wat(")");

                ctx.remove_indention();
                ctx.add_wat(")");
        */
        Ok(ASTValue::None)
    }

    fn break_stmt(
        &mut self,
        _loc: &Location,
        _ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        Ok(ASTValue::None)
    }

    fn empty_stmt(&mut self, _ctx: &mut Context) -> Result<ASTValue, RuntimeError> {
        Ok(ASTValue::None)
    }

    fn logical_expr(
        &mut self,
        left: &Expr,
        op: &LogicalOperator,
        right: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let _l = left.accept(self, ctx)?;
        let _r = right.accept(self, ctx)?;

        match op {
            LogicalOperator::And => {
                ctx.emit(NodeOp::And);
            }
            LogicalOperator::Or => {
                ctx.emit(NodeOp::Or);
            }
        }

        Ok(ASTValue::None)
    }
}
