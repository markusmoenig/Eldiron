//! Parameter-rich graph editing, independent of gameplay execution and the legacy node canvas.
//!
//! Hosts own the document and context. The editor emits reversible changes; the painter
//! consumes read-only observations. All geometry and input use the same viewport transform.
#[cfg(feature = "ui")]
mod controls;
#[cfg(feature = "ui")]
mod editor;
#[cfg(feature = "ui")]
mod paint;
#[cfg(feature = "ui")]
pub use controls::*;
#[cfg(feature = "ui")]
pub use editor::*;
#[cfg(feature = "ui")]
pub use paint::*;
#[cfg(feature = "ui")]
mod raster;
#[cfg(feature = "ui")]
pub use raster::*;
#[cfg(all(test, feature = "ui"))]
mod tests;

#[cfg(feature = "ui")]
mod text;
#[cfg(feature = "ui")]
pub use text::*;

#[cfg(feature = "ui")]
mod picker;
#[cfg(feature = "ui")]
pub use picker::*;

#[cfg(all(feature = "text", feature = "ui"))]
mod paint_list;
#[cfg(all(feature = "text", feature = "ui"))]
pub use paint_list::*;

pub use theframework_core::graph::*;

#[cfg(all(feature = "text", feature = "ui"))]
mod native_text;
#[cfg(all(feature = "text", feature = "ui"))]
pub use native_text::*;

#[cfg(all(feature = "text", feature = "ui"))]
mod view;
#[cfg(all(feature = "text", feature = "ui"))]
pub use view::*;
