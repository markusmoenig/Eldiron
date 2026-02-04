use crate::{Map, Sector};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::{Vec2, Vec3};

use earcutr::earcut;

/// Animation type for billboards
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub enum BillboardAnimation {
    #[default]
    None,
    OpenUp,    // Gate opens upward
    OpenRight, // Gate opens to the right
    OpenDown,  // Gate opens downward
    OpenLeft,  // Gate opens to the left
    Fade,      // Gate fades in/out
}

/// Operation applied to a profile loop on this surface (non-destructive).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LoopOp {
    None,
    Relief {
        height: f32,
    }, // positive outward along surface normal
    Recess {
        depth: f32,
    }, // positive inward along surface normal
    Billboard {
        tile_id: Option<Uuid>,         // Tile UUID to render on billboard
        animation: BillboardAnimation, // Animation type
        inset: f32,                    // Offset from surface (positive = along normal)
    },
}

impl LoopOp {
    /// Convert this LoopOp to ActionProperties for use with the SurfaceAction trait system
    pub fn to_action_properties(
        &self,
        target_side: i32,
    ) -> crate::chunkbuilder::action::ActionProperties {
        use crate::chunkbuilder::action::ActionProperties;

        match self {
            LoopOp::None => ActionProperties::default().with_target_side(target_side),
            LoopOp::Relief { height } => ActionProperties::default()
                .with_height(*height)
                .with_target_side(target_side),
            LoopOp::Recess { depth } => ActionProperties::default()
                .with_depth(*depth)
                .with_target_side(target_side),
            LoopOp::Billboard {
                tile_id,
                animation,
                inset,
            } => ActionProperties::default()
                .with_depth(*inset)
                .with_target_side(target_side)
                .with_tile_id(*tile_id)
                .with_animation(*animation),
        }
    }

    /// Get the appropriate SurfaceAction implementation for this operation
    pub fn get_action(&self) -> Option<Box<dyn crate::chunkbuilder::action::SurfaceAction>> {
        use crate::chunkbuilder::action::{
            BillboardAction, HoleAction, RecessAction, ReliefAction,
        };

        match self {
            LoopOp::None => Some(Box::new(HoleAction)),
            LoopOp::Relief { .. } => Some(Box::new(ReliefAction)),
            LoopOp::Recess { .. } => Some(Box::new(RecessAction)),
            LoopOp::Billboard { .. } => Some(Box::new(BillboardAction)),
        }
    }
}

/// One closed loop in the surface's UV/profile space.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileLoop {
    pub path: Vec<Vec2<f32>>, // points in UV space, assumed to be simple polygon
    pub op: LoopOp,           // optional loop-specific op
    /// The profile-map sector this loop came from. `None` for the outer host loop.
    pub origin_profile_sector: Option<u32>,
}

/// Represents a geometric plane defined by an origin and a normal vector.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Plane {
    pub origin: Vec3<f32>,
    pub normal: Vec3<f32>,
}

/// Represents a 3D basis with right, up, and normal vectors.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Basis3 {
    pub right: Vec3<f32>,
    pub up: Vec3<f32>,
    pub normal: Vec3<f32>,
}

/// Defines an editable plane with origin, axes for 2D editing, and a scale factor.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct EditPlane {
    pub origin: Vec3<f32>,
    pub right: Vec3<f32>,
    pub up: Vec3<f32>,
    pub scale: f32,
}

/// Represents an attachment with a transform relative to a surface and optional mesh or procedural references.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Attachment {
    pub id: Uuid,
    pub surface_id: Uuid,
    pub transform: [[f32; 4]; 4],
    pub mesh_ref: Option<Uuid>,
    pub proc_ref: Option<Uuid>,
}

/// UV mapping strategy for extruded side walls and caps.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ExtrudeUV {
    /// U follows edge length; V follows depth. Scales apply as multipliers.
    Stretch { scale_u: f32, scale_v: f32 },
    /// Planar UV for caps (using surface UV), stretch for sides with uniform scale.
    PlanarFront { scale: f32 },
}

impl Default for ExtrudeUV {
    fn default() -> Self {
        Self::Stretch {
            scale_u: 1.0,
            scale_v: 1.0,
        }
    }
}

