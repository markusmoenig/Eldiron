//! GPU execution of ordered paint lists, with stencil clips and isolated opacity.
mod atlas;
mod bindings;
pub use bindings::BindingCacheUsage;
mod mesh;
mod sampled_lines;
mod renderer;
pub use renderer::{FrameStats, PaintRenderer};

#[cfg(feature = "ui")]
mod ui_images;
#[cfg(feature = "ui")]
pub use ui_images::{UiFrameImages, UiImageCache, UiImageCacheStats, UiImages};
