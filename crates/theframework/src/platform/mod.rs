//! Application-owned GPU presentation and platform-neutral input. No scene VM
//! or CPU framebuffer is owned by this layer.
mod ime;
mod input;
mod wake;
pub use wake::{ApplicationWake, WakeSender};
pub mod image_clipboard;
pub use image_clipboard::{ImageClipboardRequest, ImageClipboardResult};
pub use ime::ImeSession;
#[cfg(feature = "render-wgpu")]
mod session;
#[cfg(feature = "render-wgpu")]
mod foreign;
#[cfg(all(feature = "render-wgpu", feature = "ui"))]
mod ui;
#[cfg(feature = "platform-winit")]
pub mod winit;
pub use input::*;
#[cfg(feature = "render-wgpu")]
pub use session::*;
#[cfg(feature = "render-wgpu")]
pub use foreign::*;
#[cfg(all(feature = "render-wgpu", feature = "ui"))]
pub use ui::*;
#[cfg(feature = "text")]
pub mod native_text_wire;
