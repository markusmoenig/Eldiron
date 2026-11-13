//! Immutable CPU frame descriptions for editor scene preparation.
//!
//! Cameras, transforms, grid and lighting policy are independent of pixel output.
//! The CPU reference client and application-owned GPU views apply these same
//! descriptions to their own SceneHandler. This module neither draws nor reads
//! back a viewport, and never owns a window, document or simulation clock.
use crate::{Client, Map, Pixel, SceneHandler};
use scenevm::Camera3D;
use vek::{Mat3, Vec2, Vec4};

/// Base-scene overview with independent X/Y fitting and stable daytime lighting.
/// Shared by the explicit raster reference and the application-owned GPU view.
pub struct MinimapView2D {
    pub size: (u32, u32),
    pub bounds: Vec4<f32>,
    pub animation_frame: usize,
}
impl MinimapView2D {
    pub fn prepare(&self, scene: &mut SceneHandler) {
        let size = Vec2::new(self.size.0 as f32, self.size.1 as f32);
        let scale = size / Vec2::new(self.bounds.z, self.bounds.w);
        let center = Vec2::new(
            self.bounds.x + self.bounds.z * 0.5,
            self.bounds.y + self.bounds.w * 0.5,
        );
        let transform = Mat3::<f32>::translation_2d(size * 0.5 - center * scale)
            * Mat3::new(scale.x, 0., 0., 0., scale.y, 0., 0., 0., 1.);
        scene.vm.set_active_vm(0);
        scene.vm.set_layer_enabled(0, true);
        for layer in 1..scene.vm.vm_layer_count() {
            scene.vm.set_layer_enabled(layer, false);
        }
        scene.vm.execute(scenevm::Atom::SetRenderMode(
            scene.settings.scenevm_mode_2d(),
        ));
        scene.settings.apply_hour(12.);
        scene.settings.apply_2d(&mut scene.vm);
        scene.vm.execute(scenevm::Atom::SetGP0(Vec4::zero()));
        scene.vm.execute(scenevm::Atom::SetTransform2D(transform));
        scene
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));
        scene.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
    }
}