/// How this surface turns into 3D geometry.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ExtrusionSpec {
    pub enabled: bool,     // if false, flat cap only
    pub depth: f32,        // thickness along +N (negative = -N)
    pub cap_front: bool,   // cap at origin plane
    pub cap_back: bool,    // cap at origin + depth
    pub flip_normal: bool, // invert N at build-time if needed
    pub uv: ExtrudeUV,     // UV mapping mode for sides/caps
}

impl Default for ExtrusionSpec {
    fn default() -> Self {
        Self {
            enabled: false,
            depth: 0.0,
            cap_front: true,
            cap_back: false,
            flip_normal: false,
            uv: ExtrudeUV::default(),
        }
    }
}

/// Represents a surface with the sector owner, geometry, and profile.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Surface {
    pub id: Uuid,
    pub sector_id: u32,

    /// Geometric frame of the editable plane for this surface
    pub plane: Plane,
    pub frame: Basis3,
    pub edit_uv: EditPlane,

    /// Extrusion parameters for this surface (depth, caps, UVs).
    #[serde(default)]
    pub extrusion: ExtrusionSpec,

    /// Uuid of the Profile
    pub profile: Option<Uuid>,

    /// Optional, the vertices of the surface in world coordinates, used in cases where we need to pass standalone surfaces.
    #[serde(skip)]
    pub world_vertices: Vec<Vec3<f32>>,
}

impl Surface {
    pub fn new(sector_id: u32) -> Surface {
        Surface {
            id: Uuid::new_v4(),
            sector_id,
            plane: Plane::default(),
            frame: Basis3::default(),
            edit_uv: EditPlane::default(),
            extrusion: ExtrusionSpec::default(),
            profile: None,
            world_vertices: vec![],
        }
    }

    /// Returns true if the surface has valid (finite) transform values
    pub fn is_valid(&self) -> bool {
        self.plane.origin.x.is_finite()
            && self.plane.origin.y.is_finite()
            && self.plane.origin.z.is_finite()
            && self.plane.normal.x.is_finite()
            && self.plane.normal.y.is_finite()
            && self.plane.normal.z.is_finite()
            && self.frame.right.x.is_finite()
            && self.frame.right.y.is_finite()
            && self.frame.right.z.is_finite()
            && self.frame.up.x.is_finite()
            && self.frame.up.y.is_finite()
            && self.frame.up.z.is_finite()
            && self.frame.normal.x.is_finite()
            && self.frame.normal.y.is_finite()
            && self.frame.normal.z.is_finite()
    }

    /// Calculate the geometry
    pub fn calculate_geometry(&mut self, map: &Map) {
        if let Some(sector) = map.find_sector(self.sector_id) {
            if let Some(points) = sector.vertices_world(map) {
                // existing logic using `points`
                let (centroid, mut normal) = newell_plane(&points);
                if normal.magnitude() < 1e-6 {
                    normal = Vec3::new(0.0, 1.0, 0.0);
                }
                let mut right = stable_right(&points, normal);
                let mut up = normalize_or_zero(normal.cross(right));

                if up.magnitude() < 1e-6 {
                    // fallback: try swapping axes
                    right = normalize_or_zero(normal.cross(Vec3::new(0.0, 1.0, 0.0)));
                    up = normalize_or_zero(normal.cross(right));
                }

                if up.magnitude() < 1e-6 {
                    // final fallback
                    right = Vec3::new(1.0, 0.0, 0.0);
                    up = normalize_or_zero(normal.cross(right));
                }

                // ensure orthonormal basis (flip right if needed)
                let test_up = normalize_or_zero(normal.cross(right));
                if test_up.magnitude() > 1e-6 && (test_up - up).magnitude() > 1e-6 {
                    right = -right;
                    up = normalize_or_zero(normal.cross(right));
                }

                self.plane.origin = centroid;
                self.plane.normal = normal;

                self.frame.right = right;
                self.frame.up = up;
                self.frame.normal = self.plane.normal;

                self.edit_uv.origin = self.plane.origin;
                self.edit_uv.right = self.frame.right;
                self.edit_uv.up = self.frame.up;
                self.edit_uv.scale = 1.0;
                return;
            } else {
                self.plane = Default::default();
                self.frame = Default::default();
                self.edit_uv = Default::default();
                return;
            }
        }
        self.plane = Default::default();
        self.frame = Default::default();
        self.edit_uv = Default::default();
    }

