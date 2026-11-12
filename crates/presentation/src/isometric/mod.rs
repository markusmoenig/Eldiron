//! Native isometric presentation owners; borrowed CPU plans stay in Rusterix.
mod sources;
pub use sources::{IsometricSourceVersion, IsometricSources};

mod material_sources;
pub use material_sources::{GpuMaterialSource, IsometricMaterialSources};

mod surface_renderer;
pub use surface_renderer::{IsometricSurfaceRenderer, SurfaceKind, SurfaceTriangle};
