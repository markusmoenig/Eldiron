//! Explicit GPU ownership and resources shared by UI and scene renderers.
//! No application event loop, widget tree, or gameplay state lives here.
mod context;
mod image;
mod readback;
mod surface;
pub use context::*;
pub use image::*;
pub use readback::*;
pub use surface::*;
pub use theframework_core::{AlphaMode, ColorSpace, ImageAsset, ImageHandle};
pub use wgpu;

#[derive(Debug, Clone)]
pub struct RenderError(pub String);
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}
pub type Result<T> = std::result::Result<T, RenderError>;