    /// Map a UV point on the surface plane to world space (w = 0 plane).
    pub fn uv_to_world(&self, uv: Vec2<f32>) -> Vec3<f32> {
        self.edit_uv.origin
            + self.edit_uv.right * uv.x * self.edit_uv.scale
            + self.edit_uv.up * uv.y * self.edit_uv.scale
    }

    /// Map a UVW point (UV on the surface, W along the surface normal) to world space.
    pub fn uvw_to_world(&self, uv: Vec2<f32>, w: f32) -> Vec3<f32> {
        self.uv_to_world(uv) + self.frame.normal * w
    }

    pub fn world_to_uv(&self, p: Vec3<f32>) -> Vec2<f32> {
        let rel = p - self.edit_uv.origin;
        Vec2::new(rel.dot(self.edit_uv.right), rel.dot(self.edit_uv.up)) / self.edit_uv.scale
    }

    /// Map a world point to discrete tile coordinates (1x1 grid cells in UV space).
    /// Returns (tile_x, tile_y) representing which tile cell the point falls into.
    /// This is useful for tile override systems that assign different tiles to different regions.
    pub fn world_to_tile(&self, p: Vec3<f32>) -> (i32, i32) {
        let uv = self.world_to_uv(p);
        (uv.x.floor() as i32, uv.y.floor() as i32)
    }

    /// Get the four world-space corners of a 1x1 tile cell at the given tile coordinates.
    /// Corners are ordered around the cell starting at (tile_x, tile_y) and proceeding CCW.
    pub fn tile_outline_world(&self, tile: (i32, i32)) -> [Vec3<f32>; 4] {
        let (tx, ty) = tile;
        let corners_uv = [
            Vec2::new(tx as f32, ty as f32),
            Vec2::new(tx as f32 + 1.0, ty as f32),
            Vec2::new(tx as f32 + 1.0, ty as f32 + 1.0),
            Vec2::new(tx as f32, ty as f32 + 1.0),
        ];
        corners_uv.map(|uv| self.uv_to_world(uv))
    }

    /// Project the owning sector polygon into this surface's UV space (CCW ensured).
    pub fn sector_loop_uv(&self, map: &Map) -> Option<Vec<Vec2<f32>>> {
        let sector = map.find_sector(self.sector_id)?;
        let pts3 = sector.vertices_world(map)?;
        if pts3.len() < 3 {
            return None;
        }
        let mut uv: Vec<Vec2<f32>> = pts3.iter().map(|p| self.world_to_uv(*p)).collect();
        if polygon_signed_area_uv(&uv) < 0.0 {
            uv.reverse();
        }
        Some(uv)
    }

    /// Triangulate a cap defined by an outer loop and optional hole loops in UV space.
    /// Returns (world_positions, triangle_indices, uv_positions).
    pub fn triangulate_cap_with_holes(
        &self,
        outer_uv: &[Vec2<f32>],
        holes_uv: &[Vec<Vec2<f32>>],
    ) -> Option<(Vec<[f32; 4]>, Vec<(usize, usize, usize)>, Vec<[f32; 2]>)> {
        if outer_uv.len() < 3 {
            return None;
        }
        // Build flattened buffer: outer first, then each hole
        let mut verts: Vec<Vec2<f32>> =
            Vec::with_capacity(outer_uv.len() + holes_uv.iter().map(|h| h.len()).sum::<usize>());
        let mut holes_idx: Vec<usize> = Vec::with_capacity(holes_uv.len());

        // Outer (ensure CCW)
        if polygon_signed_area_uv(outer_uv) < 0.0 {
            let mut ccw = outer_uv.to_vec();
            ccw.reverse();
            verts.extend(ccw);
        } else {
            verts.extend_from_slice(outer_uv);
        }
        // Holes (ensure CW per earcut convention)
        let mut offset = outer_uv.len();
        for h in holes_uv {
            holes_idx.push(offset);
            if polygon_signed_area_uv(h) > 0.0 {
                // if CCW, flip to CW
                let mut cw = h.clone();
                cw.reverse();
                verts.extend(cw);
            } else {
                verts.extend_from_slice(h);
            }
            offset += h.len();
        }

        // Flatten to f64 for earcut
        let flat: Vec<f64> = verts
            .iter()
            .flat_map(|v| [v.x as f64, v.y as f64])
            .collect();
        let idx = earcut(&flat, &holes_idx, 2).ok()?;
        let indices: Vec<(usize, usize, usize)> =
            idx.chunks_exact(3).map(|c| (c[2], c[1], c[0])).collect();

        let verts_uv: Vec<[f32; 2]> = verts.iter().map(|v| [v.x, v.y]).collect();
        let world_vertices: Vec<[f32; 4]> = verts
            .iter()
            .map(|uv| {
                let p = self.uv_to_world(*uv);
                [p.x, p.y, p.z, 1.0]
            })
            .collect();

        Some((world_vertices, indices, verts_uv))
    }

