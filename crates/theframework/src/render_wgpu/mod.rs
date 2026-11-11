//! GPU execution of ordered paint lists, with stencil clips and isolated opacity.
mod atlas;
mod mesh;
mod renderer;
pub use renderer::PaintRenderer;

#[cfg(feature = "ui")]
mod ui_images;
#[cfg(feature = "ui")]
pub use ui_images::UiImages;
