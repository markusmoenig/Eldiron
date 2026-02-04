//! UI module (feature `ui`): node-driven workspace and drawable emission.
//! This is a lightweight scaffold for a Procreate-like UI layer. It currently
//! defines node/view plumbing and drawable collection; rendering is expected
//! to use the existing 2D path.

mod drawable;
mod event;
pub mod layouts;
mod renderer;
mod style;
mod text;
mod theme;
mod undo;
mod widgets;
mod workspace;

// Project management (part of UI system)
#[cfg(not(target_os = "ios"))]
mod file_dialog;
mod project;
mod project_browser;

pub use drawable::{Drawable, UiColor, UiImage};
pub use event::{UiAction, UiEvent, UiEventKind, UiEventOutcome};
pub use layouts::{Alignment, HStack, Layoutable, VStack};
pub use renderer::UiRenderer;
pub use style::{StyleId, StyleParams, StyleRegistry};
pub use text::TextCache;
pub use theme::Theme;
pub use undo::{UndoCommand, UndoStack};
pub use widgets::{
    Button, ButtonGroup, ButtonGroupOrientation, ButtonGroupStyle, ButtonKind, ButtonStyle, Canvas,
    ColorButton, ColorButtonStyle, ColorWheel, DropdownList, DropdownListStyle, HAlign, Image,
    ImageStyle, Label, LabelRect, ParamList, ParamListEntry, ParamListStyle, PopupAlignment,
    ProjectBrowser, ProjectBrowserItem, ProjectBrowserStyle, Slider, SliderStyle, Spacer,
    TabbedPanel, TabbedPanelStyle, TextButton, Toolbar, ToolbarOrientation, ToolbarSeparator,
    ToolbarStyle, VAlign,
};
pub use workspace::{NodeId, UiView, ViewContext, Workspace};

// Project management
#[cfg(not(target_os = "ios"))]
pub use file_dialog::FileDialog;
pub use project::{Project, ProjectError, ProjectMetadata};
pub use project_browser::{RecentProject, RecentProjects};

/// Helper function to create empty material data for non-style tiles.
/// Use this when adding image tiles that don't need style rendering.
pub fn create_tile_material(width: u32, height: u32) -> Vec<u8> {
    vec![0u8; (width * height * 4) as usize]
}