#[derive(Clone, Debug)]
pub struct EditorView2D {
    pub size: (u32, u32),
    pub grid_size: f32,
    pub subdivisions: f32,
    pub offset: Vec2<f32>,
    pub draw_grid: bool,
    pub hour: f32,
    pub animation_frame: usize,
    pub canvas_colors: Option<[Pixel; 3]>,
}
#[derive(Clone, Debug)]
pub struct EditorView3D {
    pub size: (u32, u32),
    pub grid_size: f32,
    pub offset: Vec2<f32>,
    pub hour: f32,
    pub animation_frame: usize,
    pub camera: Camera3D,
    /// Preserve the reference's separate editor-guide camera when the base view
    /// uses a bake/capture override.
    pub overlay_camera: Camera3D,
    pub canvas_colors: Option<[Pixel; 3]>,
    pub neutral_background: bool,
    pub static_geometry: bool,
    pub transparent_background: bool,
}
#[derive(Clone, Debug)]
pub enum EditorViewFrame {
    D2(EditorView2D),
    D3(EditorView3D),
}
impl EditorViewFrame {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::D2(frame) => frame.size,
            Self::D3(frame) => frame.size,
        }
    }
    /// Apply only scene state. The caller chooses its retained GPU target or
    /// explicit CPU reference/capture separately. Sprite-isometric source
    /// preparation belongs to its dedicated compositor, not this ordinary view.
    pub fn prepare(&self, scene: &mut SceneHandler) {
        match self {
            Self::D2(frame) => frame.prepare(scene),
            Self::D3(frame) => frame.prepare(scene),
        }
    }
}
impl Client {
    /// Capture plain authoring-view state without retaining a Client or Map.
    pub fn editor_view_2d(&self, map: &Map, size: (u32, u32)) -> EditorViewFrame {
        EditorViewFrame::D2(EditorView2D {
            size,
            grid_size: map.grid_size,
            subdivisions: map.subdivisions,
            offset: map.offset,
            draw_grid: self.builder_d2.draw_grid,
            hour: self.server_time.to_f32(),
            animation_frame: self.animation_frame,
            canvas_colors: self.editor_canvas_colors.filter(|_| self.use_editor_canvas),
        })
    }
    pub fn editor_view_3d(
        &self,
        map: &Map,
        size: (u32, u32),
        neutral_background: bool,
        camera_override: Option<Camera3D>,
        static_geometry: bool,
        transparent_background: bool,
    ) -> EditorViewFrame {
        let camera = self
            .camera_d3
            .as_scenevm_camera_for_surface(size.0 as f32, size.1 as f32);
        EditorViewFrame::D3(EditorView3D {
            size,
            grid_size: map.grid_size,
            offset: map.offset,
            hour: self.server_time.to_f32(),
            animation_frame: self.animation_frame,
            camera: camera_override.unwrap_or(camera),
            overlay_camera: camera,
            canvas_colors: self.editor_canvas_colors.filter(|_| self.use_editor_canvas),
            neutral_background,
            static_geometry,
            transparent_background,
        })
    }
}
impl EditorView2D {
    fn prepare(&self, scene_handler: &mut SceneHandler) {
        let screen_size = Vec2::new(self.size.0 as f32, self.size.1 as f32);
        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            self.offset.x + screen_size.x / 2.0,
            -self.offset.y + screen_size.y / 2.0,
        ));
        let scale_matrix = Mat3::new(
            self.grid_size,
            0.0,
            0.0,
            0.0,
            self.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;
        let rendered_grid_size = if self.draw_grid { self.grid_size } else { 0.0 };
        let scenevm_mode_2d = scene_handler.settings.scenevm_mode_2d();
        scene_handler.vm.set_active_vm(0);
        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                rendered_grid_size,
                self.subdivisions,
                self.offset.x,
                -self.offset.y,
            )));
        }

        let hour = self.hour;

        // Ensure base scene layer is visible in editor 2D mode.
        let overlay_layer_enabled = if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.is_layer_enabled(1).unwrap_or(true)
        } else {
            false
        };
        scene_handler.vm.set_layer_enabled(0, true);
        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, overlay_layer_enabled);
        }
        for layer in 2..scene_handler.vm.vm_layer_count() {
            scene_handler.vm.set_layer_enabled(layer, false);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));

        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_2d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_2d();
        let colors = self.canvas_colors;
        let rgba = |c: Pixel| {
            // Raster clears pass through gamma encoding; custom compute output
            // already writes display-space pixels, just like the overlay layer.
            let component = |v: u8| {
                let value = v as f32 / 255.;
                if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
                    value
                } else {
                    value.powf(2.2)
                }
            };
            Vec4::new(component(c[0]), component(c[1]), component(c[2]), 1.)
        };
        if let Some([background, grid, subgrid]) = colors {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetBackground(rgba(background)));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP5(rgba(background)));
            scene_handler.vm.execute(scenevm::Atom::SetGP6(rgba(grid)));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP7(rgba(subgrid)));
        } else {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP5(Vec4::zero()));
        }

        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                rendered_grid_size,
                self.subdivisions,
                self.offset.x,
                -self.offset.y,
            )));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        // Set the transform for the overlay if active
        if scene_handler.vm.vm_layer_count() > 1 && overlay_layer_enabled {
            for layer in 2..scene_handler.vm.vm_layer_count() {
                scene_handler.vm.set_layer_enabled(layer, false);
            }
            scene_handler.vm.set_active_vm(1);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            // 2D overlay shader expects grid params in GP0 and a non-zero GP2.x to draw background/grid.
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                rendered_grid_size,
                self.subdivisions,
                self.offset.x,
                -self.offset.y,
            )));
            scene_handler.vm.execute(scenevm::Atom::SetGP2(Vec4::one()));
            // Overlay compute output is display-space, unlike the base raster clear.
            let color = |c: Pixel| {
                Vec4::new(
                    c[0] as f32 / 255.,
                    c[1] as f32 / 255.,
                    c[2] as f32 / 255.,
                    1.,
                )
            };
            if let Some(colors) = self.canvas_colors {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(color(colors[0])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP6(color(colors[1])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP7(color(colors[2])));
            } else {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(Vec4::zero()));
            }

            scene_handler
                .vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));
            scene_handler.vm.set_active_vm(0);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));
    }
}
impl EditorView3D {
    fn prepare(&self, scene_handler: &mut SceneHandler) {
        scene_handler.vm.set_active_vm(0);

        let hour = self.hour;

        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_3d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_3d();
        if self.neutral_background {
            scene_handler.vm.execute(scenevm::Atom::SetGP0({
                let rgb = self
                    .canvas_colors
                    .map(|colors| {
                        std::array::from_fn(|i| {
                            let display = (colors[0][i] as f32 / 255.).powf(2.2);
                            display / (1. - display).max(0.001)
                        })
                    })
                    .unwrap_or([0.018, 0.018, 0.020]);
                Vec4::new(rgb[0], rgb[1], rgb[2], 0.)
            }));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));

