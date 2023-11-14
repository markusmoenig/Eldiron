use theframework::*;

pub mod editor;
pub mod sidebar;
pub mod project;
pub mod tilemap;
pub mod browser;

pub mod prelude {
    pub use theframework::prelude::*;
    pub use ::serde::{Serialize, Deserialize};

    pub use crate::sidebar::*;
    pub use crate::tilemap::*;
    pub use crate::project::*;
    pub use  crate::browser::*;
}

use crate::editor::Editor;

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "1");

    let editor = Editor::new();
    let mut app = TheApp::new();

    _ = app.run(Box::new(editor));
}
