//! Parameter-rich graph editing, independent of gameplay execution and the legacy node canvas.
//!
//! Hosts own the document and context. The editor emits reversible changes; the painter
//! consumes read-only observations. All geometry and input use the same viewport transform.
mod controls;
mod editor;
mod model;
mod paint;
pub use controls::*;
pub use editor::*;
pub use model::*;
pub use paint::*;
mod raster;
pub use raster::*;
#[cfg(test)]
mod tests;

mod text;
pub use text::*;

mod definitions;
pub use definitions::*;
mod picker;
pub use picker::*;
