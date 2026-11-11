//! CPU-only shared data. This crate has no UI, window, font, or GPU dependencies.
pub mod image;
pub mod thecolor;
pub mod thedim;
pub mod thepalette;
pub mod thetime;
pub use image::{AlphaMode, ColorSpace, ImageAsset, ImageError, ImageHandle};
pub use thecolor::TheColor;
pub use thedim::{TheDim, TheDimShrinker};
pub use thepalette::ThePalette;
pub use thetime::TheTime;
