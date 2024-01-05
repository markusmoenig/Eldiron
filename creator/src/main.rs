use theframework::*;

pub mod browser;
pub mod editor;
pub mod misc;
pub mod panels;
pub mod sidebar;
pub mod tileeditor;
pub mod tilemapeditor;
pub mod tilepicker;

pub mod widgets;

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

    pub use crate::browser::*;
    pub use crate::misc::*;
    pub use crate::panels::*;
    pub use crate::sidebar::*;
    pub use crate::tileeditor::*;
    pub use crate::tilemapeditor::*;
    pub use crate::tilepicker::*;

    pub use crate::widgets::therenderview::{TheRenderView, TheRenderViewTrait};
}

use crate::editor::Editor;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let editor = Editor::new();
    let mut app = TheApp::new();

    _ = app.run(Box::new(editor));
}
