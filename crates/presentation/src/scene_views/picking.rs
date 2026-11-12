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

/// Authoring queries preserve the separate base, guide and gizmo policies. A
/// confirmed miss still carries its camera ray; unavailable output carries none.
#[derive(Clone, Copy, Debug)]
pub struct EditorPick {
    pub camera: scenevm::Camera3D,
    pub dynamic: Option<(GeoId, scenevm::prelude::Vec3<f32>, f32)>,
    pub paint_geo: Option<[u32; 4]>,
    pub ray: Option<(scenevm::prelude::Vec3<f32>, scenevm::prelude::Vec3<f32>)>,
    pub base: Option<(
        GeoId,
        scenevm::prelude::Vec3<f32>,
        f32,
        scenevm::prelude::Vec3<f32>,
    )>,
    pub guide: Option<(GeoId, scenevm::prelude::Vec3<f32>, f32)>,
    pub gizmo: Option<(GeoId, scenevm::prelude::Vec3<f32>, f32)>,
}
impl SceneViews {
    /// Read the exact completed output without invalidating it. Layer selection
    /// is restored; this API cannot update scene content or acknowledge input.
    pub fn pick_editor(
        &mut self,
        displayed: ViewImage,
        uv: [f32; 2],
        gizmos: bool,
        guides: bool,
    ) -> Option<EditorPick> {
        if uv.iter().any(|v| !v.is_finite()) || self.context.is_lost() {
            return None;
        }
        let view = self.views.get_mut(&displayed.view)?;
        if view.isometric
            || view.dirty
            || view.size.0 == 0
            || view.size.1 == 0
            || view.output != Some(displayed)
        {
            return None;
        }
        let vm = &mut view.scene.vm;
        let active = vm.active_vm_index();
        vm.set_active_vm(0);
        let camera = vm.active_vm().camera3d;
        let dynamic = vm.pick_geo_id_at_uv(view.size.0, view.size.1, uv, false, true);
        let paint_geo = vm
            .pick_paint_surface_at_uv(view.size.0, view.size.1, uv)
            .filter(|surface| surface.valid)
            .map(|surface| surface.paint_geo);
        let ray = vm.ray_from_uv_with_size(view.size.0, view.size.1, uv);
        let base = vm.pick_geo_id_normal_at_uv(view.size.0, view.size.1, uv, false, false);
        let (gizmo, guide) = if vm.vm_layer_count() > 2 && (gizmos || guides) {
            vm.set_active_vm(2);
            let hit = vm.pick_geo_id_at_uv(view.size.0, view.size.1, uv, false, false);
            (
                hit.filter(|(id, _, _)| gizmos && matches!(id, GeoId::Gizmo(_))),
                hit.filter(|_| guides),
            )
        } else {
            (None, None)
        };
        vm.set_active_vm(active);
        Some(EditorPick {
            camera,
            dynamic,
            paint_geo,
            ray,
            base,
            guide,
            gizmo,
        })
    }
    /// Rectangle selection reads the same displayed guide layer as pointer
    /// selection. It never rebuilds geometry or changes output revisions.
    pub fn pick_editor_rectangle(
        &mut self,
        displayed: ViewImage,
        top: scenevm::prelude::Vec2<f32>,
        bottom: scenevm::prelude::Vec2<f32>,
        kind: GeoId,
    ) -> Option<Vec<GeoId>> {
        if self.context.is_lost()
            || [top.x, top.y, bottom.x, bottom.y]
                .iter()
                .any(|v| !v.is_finite())
        {
            return None;
        }
        let view = self.views.get_mut(&displayed.view)?;
        if view.dirty
            || view.isometric
            || view.output != Some(displayed)
            || view.scene.vm.vm_layer_count() <= 2
        {
            return None;
        }
        let active = view.scene.vm.active_vm_index();
        view.scene.vm.set_active_vm(2);
        let hits = view.scene.vm.active_vm().pick_geo_ids_in_rect(
            view.size.0,
            view.size.1,
            top,
            bottom,
            kind,
            false,
            false,
        );
        view.scene.vm.set_active_vm(active);
        Some(hits)
    }
}
