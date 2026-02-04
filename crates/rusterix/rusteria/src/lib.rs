pub mod ast;
pub mod astvalue;
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
pub mod textures;

pub type Value = vek::Vec3<f32>;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "embedded/"]
struct Embedded;

pub use crate::{
    ast::{
        AssignmentOperator, BinaryOperator, ComparisonOperator, EqualityOperator, Expr, Location,
        LogicalOperator, Stmt, UnaryOperator, Visitor,
    },
    astvalue::ASTValue,
    compile::CompileVisitor,
    context::Context,
    environment::Environment,
    errors::{ParseError, RuntimeError},
    idverifier::IdVerifier,
    module::Module,
    node::execution::Execution,
    node::{nodeop::NodeOp, program::Program},
    optimize::optimize,
    parser::Parser,
    renderbuffer::RenderBuffer,
    scanner::{Scanner, Token, TokenType},
    textures::{
        TexStorage,
        patterns::{PatternKind, ensure_patterns_initialized},
    },
};

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use theframework::theui::ThePalette;
use vek::Vec3;

pub struct Rusteria {
    path: PathBuf,
    pub context: Context,
    defaults: Option<Module>,
}

impl Default for Rusteria {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusteria {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            context: Context::new(FxHashMap::default()),
            defaults: None,
        }
    }

    /// Returns the default palette: https://lospec.com/palette-list/duel
    pub fn create_default_palette(&self) -> ThePalette {
        let mut palette = ThePalette::default();
        if let Some(bytes) = Embedded::get("duel.txt") {
            if let Ok(txt) = std::str::from_utf8(bytes.data.as_ref()) {
                palette.load_from_txt(txt.to_string());
            }
        }
        palette
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
        self.context.program.strings = module.strings.clone();

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
        optimize(&mut self.context.program.body);

        self.context.program.globals = self.context.globals.len();
        self.context.program.strings = module.strings.clone();

        Ok(())
    }

    /// Compile the voxels into the VoxelGrid.
    pub fn execute(&mut self, palette: &ThePalette) -> Option<Value> {
        let mut execution = Execution::new(self.context.globals.len());

        // Execute the main program to compile all voxels.
        execution.execute(&&self.context.program.body, &self.context.program, palette);

        execution.stack.pop()
    }

    pub fn execute_string(&mut self, str: &str, palette: &ThePalette) -> Option<Value> {
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

    pub fn shade(
        &self,
        buffer: &mut Arc<Mutex<RenderBuffer>>,
        function_index: usize,
        palette: &ThePalette,
    ) {
        let tile_size = (80, 80);

        let width = buffer.lock().unwrap().width;
        let height = buffer.lock().unwrap().height;

        let tiles = self.create_tiles(width, height, tile_size.0, tile_size.1);
        let screen_size = vek::Vec2::new(width as f32, height as f32);

        tiles.par_iter().for_each(|tile| {
            let mut tile_buffer = RenderBuffer::new(tile.width, tile.height);
            let mut execution = Execution::new(self.context.program.globals);

            for h in 0..tile.height {
                for w in 0..tile.width {
                    let x = tile.x + w;
                    let y = tile.y + h;

                    if x >= width || y >= height {
                        continue;
                    }

                    execution.uv = vek::Vec3::new(
                        x as f32 / screen_size.x,
                        1.0 - (y as f32 / screen_size.y),
                        0.0,
                    );

                    execution.color = Vec3::zero();
                    execution.shade(function_index, &self.context.program, palette);

                    tile_buffer.set(
                        w,
                        h,
                        [execution.color.x, execution.color.y, execution.color.z, 1.0],
                    );
                }
            }

            buffer
                .lock()
                .unwrap()
                .accum_from(tile.x, tile.y, &tile_buffer);
        });
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

    /// Create the tiles for the given image size.
    fn create_tiles(
        &self,
        image_width: usize,
        image_height: usize,
        tile_width: usize,
        tile_height: usize,
    ) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let mut x = 0;
        let mut y = 0;
        while x < image_width && y < image_height {
            let tile = Tile {
                x,
                y,
                width: tile_width,
                height: tile_height,
            };
            tiles.push(tile);
            x += tile_width;
            if x >= image_width {
                x = 0;
                y += tile_height;
            }
        }

        tiles
    }
}

#[derive(Debug, Clone, Copy)]
struct Tile {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let mut script = Rusteria::default();
        let result = script.execute_string("let a = 2; a + 2;".into(), &ThePalette::default());
        assert_eq!(result.unwrap().x, 4.0);
    }

    #[test]
    fn fib() {
        let mut script = Rusteria::default();
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
}
