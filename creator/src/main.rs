use theframework::*;

pub mod editor;
pub mod sidebar;

pub mod prelude {
    pub use crate::sidebar::*;
    pub use theframework::prelude::*;
}

use crate::editor::Editor;

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "1");

    let editor = Editor::new();
    let mut app = TheApp::new();

    _ = app.run(Box::new(editor));
}
