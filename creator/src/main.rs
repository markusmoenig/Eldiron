#![windows_subsystem = "windows"]

use theframework::*;

pub mod brushes;
pub mod editor;
pub mod externals;
pub mod misc;
pub mod modelfxeditor;
pub mod panels;
pub mod regionfxeditor;
pub mod screeneditor;
pub mod self_update;
pub mod sidebar;
pub mod tileeditor;
pub mod tilefxeditor;
pub mod tilemapeditor;
pub mod tilepicker;
pub mod toollist;
pub mod tools;
pub mod undo;

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

    pub use crate::editor::ActiveEditor;

    pub use crate::externals::*;
    pub use crate::misc::*;
    pub use crate::modelfxeditor::*;
    pub use crate::panels::*;
    pub use crate::regionfxeditor::*;
    pub use crate::screeneditor::*;
    pub use crate::sidebar::*;
    pub use crate::tileeditor::*;
    pub use crate::tilefxeditor::*;
    pub use crate::tilemapeditor::*;
    pub use crate::tilepicker::*;
    pub use crate::toollist::*;
    pub use crate::undo::materialfx_undo::*;
    pub use crate::undo::region_undo::*;
    pub use crate::undo::*;

    pub use crate::tools::code::CodeTool;
    pub use crate::tools::draw::DrawTool;
    pub use crate::tools::eraser::EraserTool;
    pub use crate::tools::mapobjects::MapObjectsTool;
    pub use crate::tools::picker::PickerTool;
    pub use crate::tools::render::RenderTool;
    pub use crate::tools::screen::eraser::ScreenEraserTool;
    pub use crate::tools::screen::picker::ScreenPickerTool;
    pub use crate::tools::screen::tiledrawer::ScreenTileDrawerTool;
    pub use crate::tools::selection::SelectionTool;
    pub use crate::tools::tiledrawer::TileDrawerTool;
    pub use crate::tools::tilemap::TilemapTool;
    pub use crate::tools::zoom::ZoomTool;
    pub use crate::tools::*;

    pub use crate::brushes::disc::DiscBrush;
    pub use crate::brushes::rect::RectBrush;
    pub use crate::brushes::*;
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
