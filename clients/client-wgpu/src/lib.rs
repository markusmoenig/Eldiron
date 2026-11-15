// The old SceneVM runner/CPU overlay is an explicit reference API. Bare
// library builds retain the historical Apple facade; canonical hosts do not
// compile or expose this adapter unless legacy-reference is selected.
#[cfg(any(feature = "legacy-reference", not(feature = "native-player")))]
pub mod app;
pub mod misc;
pub mod player_session;

#[cfg(any(feature = "legacy-reference", not(feature = "native-player")))]
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
