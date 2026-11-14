//! Application-owned graphical presentation. No window surface or server clock.
mod scene_views;
pub use scene_views::*;

pub mod screen;

mod game_views;
pub use game_views::*;

mod game_screen;
pub use game_screen::{
    GameFontVersion, GameFonts, GameScreenController, HudControlId, HudPanelId, HudPart,
    ScreenActivation, ScreenCapture, ScreenEffect, ScreenFocus, ScreenInputResponse,
    ScreenTextResponse, ScreenTextTarget, SpeechOverlay, WorldTextOverlay,
};

pub mod isometric;

mod viewport;
pub use viewport::{ScreenViewport, player_binding_key};

pub mod overview2d;
pub mod orthographic_bake;

/// Project setup bridges for graphical owners and explicit reference hosts.
pub mod project_bridge;
