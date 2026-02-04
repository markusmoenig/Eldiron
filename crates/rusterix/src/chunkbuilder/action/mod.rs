mod billboard;
mod hole;
mod recess;
mod relief;

// Re-export action implementations
pub use billboard::BillboardAction;
pub use hole::HoleAction;
pub use recess::RecessAction;
pub use relief::ReliefAction;

use vek::Vec2;

/// Connection mode for how a surface action's mesh connects to the surrounding surface
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionMode {
    /// Hard edge connection (no blending)
    Hard,
    /// Smooth connection (blend normals)
    Smooth,
    /// Beveled edge connection
    Bevel { segments: u8, radius: f32 },
}

/// Describes how a control point should be generated
#[derive(Debug, Clone, Copy)]
pub struct ControlPoint {
    /// Position in surface UV space (x, z in local surface coordinates)
    pub uv: Vec2<f32>,
    /// Extrusion parameter (0.0 = on surface, positive = outward along normal, negative = inward)
    pub extrusion: f32,
}

/// Describes the topology of control points for a single edge or region
#[derive(Debug, Clone)]
pub enum MeshTopology {
    /// A simple loop of control points forming a closed boundary
    Loop(Vec<ControlPoint>),
    /// A filled region with outer boundary and optional holes
    FilledRegion {
        outer: Vec<ControlPoint>,
        holes: Vec<Vec<ControlPoint>>,
    },
    /// A quad strip connecting two loops
    QuadStrip {
        loop_a: Vec<ControlPoint>,
        loop_b: Vec<ControlPoint>,
    },
}

/// Describes what mesh geometry should be generated for a surface sector
#[derive(Debug, Clone)]
pub struct SectorMeshDescriptor {
    /// If true, this action cuts a hole through the base surface (no cap)
    pub is_hole: bool,
    /// The topology of the cap (if any)
    pub cap: Option<MeshTopology>,
    /// The topology of the sides/walls (if any)
    pub sides: Option<MeshTopology>,
    /// How the edges should connect to the surrounding surface
    pub connection: ConnectionMode,
}

/// Trait for surface actions that can be applied to sectors
///
/// Implementations describe WHAT mesh to generate (control points, topology)
/// rather than HOW to generate it. The actual meshing is done by a unified
/// algorithm that processes all surface actions consistently.
pub trait SurfaceAction: Send + Sync {
    /// Returns a descriptor of the mesh that should be generated for this sector.
    ///
    /// # Parameters
    /// - `sector_uv`: The UV boundary of the sector in surface-relative coordinates
    /// - `surface_thickness`: The thickness/extrusion depth of the surface (0 if flat)
    /// - `properties`: Action-specific properties (height, depth, etc.)
    ///
    /// # Returns
    /// A descriptor of the mesh geometry, or None if no mesh should be generated
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        surface_thickness: f32,
        properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor>;

    /// Returns a human-readable name for this action
    fn name(&self) -> &'static str;
}

/// Properties that can be passed to surface actions
#[derive(Debug, Clone, Default)]
pub struct ActionProperties {
    /// Height for relief/extrusion (positive = outward)
    pub height: f32,
    /// Depth for recess/inset (positive = inward)
    pub depth: f32,
    /// Which side of the surface to target (0 = front/default, 1 = back)
    pub target_side: i32,
    /// Width of slope for ridge actions (distance from edge to flat top)
    pub slope_width: f32,
    /// Custom connection mode override
    pub connection_override: Option<ConnectionMode>,
    /// Tile ID for billboard rendering
    pub tile_id: Option<uuid::Uuid>,
    /// Animation type for billboards
    pub animation: crate::map::surface::BillboardAnimation,
}

impl ActionProperties {
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_target_side(mut self, target_side: i32) -> Self {
        self.target_side = target_side;
        self
    }

    pub fn with_connection(mut self, connection: ConnectionMode) -> Self {
        self.connection_override = Some(connection);
        self
    }

    pub fn with_slope_width(mut self, slope_width: f32) -> Self {
        self.slope_width = slope_width;
        self
    }

    pub fn with_tile_id(mut self, tile_id: Option<uuid::Uuid>) -> Self {
        self.tile_id = tile_id;
        self
    }

    pub fn with_animation(mut self, animation: crate::map::surface::BillboardAnimation) -> Self {
        self.animation = animation;
        self
    }
}
