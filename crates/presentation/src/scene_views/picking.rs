//! Read-only CPU queries against a specific completed scene output. Callers must
//! supply the stamp actually displayed; a newer/mutated VM cannot answer for it.
use super::{SceneViews, ViewImage};
use scenevm::GeoId;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewPickHit {
    pub output: ViewImage,
    pub geo: GeoId,
    pub world: [f32; 3],
    pub distance: f32,
    pub paint_geo: Option<[u32; 4]>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewPick {
    /// Invalid input, zero-sized/lost/replaced view or a VM which no longer describes
    /// the displayed output. This is not a confirmed miss or a ground target.
    Unavailable,
    Miss(ViewImage),
    Hit(ViewPickHit),
}
impl SceneViews {
    pub fn pick_game(&self, displayed: ViewImage, uv: [f32; 2]) -> ViewPick {
        if uv.iter().any(|v| !v.is_finite() || *v < 0. || *v >= 1.) || self.context.is_lost() {
            return ViewPick::Unavailable;
        }
        let Some(view) = self.views.get(&displayed.view) else {
            return ViewPick::Unavailable;
        };
        if view.isometric
            || view.dirty
            || view.size.0 == 0
            || view.size.1 == 0
            || view.output != Some(displayed)
        {
            return ViewPick::Unavailable;
        }
        let vm = &view.scene.vm;
        let Some((geo, world, distance)) =
            vm.pick_geo_id_at_uv(view.size.0, view.size.1, uv, false, true)
        else {
            return ViewPick::Miss(displayed);
        };
        let paint_geo = if matches!(geo, GeoId::GeometryObject(_)) {
            // Paint picking currently queries static surfaces. A dynamic mesh
            // may be closer; never attach a static face from behind that hit.
            let tolerance = 0.0001_f32.max(16. * f32::EPSILON * distance.abs());
            vm.pick_paint_surface_at_uv(view.size.0, view.size.1, uv)
                .filter(|surface| {
                    surface.valid
                        && (surface.depth - distance).abs() <= tolerance
                        && surface
                            .world
                            .into_iter()
                            .zip([world.x, world.y, world.z])
                            .all(|(a, b)| (a - b).abs() <= tolerance)
                })
                .map(|surface| surface.paint_geo)
        } else {
            None
        };
        ViewPick::Hit(ViewPickHit {
            output: displayed,
            geo,
            world: [world.x, world.y, world.z],
            distance,
            paint_geo,
        })
    }
}
