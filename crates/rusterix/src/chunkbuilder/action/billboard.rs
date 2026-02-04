use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A billboard that faces into the surface normal (for doors/gates)
pub struct BillboardAction;

impl SurfaceAction for BillboardAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        _surface_thickness: f32,
        properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 {
            return None;
        }

        // Billboard is positioned at the inset depth along the surface normal
        // We create a single quad that faces INTO the surface (along -normal direction)
        // This means the billboard is visible when looking at the hole from the front

        let inset = properties.depth; // Use depth as inset offset

        // Create a filled region at the inset depth
        // This will be a flat quad facing the viewer
        let billboard_points: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: inset, // Position along surface normal
            })
            .collect();

        Some(SectorMeshDescriptor {
            is_hole: false,
            cap: Some(MeshTopology::FilledRegion {
                outer: billboard_points,
                holes: vec![],
            }),
            sides: None, // No sides - just a flat billboard
            connection: ConnectionMode::Hard,
        })
    }

    fn name(&self) -> &'static str {
        "Billboard"
    }
}
