//! Explicit CPU reference/bake adapter retained during caller retirement.
//! Creator lends its controller; standalone compatibility callers retain theirs.
use super::*;

impl Rusterix {
    /// Start a progressive, world-tiled bake for an orthographic camera.
    /// The previous committed frame remains available until the replacement
    /// successfully completes.
    pub fn request_orthographic_bake(&mut self, region_id: Uuid, samples: u32) {
        self.scene_handler.vm.clear_layer_frozen_rgba(0);
        self.scene_handler
            .vm
            .set_layer_progressive_sample_index(0, None);
        self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);
        self.orthographic_bake.request_render(region_id, samples);
    }

    pub fn toggle_orthographic_bake_visibility(&mut self) -> Option<bool> {
        let visible = self.orthographic_bake.toggle_visibility()?;
        if !visible {
            self.scene_handler.vm.clear_layer_frozen_rgba(0);
        }
        Some(visible)
    }

    pub fn clear_orthographic_bake(&mut self) {
        self.orthographic_bake.clear();
        self.scene_handler.vm.clear_layer_frozen_rgba(0);
        self.scene_handler
            .vm
            .set_layer_progressive_sample_index(0, None);
        self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);
    }

    pub fn take_orthographic_bake_persisted_update(
        &mut self,
    ) -> Option<Option<crate::map::OrthographicBakeAsset>> {
        self.orthographic_bake.take_persisted_update()
    }

    pub(super) fn orthographic_bake_lighting(&self) -> OrthographicBakeLighting {
        OrthographicBakeLighting::from_settings(&self.scene_handler.settings)
    }

    pub(super) fn orthographic_bake_background(&self, editor_neutral_background: bool) -> [u8; 4] {
        crate::orthographic_bake::background_color(
            &self.scene_handler.settings,
            editor_neutral_background,
            self.client
                .editor_canvas_colors
                .filter(|_| self.client.use_editor_canvas)
                .map(|colors| colors[0]),
        )
    }

    fn reset_orthographic_bake_renderer(&mut self) {
        self.scene_handler.vm.clear_layer_frozen_rgba(0);
        self.scene_handler
            .vm
            .set_layer_progressive_sample_index(0, None);
        self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);
    }

    /// Compatibility CPU viewport adapter. Canonical GPU consumers borrow
    /// their application-owned controller through immutable tile sources.
    pub fn draw_d3_with_editor_background(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        editor_neutral_background: bool,
    ) {
        let mut bake = std::mem::take(&mut self.orthographic_bake);
        self.draw_d3_with_bake_controller(
            map,
            pixels,
            width,
            height,
            editor_neutral_background,
            &mut bake,
        );
        self.orthographic_bake = bake;
    }

    /// Draw the client scene in 3D, optionally using the neutral construction background.
    pub fn draw_d3_with_bake_controller(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        editor_neutral_background: bool,
        bake: &mut OrthographicBakeController,
    ) {
        if !bake.is_rendering() || matches!(bake.status(), crate::OrthographicBakeStatus::Requested)
        {
            self.reset_orthographic_bake_renderer();
        }
        self.flush_deferred_tile_textures_3d(true);
        // Baking uses authored sky/material state, while its dedicated path
        // pass deliberately excludes direct sun and point lights. The editor's
        // flat-lighting preview must not alter the persisted transport data.
        let bake_rendering = bake.is_rendering();
        self.apply_runtime_render_state(map, !bake_rendering);
        self.scene_handler.apply_runtime_render_state_settings();
        self.scene_handler
            .settings
            .apply_hour(self.client.server_time.to_f32());

        let camera = self
            .client
            .camera_d3
            .as_scenevm_camera_for_surface(width as f32, height as f32);
        bake.sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
        let scene_bounds = if bake.needs_scene_bounds(map.id) {
            self.scene_handler.vm.layer_scene_projected_bounds_3d(
                0,
                camera.right.normalized(),
                camera.up.normalized(),
            )
        } else {
            None
        };
        let bake_work =
            bake.prepare_work(map.id, width as u32, height as u32, camera, scene_bounds);

        let bake_work = match bake_work {
            Ok(work) => work,
            Err(error) => {
                bake.fail(error);
                self.scene_handler
                    .vm
                    .set_layer_progressive_sample_index(0, None);
                self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);
                None
            }
        };

        if let Some(work) = bake_work {
            self.scene_handler.vm.clear_layer_frozen_rgba(0);
            self.scene_handler.vm.set_active_vm(0);
            self.scene_handler.vm.execute(scenevm::Atom::ClearDynamics);
            if work.sample == 0 {
                self.scene_handler.vm.set_layer_ping_pong_enabled(0, true);
            }
            self.scene_handler
                .vm
                .set_layer_progressive_sample_index(0, Some(work.sample));

            let tile_len = work.tile_size as usize * work.tile_size as usize * 4;
            self.orthographic_bake_work_pixels.resize(tile_len, 0);

            self.client.draw_d3_with_camera_override(
                map,
                &mut self.orthographic_bake_work_pixels,
                work.tile_size as usize,
                work.tile_size as usize,
                &self.assets,
                &mut self.scene_handler,
                editor_neutral_background,
                Some(work.camera),
                true,
                false,
            );
            if let Err(error) = bake.preview_work(&work, &self.orthographic_bake_work_pixels) {
                bake.fail(error);
            }
            // The bake deliberately excludes characters, editor preview icons,
            // particles, and doors. Rebuild them for the next live overlay pass.
            self.scene_handler.mark_dynamics_dirty();

            if bake.finish_work_sample(&work).unwrap_or(false) {
                let captured = self.scene_handler.vm.capture_layer_rgba(0);
                self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);

                match captured {
                    Some((_width, _height, rgba)) => {
                        self.scene_handler
                            .vm
                            .set_layer_progressive_sample_index(0, Some(0));
                        self.scene_handler.vm.set_layer_compute3d_bake_output(0, 1);
                        self.client.draw_d3_with_camera_override(
                            map,
                            &mut self.orthographic_bake_work_pixels,
                            work.tile_size as usize,
                            work.tile_size as usize,
                            &self.assets,
                            &mut self.scene_handler,
                            editor_neutral_background,
                            Some(work.camera),
                            true,
                            false,
                        );
                        let albedo = self.orthographic_bake_work_pixels.clone();

                        self.scene_handler.vm.set_layer_compute3d_bake_output(0, 2);
                        self.client.draw_d3_with_camera_override(
                            map,
                            &mut self.orthographic_bake_work_pixels,
                            work.tile_size as usize,
                            work.tile_size as usize,
                            &self.assets,
                            &mut self.scene_handler,
                            editor_neutral_background,
                            Some(work.camera),
                            true,
                            false,
                        );
                        let material = self.orthographic_bake_work_pixels.clone();

                        self.scene_handler.vm.set_layer_compute3d_bake_output(0, 3);
                        self.client.draw_d3_with_camera_override(
                            map,
                            &mut self.orthographic_bake_work_pixels,
                            work.tile_size as usize,
                            work.tile_size as usize,
                            &self.assets,
                            &mut self.scene_handler,
                            editor_neutral_background,
                            Some(work.camera),
                            true,
                            false,
                        );
                        let normal = self.orthographic_bake_work_pixels.clone();

                        self.scene_handler.vm.set_active_vm(0);
                        let surface = self
                            .scene_handler
                            .vm
                            .paint_surface_buffer_gpu(work.tile_size, work.tile_size);
                        let pixel_count = work.tile_size as usize * work.tile_size as usize;
                        let mut depth = vec![f32::INFINITY; pixel_count];
                        if let Some(surface) = surface {
                            for (index, pixel) in surface.pixels.iter().enumerate() {
                                if !pixel.valid || normal[index * 4 + 3] == 0 {
                                    continue;
                                }
                                depth[index] = pixel.depth;
                            }
                        }

                        self.scene_handler.vm.set_layer_compute3d_bake_output(0, 0);
                        self.scene_handler
                            .vm
                            .set_layer_progressive_sample_index(0, None);
                        if let Err(error) =
                            bake.commit_work(&work, rgba, depth, albedo, normal, material)
                        {
                            bake.fail(error);
                        }
                    }
                    None => {
                        self.scene_handler.vm.set_layer_compute3d_bake_output(0, 0);
                        self.scene_handler
                            .vm
                            .set_layer_progressive_sample_index(0, None);
                        bake.fail("The rendered bake could not be read back from the GPU.");
                    }
                }
            }

            let lighting = self.orthographic_bake_lighting();
            if let Some(mut composed) =
                bake.compose_rgba_lit(map.id, width as u32, height as u32, &camera, Some(lighting))
            {
                composite_rgba_over_solid_background(
                    &mut composed,
                    self.orthographic_bake_background(editor_neutral_background),
                );
                let copy_len = pixels.len().min(composed.len());
                pixels[..copy_len].copy_from_slice(&composed[..copy_len]);
            } else {
                // Do not leave the previous committed bake (and any old baked
                // editor symbols) in the viewport before the first new tile is ready.
                pixels.fill(0);
            }
            return;
        }

        let lighting = self.orthographic_bake_lighting();
        if let Some(mut composed) =
            bake.compose_rgba_lit(map.id, width as u32, height as u32, &camera, Some(lighting))
        {
            let baked_depth = bake.compose_depth(map.id, width as u32, height as u32, &camera);
            self.scene_handler.vm.clear_layer_frozen_rgba(0);
            self.orthographic_bake_overlay_pixels
                .resize(width.saturating_mul(height).saturating_mul(4), 0);
            self.orthographic_bake_overlay_pixels.fill(0);
            self.client.draw_d3_dynamic_overlay(
                map,
                &mut self.orthographic_bake_overlay_pixels,
                width,
                height,
                &self.assets,
                &mut self.scene_handler,
            );
            self.scene_handler.vm.set_active_vm(0);
            let dynamic_surface = self
                .scene_handler
                .vm
                .paint_surface_buffer_gpu_dynamic_only(width as u32, height as u32);
            blend_rgba_over_depth(
                &mut composed,
                &self.orthographic_bake_overlay_pixels,
                baked_depth.as_deref(),
                dynamic_surface.as_ref(),
            );
            composite_rgba_over_solid_background(
                &mut composed,
                self.orthographic_bake_background(editor_neutral_background),
            );
            let copy_len = pixels.len().min(composed.len());
            pixels[..copy_len].copy_from_slice(&composed[..copy_len]);
            self.scene_handler
                .vm
                .set_layer_raster3d_static_geometry_enabled(0, true);
            return;
        }

        self.scene_handler.vm.clear_layer_frozen_rgba(0);
        self.scene_handler
            .vm
            .set_layer_raster3d_static_geometry_enabled(0, true);

        self.client.draw_d3(
            map,
            pixels,
            width,
            height,
            &self.assets,
            &mut self.scene_handler,
            editor_neutral_background,
        );
    }
}
