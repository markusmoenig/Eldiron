use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A recessed pocket into the surface
pub struct RecessAction;

impl SurfaceAction for RecessAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        surface_thickness: f32,
        properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 || properties.depth <= 0.0 {
            return None;
        }

        // Determine which side we're recessing into
        let base_extrusion = if properties.target_side == 1 {
            // Recessing into back side
            surface_thickness
        } else {
            // Recessing into front side (default)
            0.0
        };

        // Direction: inward (opposite of relief)
        let direction = if properties.target_side == 1 {
            -1.0 // back recesses toward front (along -normal)
        } else {
            1.0 // front recesses toward back (along +normal)
        };

        // Always build recess geometry (cap + jamb) regardless of depth
        // The recess creates a visible pocket that extends along the surface normal
        // Note: We no longer convert deep recesses to holes - they always render as pockets

        let recess_extrusion = base_extrusion + direction * properties.depth;

        // Cap at the recess depth (faces inward)
        let cap_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: recess_extrusion,
            })
            .collect();

        // Side walls forming the pocket
        let base_loop: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: base_extrusion,
            })
            .collect();

        let recess_loop: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: recess_extrusion,
            })
            .collect();

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: cap_points,
                holes: vec![],
            }),
            sides: Some(MeshTopology::QuadStrip {
                loop_a: base_loop,
                loop_b: recess_loop,
            }),
            connection: properties
                .connection_override
                .unwrap_or(ConnectionMode::Hard),
        })
    }

    fn name(&self) -> &'static str {
        "Recess"
    }
}
