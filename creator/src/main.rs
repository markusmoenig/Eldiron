#![windows_subsystem = "windows"]

use theframework::*;

pub mod editor;
pub mod externals;
pub mod misc;
pub mod panels;
pub mod regionmodeler;
pub mod regionrender;
pub mod screeneditor;
pub mod sidebar;
pub mod tileeditor;
pub mod tilefxeditor;
pub mod tilemapeditor;
pub mod tilepicker;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub mod prelude {

    pub use ::serde::{Deserialize, Serialize};
    pub use shared::prelude::*;
    pub use theframework::prelude::*;

    pub use crate::externals::*;
    pub use crate::misc::*;
    pub use crate::panels::*;
    pub use crate::regionmodeler::*;
    pub use crate::regionrender::*;
    pub use crate::screeneditor::*;
    pub use crate::sidebar::*;
    pub use crate::tileeditor::*;
    pub use crate::tilefxeditor::*;
    pub use crate::tilemapeditor::*;
    pub use crate::tilepicker::*;
}

use crate::editor::Editor;

fn main() {
    let args: Vec<_> = std::env::args().collect();

    std::env::set_var("RUST_BACKTRACE", "1");

    let editor = Editor::new();
    let mut app = TheApp::new();
    app.set_cmd_line_args(args);

    _ = app.run(Box::new(editor));
}
