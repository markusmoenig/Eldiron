use crate::objectd::FunctionD;
use crate::{
    ASTValue, AssignmentOperator, BinaryOperator, ComparisonOperator, Context, Environment,
    EqualityOperator, Expr, Location, LogicalOperator, Module, NodeOp, PatternKind, RuntimeError,
    Stmt, UnaryOperator, Value, Visitor, optimize,
};
use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use vek::Vec3;

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

    user_functions: IndexMap<String, (usize, IndexMap<String, Option<Vec<NodeOp>>>, usize)>,

    /// List of local variables which are in scope (inside functions)
    locals: IndexSet<String>,
}

impl Visitor for CompileVisitor {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut functions: FxHashMap<String, ASTFunction> = FxHashMap::default();
        functions.insert(
            "length".to_string(),
            ASTFunction {
                name: "length".to_string(),
                arguments: 1,
                op: NodeOp::Length,
            },
        );
        functions.insert(
            "length2".to_string(),
            ASTFunction {
                name: "length2".to_string(),
                arguments: 1,
                op: NodeOp::Length2,
            },
        );
        functions.insert(
            "length3".to_string(),
            ASTFunction {
                name: "length3".to_string(),
                arguments: 1,
                op: NodeOp::Length3,
            },
        );
        functions.insert(
            "abs".to_string(),
            ASTFunction {
                name: "abs".to_string(),
                arguments: 1,
                op: NodeOp::Abs,
            },
        );
        functions.insert(
            "sin".to_string(),
            ASTFunction {
                name: "sin".to_string(),
                arguments: 1,
                op: NodeOp::Sin,
            },
        );
        functions.insert(
            "sin1".to_string(),
            ASTFunction {
                name: "sin1".to_string(),
                arguments: 1,
                op: NodeOp::Sin1,
            },
        );
        functions.insert(
            "sin2".to_string(),
            ASTFunction {
                name: "sin2".to_string(),
                arguments: 1,
                op: NodeOp::Sin2,
            },
        );
        functions.insert(
            "cos".to_string(),
            ASTFunction {
                name: "cos".to_string(),
                arguments: 1,
                op: NodeOp::Cos,
            },
        );
        functions.insert(
            "cos1".to_string(),
            ASTFunction {
                name: "cos1".to_string(),
                arguments: 1,
                op: NodeOp::Cos1,
            },
        );
        functions.insert(
            "cos2".to_string(),
            ASTFunction {
                name: "cos2".to_string(),
                arguments: 1,
                op: NodeOp::Cos2,
            },
        );
        functions.insert(
            "normalize".to_string(),
            ASTFunction {
                name: "normalize".to_string(),
                arguments: 1,
                op: NodeOp::Normalize,
            },
        );
        functions.insert(
            "tan".to_string(),
            ASTFunction {
                name: "tan".to_string(),
                arguments: 1,
                op: NodeOp::Tan,
            },
        );
        functions.insert(
            "atan".to_string(),
            ASTFunction {
                name: "atan".to_string(),
                arguments: 1,
                op: NodeOp::Atan,
            },
        );
        functions.insert(
            "atan2".to_string(),
            ASTFunction {
                name: "atan2".to_string(),
                arguments: 2,
                op: NodeOp::Atan2,
            },
        );
        functions.insert(
            "dot".to_string(),
            ASTFunction {
                name: "dot".to_string(),
                arguments: 2,
                op: NodeOp::Dot,
            },
        );
        functions.insert(
            "dot2".to_string(),
            ASTFunction {
                name: "dot2".to_string(),
                arguments: 2,
                op: NodeOp::Dot2,
            },
        );
        functions.insert(
            "dot3".to_string(),
            ASTFunction {
                name: "dot3".to_string(),
                arguments: 3,
                op: NodeOp::Dot3,
            },
        );
        functions.insert(
            "cross".to_string(),
            ASTFunction {
                name: "cross".to_string(),
                arguments: 2,
                op: NodeOp::Cross,
            },
        );
        functions.insert(
            "floor".to_string(),
            ASTFunction {
                name: "floor".to_string(),
                arguments: 1,
                op: NodeOp::Floor,
            },
        );
        functions.insert(
            "ceil".to_string(),
            ASTFunction {
                name: "ceil".to_string(),
                arguments: 1,
                op: NodeOp::Ceil,
            },
        );
        functions.insert(
            "fract".to_string(),
            ASTFunction {
                name: "fract".to_string(),
                arguments: 1,
                op: NodeOp::Fract,
            },
        );
        functions.insert(
            "radians".to_string(),
            ASTFunction {
                name: "radians".to_string(),
                arguments: 1,
                op: NodeOp::Radians,
            },
        );
        functions.insert(
            "degrees".to_string(),
            ASTFunction {
                name: "degrees".to_string(),
                arguments: 1,
                op: NodeOp::Degrees,
            },
        );
        functions.insert(
            "min".to_string(),
            ASTFunction {
                name: "min".to_string(),
                arguments: 2,
                op: NodeOp::Min,
            },
        );
        functions.insert(
            "max".to_string(),
            ASTFunction {
                name: "max".to_string(),
                arguments: 2,
                op: NodeOp::Max,
            },
        );
        functions.insert(
            "mix".to_string(),
            ASTFunction {
                name: "mix".to_string(),
                arguments: 3,
                op: NodeOp::Mix,
            },
        );
        functions.insert(
            "smoothstep".to_string(),
            ASTFunction {
                name: "smoothstep".to_string(),
                arguments: 3,
                op: NodeOp::Smoothstep,
            },
        );
        functions.insert(
            "step".to_string(),
            ASTFunction {
                name: "step".to_string(),
                arguments: 2,
                op: NodeOp::Step,
            },
        );
        functions.insert(
            "mod".to_string(),
            ASTFunction {
                name: "mod".to_string(),
                arguments: 2,
                op: NodeOp::Mod,
            },
        );
        functions.insert(
            "clamp".to_string(),
            ASTFunction {
                name: "clamp".to_string(),
                arguments: 3,
                op: NodeOp::Clamp,
            },
        );
        functions.insert(
            "sqrt".to_string(),
            ASTFunction {
                name: "sqrt".to_string(),
                arguments: 1,
                op: NodeOp::Sqrt,
            },
        );
        functions.insert(
            "log".to_string(),
            ASTFunction {
                name: "log".to_string(),
                arguments: 1,
                op: NodeOp::Log,
            },
        );
        functions.insert(
            "pow".to_string(),
            ASTFunction {
                name: "pow".to_string(),
                arguments: 2,
                op: NodeOp::Pow,
            },
        );
        functions.insert(
            "print".to_string(),
            ASTFunction {
                name: "print".to_string(),
                arguments: 1,
                op: NodeOp::Print,
            },
        );
        functions.insert(
            "sample".to_string(),
            ASTFunction {
                name: "sample".to_string(),
                arguments: 2,
                op: NodeOp::Sample,
            },
        );
        functions.insert(
            "sample_normal".to_string(),
            ASTFunction {
                name: "sample_normal".to_string(),
                arguments: 2,
                op: NodeOp::SampleNormal,
            },
        );