    /// Normalized surface normal.
    pub fn normal(&self) -> Vec3<f32> {
        let n = self.plane.normal;
        let m = n.magnitude();
        if m > 1e-6 {
            n / m
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        }
    }

    /// Triangulate the owning sector in this surface's local UV space and return world vertices, indices, and UVs.
    /// This treats the sector's 3D polygon as the base face of the surface; any vertical/tilted walls are handled correctly.
    pub fn triangulate(
        &self,
        sector: &Sector,
        map: &Map,
    ) -> Option<(Vec<[f32; 4]>, Vec<(usize, usize, usize)>, Vec<[f32; 2]>)> {
        // 1) Get ordered 3D polygon for the sector
        let points3 = sector.vertices_world(map)?;
        if points3.len() < 3 {
            return None;
        }

        // 2) Project to this surface's local UV space
        let verts_uv: Vec<[f32; 2]> = points3
            .iter()
            .map(|p| {
                let uv = self.world_to_uv(*p);
                [uv.x, uv.y]
            })
            .collect();

        // 3) Triangulate in 2D (UV) using earcut (no holes for now)
        let flattened: Vec<f64> = verts_uv
            .iter()
            .flat_map(|v| [v[0] as f64, v[1] as f64])
            .collect();
        let holes: Vec<usize> = Vec::new();
        let idx = earcut(&flattened, &holes, 2).ok()?; // Vec<usize>

        // Convert to triangle triplets, flipping winding to match your renderer if needed
        let indices: Vec<(usize, usize, usize)> =
            idx.chunks_exact(3).map(|c| (c[2], c[1], c[0])).collect();

        // 4) Map UV back to world using this surface's frame
        let world_vertices: Vec<[f32; 4]> = verts_uv
            .iter()
            .map(|v| {
                let p = self.uv_to_world(vek::Vec2::new(v[0], v[1]));
                [p.x, p.y, p.z, 1.0]
            })
            .collect();

        Some((world_vertices, indices, verts_uv))
    }
}

fn normalize_or_zero(v: Vec3<f32>) -> Vec3<f32> {
    let m = v.magnitude();
    if m > 1e-6 { v / m } else { Vec3::zero() }
}

fn newell_plane(points: &[Vec3<f32>]) -> (Vec3<f32>, Vec3<f32>) {
    let mut centroid = Vec3::zero();
    let mut normal = Vec3::zero();
    let n = points.len();
    for i in 0..n {
        let current = points[i];
        let next = points[(i + 1) % n];
        centroid += current;
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    centroid /= n as f32;
    let m = normal.magnitude();
    if m > 1e-6 {
        normal /= m;
    } else {
        normal = Vec3::zero();
    }
    (centroid, normal)
}

fn stable_right(points: &[Vec3<f32>], normal: Vec3<f32>) -> Vec3<f32> {
    let n = points.len();
    let mut max_len = 0.0;
    let mut right = Vec3::zero();
    for i in 0..n {
        let edge = points[(i + 1) % n] - points[i];
        let proj = edge - normal * normal.dot(edge);
        let len = proj.magnitude();
        if len > max_len {
            max_len = len;
            right = proj;
        }
    }
    if max_len < 1e-6 {
        // fallback: pick any axis orthogonal to normal
        if normal.x.abs() < normal.y.abs() && normal.x.abs() < normal.z.abs() {
            right = Vec3::new(0.0, -normal.z, normal.y);
        } else if normal.y.abs() < normal.z.abs() {
            right = Vec3::new(-normal.z, 0.0, normal.x);
        } else {
            right = Vec3::new(-normal.y, normal.x, 0.0);
        }
    }
    normalize_or_zero(right)
}

fn polygon_signed_area_uv(poly: &[Vec2<f32>]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut a = 0.0f32;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += p.x * q.y - q.x * p.y;
    }
    0.5 * a
}
