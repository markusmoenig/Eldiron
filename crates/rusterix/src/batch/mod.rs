pub mod batch2d;
pub mod batch3d;

/// The primitive mode. The rasterizer can draw triangles and lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveMode {
    /// Draw as triangles.
    Triangles,
    /// Draw connected vertices / points.
    Lines,
    /// Draw a line strip around the triangles.
    LineStrip,
    /// Draw a closed line strip around the triangles.
    LineLoop,
}

/// The CullMode of the batch, Off by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CullMode {
    /// Render all faces
    Off,
    /// Cull front-facing triangles
    Front,
    /// Cull back-facing triangles
    Back,
}

/// The source of the geometry
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometrySource {
    Unknown,
    Vertex(u32),
    Linedef(u32),
    Sector(u32),
    Entity(u32),
    Item(u32),
}
