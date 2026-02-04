use super::{
    ActionProperties, ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
    SurfaceAction,
};
use vek::Vec2;

/// A hole that cuts through the surface
pub struct HoleAction;

impl SurfaceAction for HoleAction {
    fn describe_mesh(
        &self,
        sector_uv: &[Vec2<f32>],
        surface_thickness: f32,
        _properties: &ActionProperties,
    ) -> Option<SectorMeshDescriptor> {
        if sector_uv.len() < 3 {
            return None;
        }

        // For a hole, we only need sides (tube), no caps
        let loop_front: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: 0.0, // starts at front surface
            })
            .collect();

        let loop_back: Vec<ControlPoint> = sector_uv
            .iter()
            .map(|&uv| ControlPoint {
                uv,
                extrusion: surface_thickness, // extends through to back
            })
            .collect();

        Some(SectorMeshDescriptor {
            is_hole: true,
            cap: None, // no cap for holes
            sides: Some(MeshTopology::QuadStrip {
                loop_a: loop_front,
                loop_b: loop_back,
            }),
            connection: ConnectionMode::Hard,
        })
    }

    fn name(&self) -> &'static str {
        "Hole"
    }
}
