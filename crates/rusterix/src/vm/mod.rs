pub mod ast;
pub mod astvalue;
pub mod builtin;
pub mod compile;
pub mod context;
pub mod environment;
pub mod errors;
pub mod idverifier;
pub mod module;
pub mod node;
pub mod objectd;
pub mod optimize;
pub mod parser;
pub mod renderbuffer;
pub mod scanner;
pub mod value;

pub use self::{
    ast::{
        AssignmentOperator, BinaryOperator, ComparisonOperator, EqualityOperator, Expr, Location,
        LogicalOperator, Stmt, UnaryOperator, Visitor,
    },
    astvalue::ASTValue,
    compile::CompileVisitor,
    context::Context,
    environment::Environment,
    errors::{ParseError, RuntimeError, VMError},
    idverifier::IdVerifier,
    module::Module,
    node::execution::Execution,
    node::{hosthandler::HostHandler, nodeop::NodeOp, program::Program},
    optimize::optimize,
    parser::Parser,
    renderbuffer::RenderBuffer,
    scanner::{Scanner, Token, TokenType},
    value::VMValue,
};

use rustc_hash::FxHashMap;
use std::path::PathBuf;
use theframework::theui::ThePalette;

pub struct VM {
    path: PathBuf,
    pub context: Context,
    defaults: Option<Module>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            context: Context::new(FxHashMap::default()),
            defaults: None,
        }
    }

    // Parse the source code into a module.
    pub fn parse(&mut self, path: PathBuf) -> Result<Module, ParseError> {
        self.path = path.clone();
        let mut parser = Parser::new();
        let module = parser.compile(path.clone())?;

        Ok(module)
    }

    // Parse the source code into a module.
    pub fn parse_str(&mut self, str: &str) -> Result<Module, ParseError> {
        self.path = PathBuf::from("string_based.shpz");
        let mut parser: Parser = Parser::new();

        let module = parser.compile_module("main".into(), str.into(), self.path.clone())?;

        Ok(module)
    }

    // Compile the source code
    pub fn compile(&mut self, module: &Module) -> Result<(), RuntimeError> {
        let mut visitor: CompileVisitor = CompileVisitor::new();
        self.context = Context::new(module.globals.clone());

        // Add default materials
        if let Some(defs) = &self.defaults {
            for statement in defs.stmts.clone() {
                _ = statement.accept(&mut visitor, &mut self.context)?;
            }
        }

        for statement in module.stmts.clone() {
            _ = statement.accept(&mut visitor, &mut self.context)?;
        }

        // println!("{:?}", self.context.program.user_functions);
        // optimize(&mut self.context.program.body);

        self.context.program.globals = self.context.globals.len();

        Ok(())
    }

    /// Parse and compile a string in one step, returning the compiled program or a unified error.
    pub fn prepare_str(&mut self, src: &str) -> Result<Program, VMError> {
        let module = self.parse_str(src).map_err(VMError::from)?;
        self.compile(&module).map_err(VMError::from)?;
        Ok(self.context.program.clone())
    }

    /// Compile the voxels into the VoxelGrid.
    pub fn execute(&mut self, _palette: &ThePalette) -> Option<VMValue> {
        let mut execution = Execution::new(self.context.globals.len());

        // Execute the main program to compile all voxels.
        execution.execute(&&self.context.program.body, &self.context.program);

        execution.stack.pop()
    }

    pub fn execute_string(&mut self, str: &str, palette: &ThePalette) -> Option<VMValue> {
        let result = self.parse_str(str);
        match result {
            Ok(module) => {
                let result = self.compile(&module);
                match result {
                    Ok(_) => {
                        return self.execute(palette);
                    }
                    Err(err) => println!("{}", err.to_string()),
                }
            }
            Err(err) => println!("{}", err.to_string()),
        }

        None
    }

    /// Imported paths
    pub fn imported_paths(&self) -> Vec<PathBuf> {
        self.context.imported_paths.clone()
    }

    /// Get the current time
    pub fn get_time(&self) -> u128 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().unwrap().performance().unwrap().now() as u128
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let stop = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            stop.as_millis()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let mut script = VM::default();
        let result = script.execute_string("let a = 2; a + 2;".into(), &ThePalette::default());
        assert_eq!(result.unwrap().x, 4.0);
    }

    #[test]
    fn fib() {
        let mut script = VM::default();
        let fib = r#"
        fn fib(n) {
            if n <= 1 {
                return n;
            } else {
                return fib(n - 1) + fib(n - 2);
            }
        }
        fib(27);
        "#;
        let result = script.execute_string(fib.into(), &ThePalette::default());
        assert_eq!(result.unwrap().x, 196418.0);
    }

    #[test]
    fn string_literal() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let greeting = "hello"; greeting;"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().as_string(), Some("hello"));
    }

    #[test]
    fn string_compare_literal() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let name = "abc"; name == "abc";"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().x, 1.0);
    }

    #[test]
    fn ternary_string() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let flag = 1; flag ? "yes" : "no";"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().as_string(), Some("yes"));
    }

    #[test]
    fn user_event_invocation() {
        let mut script = VM::default();
        let module = script
            .parse_str(
                r#"
                fn user_event(event, value) {
                    // no-op handler
                }
                "#,
            )
            .unwrap();
        script.compile(&module).unwrap();

        let func_index = script
            .context
            .program
            .user_functions_name_map
            .get("user_event")
            .copied()
            .unwrap();

        let mut exec = Execution::new(script.context.globals.len());
        exec.reset(script.context.globals.len());
        let args = [VMValue::broadcast(1.0), VMValue::broadcast(2.0)];
        let result = exec.execute_function(&args, func_index, &script.context.program);
        assert_eq!(result.x, 0.0);
    }

    #[test]
    fn match_syntax_event() {
        let mut script = VM::default();
        let module = script
            .parse_str(
                r#"
                fn user_event(event, value) {
                    match event {
                        "key_down" {
                            if value == "w" {
                                action("forward");
                            }
                        }
                        "key_up" {
                            action("none");
                        }
                        _ {
                            action("noop");
                        }
                    }
                }
                "#,
            )
            .unwrap();
        script.compile(&module).unwrap();

        let func_index = script
            .context
            .program
            .user_functions_name_map
            .get("user_event")
            .copied()
            .unwrap();

        let mut exec = Execution::new(script.context.globals.len());

        exec.reset(script.context.globals.len());
        let args = [VMValue::from_string("key_down"), VMValue::from_string("w")];
        let _ = exec.execute_function(&args, func_index, &script.context.program);
        assert_eq!(
            exec.outputs
                .get("action")
                .and_then(|v| v.as_string())
                .unwrap(),
            "forward"
        );

        exec.reset(script.context.globals.len());
        let args = [VMValue::from_string("key_up"), VMValue::from_string("w")];
        let _ = exec.execute_function(&args, func_index, &script.context.program);
        assert_eq!(
            exec.outputs
                .get("action")
                .and_then(|v| v.as_string())
                .unwrap(),
            "none"
        );
    }

    #[test]
    fn format_variadic() {
        let mut script = VM::default();
        let result = script.execute_string(r#"format("pos {} {}", 1, 2);"#, &ThePalette::default());
        assert_eq!(result.unwrap().as_string(), Some("pos 1 2"));
    }

    #[test]
    fn print_multiple_args() {
        let mut script = VM::default();
        let result =
            script.execute_string(r#"print("hello", 1, 2); "done";"#, &ThePalette::default());
        assert_eq!(result.unwrap().as_string(), Some("done"));
    }
}