        scene_handler.vm.execute(scenevm::Atom::SetBackground(
            if self.transparent_background {
                Vec4::zero()
            } else {
                Vec4::new(0.0, 0.0, 0.0, 1.0)
            },
        ));

        let base_render_mode = if !self.static_geometry {
            scenevm::RenderMode::Raster3D
        } else if scene_handler.vm.layer_progressive_sample_index(0).is_some() {
            scenevm::RenderMode::Compute3D
        } else {
            scene_handler.settings.scenevm_mode_3d()
        };
        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(base_render_mode));
        scene_handler
            .vm
            .set_layer_raster3d_static_geometry_enabled(0, self.static_geometry);
        // Organic foliage/paint stamps are static baked content even though Raster3D draws
        // them through a dedicated billboard pipeline. Do not draw them again in the live
        // dynamic overlay.
        scene_handler.vm.execute(scenevm::Atom::SetOrganicVisible {
            visible: self.static_geometry,
        });

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera,
        });

        // In 3D mode, enable overlay layers.
        for layer in 1..scene_handler.vm.vm_layer_count() {
            scene_handler.vm.set_layer_enabled(layer, true);
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_active_vm(1);

            let screen_size = Vec2::new(self.size.0 as f32, self.size.1 as f32);
            let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
                self.offset.x + screen_size.x / 2.0,
                -self.offset.y + screen_size.y / 2.0,
            ));
            let scale_matrix = Mat3::new(
                self.grid_size,
                0.0,
                0.0,
                0.0,
                self.grid_size,
                0.0,
                0.0,
                0.0,
                1.0,
            );
            let transform = translation_matrix * scale_matrix;
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            // Prevent stale 2D grid/background params from leaking into 3D rendering.
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP2(Vec4::zero()));

            for layer in 2..scene_handler.vm.vm_layer_count() {
                scene_handler.vm.set_active_vm(layer);
                scene_handler.apply_runtime_render_state_settings();
                scene_handler.settings.apply_3d(&mut scene_handler.vm);
                scene_handler.apply_runtime_render_state_3d();
                scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
                    camera: self.overlay_camera,
                });
                scene_handler.vm.execute(scenevm::Atom::SetRenderMode(
                    scene_handler.settings.scenevm_mode_3d(),
                ));
                // Keep editor overlay lines readable and color-accurate regardless of world lighting.
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP2(Vec4::new(0.0, 0.0, 0.0, 0.0))); // sun off
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP3(Vec4::new(1.0, 1.0, 1.0, 1.0))); // full ambient
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP4(Vec4::new(0.0, 0.0, 0.0, 0.0))); // fog off
            }
            scene_handler.vm.set_active_vm(0);
        }
    }
}
