//! CPU-only shared data. This crate has no UI, window, font, or GPU dependencies.
pub mod image;
pub mod thecolor;
pub mod thedim;
pub mod thepalette;
pub mod thetime;
pub use image::{AlphaMode, ColorSpace, ImageAsset, ImageColorFilter, ImageError, ImageHandle};
pub use thecolor::TheColor;
pub use thedim::{TheDim, TheDimShrinker};
pub use thepalette::ThePalette;
pub use thetime::TheTime;

pub mod graph;
pub mod theid;
pub mod thetilemask;
pub use theid::TheId;
pub use thetilemask::TheTileMask;
pub use uuid::Uuid;

pub mod alignment;
mod codec;
pub mod key;
pub mod thergbabuffer;
pub mod thesurface;
pub mod thevalue;
pub use alignment::{TheHorizontalAlign, TheVerticalAlign};
pub use key::TheKeyCode;
pub use thergbabuffer::{TheRGBABuffer, TheRGBARegion, TheRGBARegionSequence, TheRGBATile};
pub use thesurface::{ThePixelRect, TheRasterTarget, TheSurfaceError, TheSurfaceMut};
pub use thevalue::{TheValue, TheValueAssignment, TheValueComparison};

pub mod thergbbuffer;
pub use thergbbuffer::TheRGBBuffer;
