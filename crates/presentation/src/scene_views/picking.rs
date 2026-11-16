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
/// Base-layer surface query for tools with an explicit geometry exclusion policy.
/// The ray and camera belong to the acknowledged output, including on a miss.
#[derive(Clone, Copy, Debug)]
pub struct EditorSurfacePick {
    pub camera: scenevm::Camera3D,
    pub ray: Option<(scenevm::prelude::Vec3<f32>, scenevm::prelude::Vec3<f32>)>,
    pub base: Option<(
        GeoId,
        scenevm::prelude::Vec3<f32>,
        f32,
        scenevm::prelude::Vec3<f32>,
    )>,
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
        let view = self.editor_query_view(displayed, uv)?;
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
    /// Query the displayed base layer, rejecting transient geometry before the
    /// nearest-hit reduction. This performs no readback or scene mutation.
    pub fn pick_editor_surface_filtered(
        &mut self,
        displayed: ViewImage,
        uv: [f32; 2],
        accept: impl Fn(GeoId) -> bool + Sync,
    ) -> Option<EditorSurfacePick> {
        if uv.iter().any(|v| *v < 0. || *v >= 1.) {
            return None;
        }
        let view = self.editor_query_view(displayed, uv)?;
        let vm = &mut view.scene.vm;
        let active = vm.active_vm_index();
        vm.set_active_vm(0);
        let camera = vm.active_vm().camera3d;
        let ray = vm.ray_from_uv_with_size(view.size.0, view.size.1, uv);
        let base = vm.pick_geo_id_normal_at_uv_filtered(
            view.size.0,
            view.size.1,
            uv,
            false,
            false,
            accept,
        );
        vm.set_active_vm(active);
        Some(EditorSurfacePick { camera, ray, base })
    }
    fn editor_query_view(
        &mut self,
        displayed: ViewImage,
        uv: [f32; 2],
    ) -> Option<&mut super::SceneView> {
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
        Some(view)
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

/// Dedicated authoring coordinates from one completed view. A confirmed miss is
/// distinct from unavailable output; the material UV is never substituted.
#[derive(Clone, Copy, Debug)]
pub struct EditorPaintPick {
    pub output: ViewImage,
    pub camera: scenevm::Camera3D,
    pub ray: Option<(scenevm::prelude::Vec3<f32>, scenevm::prelude::Vec3<f32>)>,
    pub surface: Option<scenevm::PaintSurfacePixel>,
    pub brush_transform: Option<[f32; 4]>,
}
impl EditorPaintPick {
    /// Paint targets the static surface already queried here. Do not perform
    /// additional dynamic/base/guide ray scans just to populate unused hits.
    pub fn editor_pick(&self) -> EditorPick {
        EditorPick {
            camera: self.camera,
            ray: self.ray,
            base: self.surface.map(|surface| (
                surface.geo_id,
                scenevm::prelude::Vec3::from(surface.world),
                surface.depth,
                scenevm::prelude::Vec3::from(surface.normal),
            )),
            paint_geo: self.surface.map(|surface| surface.paint_geo),
            dynamic: None,
            guide: None,
            gizmo: None,
        }
    }
}
/// Explicit CPU authoring data. Camera and output identify exactly the snapshot
/// used by selection/projected strokes, independently of subsequent preparation.
pub struct EditorPaintCapture {
    pub output: ViewImage,
    pub camera: scenevm::Camera3D,
    pub buffer: std::sync::Arc<scenevm::PaintSurfaceBuffer>,
}
impl SceneViews {
    pub fn pick_editor_paint(
        &mut self,
        displayed: ViewImage,
        uv: [f32; 2],
    ) -> Option<EditorPaintPick> {
        if uv.iter().any(|v| !v.is_finite() || *v < 0. || *v >= 1.) || self.context.is_lost() {
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
        let ray = vm.ray_from_uv_with_size(view.size.0, view.size.1, uv);
        let surface = vm
            .pick_paint_surface_at_uv(view.size.0, view.size.1, uv)
            .filter(|s| s.valid);
        let brush_transform = surface.as_ref().and_then(|surface| {
            vm.paint_surface_brush_transform(view.size.0, view.size.1, uv, surface)
        });
        vm.set_active_vm(active);
        Some(EditorPaintPick {
            output: displayed,
            camera,
            ray,
            surface,
            brush_transform,
        })
    }
    /// Called only in response to an authoring request, never frame preparation
    /// or ordinary picking. Existing GPU capture and explicit CPU fallback keep
    /// identical static-surface policies; no reference VM/device is borrowed.
    pub fn capture_editor_paint(&mut self, displayed: ViewImage) -> Option<EditorPaintCapture> {
        if self.context.is_lost() {
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
        #[cfg(not(target_arch = "wasm32"))]
        let gpu_capture = vm.paint_surface_buffer_gpu(view.size.0, view.size.1);
        #[cfg(target_arch = "wasm32")]
        let gpu_capture: Option<scenevm::PaintSurfaceBuffer> = None;
        let buffer = if let Some(buffer) = gpu_capture {
            buffer
        } else {
            vm.paint_surface_buffer(view.size.0, view.size.1)
        };
        vm.set_active_vm(active);
        Some(EditorPaintCapture {
            output: displayed,
            camera,
            buffer: std::sync::Arc::new(buffer),
        })
    }
}
