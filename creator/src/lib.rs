#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
mod macros;

pub mod actionlist;
pub mod actions;
pub mod actionscript;
pub mod appearance_workspace;
pub mod avatar_atlas;
pub mod block_props;
pub mod blocks;
pub mod configeditor;
pub mod dockmanager;
pub mod docks;
pub mod editcamera;
pub mod editor;
#[cfg(feature = "native-render")]
pub mod native_application;
mod editor_overlay;
pub mod editor_display;
pub mod editor_tools;
mod geometry_editor;
pub mod hud;
#[cfg(not(target_arch = "wasm32"))]
pub mod i18n;
mod iso_editor;
mod tileset_creation;
mod item_appearance;
pub mod mapeditor;
pub mod material_editor;
pub mod minimap;
pub mod misc;
mod palette_switch;
mod procedural_material;
mod project_templates;
#[cfg(not(target_arch = "wasm32"))]
pub mod scepter;
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
pub mod self_update;
pub mod shortcuts;
pub mod sidebar;
pub mod textplay;
pub mod tile_collection;
pub mod toollist;
pub mod tools;
pub mod undo;
pub mod utils;
pub mod viewcube;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub const DEFAULT_VLAYOUT_RATIO: f32 = 0.70;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};

    pub use shared::prelude::*;
    pub use std::sync::{LazyLock, RwLock};
    pub use theframework::prelude::*;

    pub use crate::editor_display::*;
    pub use crate::mapeditor::*;
    pub use crate::misc::*;
    // pub use crate::previewview::*;
    pub use crate::actionlist::*;
    pub use crate::actionscript::*;
    pub use crate::sidebar::*;
    pub use crate::textplay::*;
    pub use crate::tile_collection::*;
    pub use crate::toollist::*;
    pub use crate::undo::project_atoms::*;
    pub use crate::undo::project_helper::*;
    pub use crate::undo::project_undo::*;
    pub use crate::undo::*;
    pub use crate::utils::*;

    pub use crate::tools::game::GameTool;
    pub use crate::tools::geometry::GeometryTool;
    pub use crate::tools::iso_paint::IsoPaintTool;
    pub use crate::tools::linedef::LinedefTool;
    pub use crate::tools::sector::SectorTool;
    pub use crate::tools::terrain::TerrainTool;
    // pub use crate::tools::tileset::TilesetTool;
    pub use crate::tools::vertex::VertexTool;

    pub use crate::docks::tiles::*;

    pub use crate::actions::*;
    pub use crate::docks::*;
    pub use crate::editor_tools::*;
    pub use crate::tools::*;

    pub use crate::configeditor::ConfigEditor;
    pub use crate::editcamera::{CustomMoveAction, EditCamera};

    pub use crate::dockmanager::{DockManager, DockManagerState};

    pub use toml::Table;

    pub const KEY_ESCAPE: u32 = 0;
    pub const KEY_RETURN: u32 = 1;
    pub const KEY_DELETE: u32 = 2;
    pub const KEY_HOME: u32 = 3;
    pub const KEY_END: u32 = 4;
    pub const KEY_UP: u32 = 5;
    pub const KEY_RIGHT: u32 = 6;
    pub const KEY_DOWN: u32 = 7;
    pub const KEY_LEFT: u32 = 8;
    pub const KEY_SPACE: u32 = 9;
    pub const KEY_TAB: u32 = 10;
}

// Xcode hosts use the same native GPU application as the Rust window host.
#[cfg(all(feature = "staticlib", any(target_os = "macos", target_os = "ios", target_os = "tvos")))]
mod apple;