        functions.insert(
            "alloc".to_string(),
            ASTFunction {
                name: "alloc".to_string(),
                arguments: 2,
                op: NodeOp::Alloc,
            },
        );
        functions.insert(
            "iterate".to_string(),
            ASTFunction {
                name: "iterate".to_string(),
                arguments: 2,
                op: NodeOp::Iterate,
            },
        );
        functions.insert(
            "save".to_string(),
            ASTFunction {
                name: "save".to_string(),
                arguments: 2,
                op: NodeOp::Save,
            },
        );
        functions.insert(
            "rotate2d".to_string(),
            ASTFunction {
                name: "rotate2d".to_string(),
                arguments: 2,
                op: NodeOp::Rotate2D,
            },
        );
        functions.insert(
            "palette".to_string(),
            ASTFunction {
                name: "palette".to_string(),
                arguments: 1,
                op: NodeOp::PaletteIndex,
            },
        );
        functions.insert(
            "round".to_string(),
            ASTFunction {
                name: "round".to_string(),
                arguments: 1,
                op: NodeOp::Round,
            },
        );

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

        if let Some(index) = self.locals.get_index_of(name) {
            ctx.emit(NodeOp::StoreLocal(index));
        } else if let Some(index) = ctx.globals.get(name) {
            ctx.emit(NodeOp::StoreGlobal(*index as usize));
        }

        // self.environment.define(name.to_string(), v);

