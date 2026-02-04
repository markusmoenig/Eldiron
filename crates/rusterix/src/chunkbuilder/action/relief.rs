use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A raised relief above the surface
pub struct ReliefAction;

impl SurfaceAction for ReliefAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        surface_thickness: f32,
        properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 || properties.height <= 0.0 {
            return None;
        }

        // Determine which side we're attached to and direction
        let base_extrusion = if properties.target_side == 1 {
            // Back side
            surface_thickness
        } else {
            // Front side (default)
            0.0
        };

        // Direction: outward from the selected cap
        let direction = if properties.target_side == 1 {
            1.0 // back faces outward along +normal
        } else {
            -1.0 // front faces outward along -normal
        };

        let relief_extrusion = base_extrusion + direction * properties.height;

        // Cap at the relief height
        let cap_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: relief_extrusion,
            })
            .collect();

        // Side walls connecting base to relief
        let base_loop: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: base_extrusion,
            })
            .collect();

        let relief_loop: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: relief_extrusion,
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
                loop_b: relief_loop,
            }),
            connection: properties
                .connection_override
                .unwrap_or(ConnectionMode::Hard),
        })
    }

    fn name(&self) -> &'static str {
        "Relief"
    }
}
