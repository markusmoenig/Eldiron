use theframework::*;

pub mod browser;
pub mod editor;
pub mod misc;
pub mod project;
pub mod sidebar;
pub mod tileeditor;

pub mod widgets;

pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};
    pub use shared::prelude::*;
    pub use theframework::prelude::*;

    pub use crate::browser::*;
    pub use crate::misc::*;
    pub use crate::project::*;
    pub use crate::sidebar::*;
    pub use crate::tileeditor::*;

    pub use crate::widgets::therenderview::{TheRenderView, TheRenderViewTrait};
}

use crate::editor::Editor;

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "1");

    let editor = Editor::new();
    let mut app = TheApp::new();

    _ = app.run(Box::new(editor));
}