        Ok(ASTValue::None)
    }

    fn variable_assignment(
        &mut self,
        name: String,
        op: &AssignmentOperator,
        swizzle: &[u8],
        _field_path: &[String],
        expression: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        // Treat built-ins as pseudo-variables with custom load/store ops
        enum Target {
            Builtin { load: NodeOp, store: NodeOp },
            Local { index: usize },
            Global { index: usize },
        }

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
        } else if let Some(index) = self.locals.get_index_of(&name) {
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

            if swizzle.is_empty() {
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
            } else {
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
            }
        }

        Ok(ASTValue::None)
    }

    fn variable(
        &mut self,
        name: String,
        swizzle: &[u8],
        _field_path: &[String],
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        let mut rc = ASTValue::None;

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
        } else if name == "time" {
            ctx.emit(NodeOp::Time);
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if self.functions.contains_key(&name) {
            rc = ASTValue::Function(name.clone(), vec![], Box::new(ASTValue::None));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if self.user_functions.contains_key(&name) {
            rc = ASTValue::Function(name.clone(), vec![], Box::new(ASTValue::None));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else if let Some(index) = self.locals.get_index_of(&name) {
            ctx.emit(NodeOp::LoadLocal(index));
            if !swizzle.is_empty() {
                ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
            }
        } else {
            if let Some(index) = ctx.globals.get(&name) {
                ctx.emit(NodeOp::LoadGlobal(*index as usize));
                if !swizzle.is_empty() {
                    ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
                }
            }
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
                    Value::broadcast(1.0)
                } else {
                    Value::broadcast(0.0)
                }));
            }
            ASTValue::Float(f) => {
                ctx.emit(NodeOp::Push(Value::broadcast(*f)));
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
            if name == "sample" || name == "sample_normal" {
                if 2 == args.len() {
                    _ = args[0].accept(self, ctx)?;

                    ctx.add_custom_target();
                    _ = args[1].accept(self, ctx)?;
                    if let Some(op) = ctx.take_last_custom_target() {
                        let mut success = false;
                        match op[0] {
                            NodeOp::Push(val) => {
                                if let Some(str) = ctx.program.strings.get(val.x as usize) {
                                    if let Some(pattern) = PatternKind::from_name(str) {
                                        let index = pattern.to_index();
                                        ctx.emit(NodeOp::Push(Vec3::broadcast(index as f32)));
                                        success = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                        if !success {
                            // Value noise by default
                            ctx.emit(NodeOp::Push(Vec3::broadcast(1.0)));
                        }
                    }

                    if name == "sample" {
                        ctx.emit(NodeOp::Sample);
                    } else {
                        ctx.emit(NodeOp::SampleNormal);
                    }

                    if !swizzle.is_empty() {
                        ctx.emit(NodeOp::GetComponents(swizzle.to_vec()));
                    }
                } else {
                    return Err(RuntimeError::new(
                        format!(
                            "Wrong amount of arguments for '{}', expected '{}' got '{}'",
                            name,
                            2,
                            args.len(),
                        ),
                        loc,
                    ));
                }
            } else if let Some(func) = &self.functions.get(&name).cloned() {
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
            } else if let Some((arity, params, index)) = self.user_functions.get(&name) {
                let func_index = *index;
                let total_locals = params.len();
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

        self.user_functions
            .insert(objectd.name.clone(), (objectd.arity, cp.clone(), index));

        objectd.block.accept(self, ctx)?;
        if let Some(mut codes) = ctx.take_last_custom_target() {
            optimize(&mut codes);
            ctx.program
                .user_functions
                .push(Arc::from(codes.into_boxed_slice()));
            ctx.program
                .user_functions_name_map
                .insert(objectd.name.clone(), index);
            if objectd.name == "shade" {
                ctx.program.shade_index = Some(index);
                ctx.program.shade_locals = cp.len();
            }
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
        _cond: &Expr,
        then_expr: &Expr,
        _else_expr: &Expr,
        _loc: &Location,
        ctx: &mut Context,
    ) -> Result<ASTValue, RuntimeError> {
        /*
        ctx.add_line();
        let _rc = cond.accept(self, ctx)?;

        let param_name = format!("$_rpu_ternary_{}", ctx.ternary_counter);
        ctx.ternary_counter += 1;

        let instr = "(if".to_string();
        ctx.add_wat(&instr);
        ctx.add_indention();

        let instr = "(then".to_string();
        ctx.add_wat(&instr);
        ctx.add_indention();

        if let Some(d) = self.break_depth.last() {
            self.break_depth.push(d + 2);
        }*/

        let then_returns = then_expr.accept(self, ctx)?;

        /*
        let def_array = then_returns.write_definition("local", &param_name, &ctx.pr);
        for d in def_array {
            let c = format!("        {}\n", d);
            ctx.wat_locals.push_str(&c);
        }

        let a_set = then_returns.write_access("local.set", &param_name);
        for a in a_set.iter().rev() {
            ctx.add_wat(a);
        }

        ctx.remove_indention();
        ctx.add_wat(")");

        if let Some(d) = self.break_depth.last() {
            self.break_depth.push(d - 2);
        }

        if let Some(d) = self.break_depth.last() {
            self.break_depth.push(d + 2);
        }
        let instr = "(else".to_string();
        ctx.add_wat(&instr);
        ctx.add_indention();

        let else_returns = else_expr.accept(self, ctx)?;
        let b_set = else_returns.write_access("local.set", &param_name);
        for b in b_set.iter().rev() {
            ctx.add_wat(b);
        }

        ctx.remove_indention();
        ctx.add_wat(")");
        if let Some(d) = self.break_depth.last() {
            self.break_depth.push(d - 2);
        }

        ctx.remove_indention();
        ctx.add_wat(")");
        //ctx.add_line();

        let a_get = then_returns.write_access("local.get", &param_name);
        for a in a_get {
            ctx.add_wat(&a);
        }
        */

        Ok(then_returns)
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
