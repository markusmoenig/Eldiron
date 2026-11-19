//! CPU-only shared data. This crate has no UI, window, font, or GPU dependencies.
pub mod image;
mod shared_bytes;
mod shared_text;
pub use shared_text::SharedText;
pub use shared_bytes::{BytePlanes, SharedBytes};
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

/// Plain CPU data and math shared by project, rules, and session consumers.
/// Presentation services belong to the Framework prelude instead.
pub mod prelude {
    pub use crate::{
        AlphaMode, ColorSpace, ImageAsset, ImageColorFilter, ImageError, ImageHandle, SharedBytes,
        TheColor, TheDim, TheDimShrinker, TheHorizontalAlign, TheId, TheKeyCode, ThePalette, ThePixelRect,
        TheRGBABuffer, TheRGBARegion, TheRGBARegionSequence, TheRGBATile, TheRGBBuffer,
        TheRasterTarget, TheSurfaceError, TheSurfaceMut, TheTileMask, TheTime, TheValue,
        TheValueAssignment, TheValueComparison, TheVerticalAlign,
    };
    pub use rustc_hash::{FxHashMap, FxHashSet};
    pub use serde::{Deserialize, Serialize};
    pub use uuid::Uuid;
    pub use vek::*;
}
