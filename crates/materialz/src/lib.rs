//! Shared procedural foundation for native editors and Rust applications.
pub mod bsdf;
pub mod cpu;
pub mod ffi;
pub mod format;
#[cfg(feature = "wgpu")]
pub mod gpu;
#[cfg(feature = "integration")]
pub mod integration;
pub mod ir;
mod json;
pub mod live;
pub mod metadata;
pub mod metal;
pub mod tilelang;
#[cfg(feature = "wgpu")]
pub mod wgsl;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub fn compile(source: &str) -> Result<ir::Program> {
    ir::Program::lower(&format::parse(source)?)
}

mod library;

mod material_mix;
mod math;
pub mod periodic;

pub mod scope;

pub mod execution;

mod fields;

pub mod image_source;
mod raster;

/// Experimental typed appearance rows. Independent of Eldiron and rendering.
#[cfg(feature = "integration")]
pub mod geometry;
