#![windows_subsystem = "windows"]

use theframework::*;

pub mod editor;

use crate::editor::CodeEditor;

pub mod prelude {
    pub use theframework::prelude::*;

    pub use ::serde::{Deserialize, Serialize};
}

fn main() {
    // #[cfg(not(target_arch = "wasm32"))]
    // std::env::set_var("RUST_BACKTRACE", "1");

    let code: CodeEditor = CodeEditor::new();

    #[allow(unused_mut)]
    let mut app = TheApp::new();
    () = app.run(Box::new(code));
}
