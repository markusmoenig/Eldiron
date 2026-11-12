pub mod app;
pub mod misc;
pub mod player_session;

pub use app::EldironPlayerApp;

#[cfg(all(
    any(target_os = "macos", target_os = "ios"),
    not(feature = "native-player")
))]
mod apple_reference;
#[cfg(all(
    any(target_os = "macos", target_os = "ios"),
    not(feature = "native-player")
))]
pub use apple_reference::*;
#[cfg(all(any(target_os = "macos", target_os = "ios"), feature = "native-player"))]
mod apple;
#[cfg(all(any(target_os = "macos", target_os = "ios"), feature = "native-player"))]
pub use apple::*;

#[cfg(feature = "native-player")]
pub mod native_player;
