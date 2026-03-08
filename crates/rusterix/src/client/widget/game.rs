use crate::prelude::*;
use crate::{PlayerCamera, Rect, SceneHandler};
use crate::{ValueGroups, ValueTomlLoader};
use theframework::prelude::*;
use vek::Vec2;

pub struct GameWidget {
    pub name: String,
    pub scenemanager: SceneManager,

    pub camera_d3: Box<dyn D3Camera>,

    pub rect: Rect,

    pub scene: Scene,

    pub buffer: TheRGBABuffer,

    pub map_bbox: Vec4<f32>,

    pub grid_size: f32,
    pub top_left: Vec2<f32>,

    pub player_pos: Vec2<f32>,

    pub toml_str: String,
    pub table: ValueGroups,

    pub camera: PlayerCamera,

    // Used to detect region changes (have to rebuild the geometry)
    pub build_region_name: String,

    // Upscale factor (1.0 = no upscaling, >1.0 = render at lower res and upscale)
    pub upscale: f32,
    // Secondary buffer for rendering at lower resolution when upscale > 1
    pub upscale_buffer: TheRGBABuffer,

    pub current_sector_name: String,
    pub iso_hidden_sectors: FxHashSet<u32>,
    pub iso_sector_fade: FxHashMap<u32, f32>,
    pub force_dynamics_rebuild: bool,
    pub firstp_eye_level: f32,
    pub loaded_chunks: FxHashSet<(i32, i32)>,
    pub stream_load_radius_chunks: i32,
    pub stream_prefetch_radius_chunks: i32,
    pub chunk_build_budget_near: i32,
    pub chunk_build_budget_far: i32,
    pub last_stream_focus_chunk: Option<(i32, i32)>,
}

impl Default for GameWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            scenemanager: SceneManager::default(),

            camera_d3: Box::new(D3FirstPCamera::new()),

            rect: Rect::default(),

            scene: Scene::default(),

            buffer: TheRGBABuffer::default(),

            map_bbox: Vec4::zero(),

            grid_size: 32.0,
            top_left: Vec2::zero(),

            player_pos: Vec2::zero(),

            toml_str: String::new(),
            table: ValueGroups::default(),

            camera: PlayerCamera::D2,

            build_region_name: String::new(),

            upscale: 1.0,
            upscale_buffer: TheRGBABuffer::default(),
            current_sector_name: String::new(),
            iso_hidden_sectors: FxHashSet::default(),
            iso_sector_fade: FxHashMap::default(),
            force_dynamics_rebuild: true,
            firstp_eye_level: 1.7,
            loaded_chunks: FxHashSet::default(),
            stream_load_radius_chunks: 2,
            stream_prefetch_radius_chunks: 5,
            chunk_build_budget_near: 10,
            chunk_build_budget_far: 2,
            last_stream_focus_chunk: None,
        }
    }

    fn apply_iso_camera_overrides(&self, iso: &mut D3IsoCamera) {
        if let Some(camera) = self.table.get("camera") {
            let default_azimuth = iso.get_parameter_f32("azimuth_deg");
            let default_elevation = iso.get_parameter_f32("elevation_deg");
            let default_scale = iso.scale();

            let azimuth = camera.get_float_default(
                "azimuth",
                camera.get_float_default("azimuth_deg", default_azimuth),
            );
            let elevation = camera.get_float_default(
                "elevation",
                camera.get_float_default("elevation_deg", default_elevation),
            );
            let scale = camera.get_float_default("scale", default_scale);

            iso.set_parameter_f32("azimuth_deg", azimuth);
            iso.set_parameter_f32("elevation_deg", elevation);
            iso.set_parameter_f32("scale", scale);
        }
    }

    pub fn set_camera_mode(&mut self, camera: PlayerCamera) {
        self.camera = camera;
        self.force_dynamics_rebuild = true;
        match self.camera {
            PlayerCamera::D2 => {}
            PlayerCamera::D3Iso => {
                let mut iso = D3IsoCamera::new();
                self.apply_iso_camera_overrides(&mut iso);
                self.camera_d3 = Box::new(iso);
            }
            PlayerCamera::D3FirstP => {
                self.camera_d3 = Box::new(D3FirstPCamera::new());
            }
        }
    }

    pub fn init(&mut self) {
        // Parse UI settings via the shared TOML loader to stay consistent.
        if let Ok(groups) = ValueTomlLoader::from_str(&self.toml_str) {
            if let Some(ui) = groups.get("ui") {
                self.grid_size = ui.get_float_default("grid_size", self.grid_size);
                self.upscale = ui.get_float_default("upscale", 1.0).max(1.0);
                self.stream_load_radius_chunks =
                    ui.get_int_default("chunk_load_radius", self.stream_load_radius_chunks);
                self.stream_prefetch_radius_chunks =
                    ui.get_int_default("chunk_prefetch_radius", self.stream_prefetch_radius_chunks);
                self.chunk_build_budget_near =
                    ui.get_int_default("chunk_build_budget_near", self.chunk_build_budget_near);
                self.chunk_build_budget_far =
                    ui.get_int_default("chunk_build_budget_far", self.chunk_build_budget_far);
            }
            self.table = groups;
            if let Some(camera) = self.table.get("camera") {
                let camera_type = camera.get_str_default("type".into(), "2d".into());
                if camera_type == "iso" {
                    self.set_camera_mode(PlayerCamera::D3Iso);
                } else if camera_type == "firstp" {
                    self.set_camera_mode(PlayerCamera::D3FirstP);
                } else {
                    self.set_camera_mode(PlayerCamera::D2);
                }
            }
        }
        self.force_dynamics_rebuild = true;
    }

    fn player_chunk_origin(&self, chunk_size: i32) -> (i32, i32) {
        let px = self.player_pos.x.floor() as i32;
        let py = self.player_pos.y.floor() as i32;
        (
            px.div_euclid(chunk_size) * chunk_size,
            py.div_euclid(chunk_size) * chunk_size,
        )
    }

    fn desired_stream_chunks(&self, map: &Map, radius_chunks: i32) -> FxHashSet<(i32, i32)> {
        let chunk_size = map.terrain.chunk_size.max(1);
        let mut bbox = map.bbox();
        if let Some(tbbox) = map.terrain.compute_bounds() {
            bbox.expand_bbox(tbbox);
        }
        let min_x = (bbox.min.x / chunk_size as f32).floor() as i32;
        let min_y = (bbox.min.y / chunk_size as f32).floor() as i32;
        let max_x = (bbox.max.x / chunk_size as f32).ceil() as i32;
        let max_y = (bbox.max.y / chunk_size as f32).ceil() as i32;

        let center = self.player_chunk_origin(chunk_size);
        let center_cx = center.0.div_euclid(chunk_size);
        let center_cy = center.1.div_euclid(chunk_size);

        let mut desired = FxHashSet::default();
        for cy in (center_cy - radius_chunks)..=(center_cy + radius_chunks) {
            for cx in (center_cx - radius_chunks)..=(center_cx + radius_chunks) {
                if cx < min_x || cy < min_y || cx >= max_x || cy >= max_y {
                    continue;
                }
                desired.insert((cx * chunk_size, cy * chunk_size));
            }
        }
        desired
    }

    fn update_streaming_chunks(&mut self, map: &Map, scene_handler: &mut SceneHandler) {
        let chunk_size = map.terrain.chunk_size.max(1);
        let focus = self.player_chunk_origin(chunk_size);
        if self.last_stream_focus_chunk == Some(focus) {
            return;
        }
        self.last_stream_focus_chunk = Some(focus);
        self.scenemanager.set_focus_chunk(Some(focus));

        let load_radius = self.stream_load_radius_chunks.max(1);
        let prefetch_radius = self.stream_prefetch_radius_chunks.max(load_radius);
        let desired_load = self.desired_stream_chunks(map, load_radius);
        let desired_prefetch = self.desired_stream_chunks(map, prefetch_radius);

        // Unload chunks that are now outside the prefetch radius.
        let unload: Vec<(i32, i32)> = self
            .loaded_chunks
            .iter()
            .copied()
            .filter(|c| !desired_prefetch.contains(c))
            .collect();
        for coord in unload {
            scene_handler.vm.execute(scenevm::Atom::RemoveChunkAt {
                origin: Vec2::new(coord.0, coord.1),
            });
            self.loaded_chunks.remove(&coord);
        }

        // Prioritize immediate neighborhood, then allow wider prefetch queue.
        let mut to_request: Vec<(i32, i32)> = desired_load
            .iter()
            .copied()
            .filter(|c| !self.loaded_chunks.contains(c))
            .collect();
        to_request.extend(
            desired_prefetch
                .iter()
                .copied()
                .filter(|c| !self.loaded_chunks.contains(c) && !desired_load.contains(c)),
        );
        if !to_request.is_empty() {
            self.scenemanager.add_dirty(to_request);
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, scene_handler: &mut SceneHandler) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }

        // Force dynamic overlays (billboards/lights) to rebuild immediately after map swaps.
        scene_handler.mark_dynamics_dirty();
        self.force_dynamics_rebuild = true;

        self.scenemanager
            .set_tile_list(assets.tile_list.clone(), assets.tile_indices.clone());
        self.scenemanager.set_palette(assets.palette.clone());

        self.scenemanager.send(SceneManagerCmd::SetMap(map.clone()));
        self.loaded_chunks.clear();
        self.last_stream_focus_chunk = None;
        // Replace full-map queue with a player-centric startup queue.
        let startup = self.desired_stream_chunks(map, self.stream_load_radius_chunks.max(1));
        self.scenemanager
            .replace_dirty(startup.iter().copied().collect::<Vec<_>>());
        self.scenemanager.set_focus_chunk(Some(
            self.player_chunk_origin(map.terrain.chunk_size.max(1)),
        ));
        self.build_region_name = map.name.clone();
        self.iso_hidden_sectors.clear();
        self.iso_sector_fade.clear();
    }

    pub fn apply_entities(
        &mut self,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        if self.force_dynamics_rebuild {
            scene_handler.mark_dynamics_dirty();
            self.force_dynamics_rebuild = false;
        }

        for entity in map.entities.iter() {
            if entity.is_player() {
                // if let Some(Value::PlayerCamera(camera)) = entity.attributes.get("player_camera") {
                //     if *camera != self.camera {
                //         self.camera = camera.clone();
                //         if self.camera == PlayerCamera::D3Iso {
                //             self.camera_d3 = Box::new(D3IsoCamera::new())
                //         } else if self.camera == PlayerCamera::D3FirstP {
                //             self.camera_d3 = Box::new(D3FirstPCamera::new());
                //         }
                //         self.build(map, assets, scene_handler);
                //     }
                // }

                if self.camera != PlayerCamera::D2 {
                    entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
                }

                self.player_pos = entity.get_pos_xz();
                self.current_sector_name = entity
                    .get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .or_else(|| map.find_sector_at(self.player_pos).map(|s| s.name.clone()))
                    .unwrap_or_default();
                break;
            }
        }

        if self.camera == PlayerCamera::D2 {
            scene_handler.build_dynamics_2d(map, animation_frame, assets);
        } else {
            scene_handler.build_dynamics_3d(map, self.camera_d3.as_ref(), animation_frame, assets);
        }
    }

    pub fn draw(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.prepare_frame(map, time, animation_frame, assets, scene_handler);
        if self.camera == PlayerCamera::D2 {
            self.render_prepared_d2(time, animation_frame, scene_handler);
        } else {
            self.render_prepared_d3(time, animation_frame, scene_handler);
        }
    }

    /// Prepare the SceneVM state for this widget without CPU readback.
    pub fn prepare_frame(
        &mut self,
        map: &Map,
        time: &TheTime,
        animation_frame: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        if map.name != self.build_region_name {
            self.build(map, assets, scene_handler);
        }
        self.update_streaming_chunks(map, scene_handler);
        // Process more chunks per frame while nearby chunks are still missing.
        let desired_load = self.desired_stream_chunks(map, self.stream_load_radius_chunks.max(1));
        let missing_nearby = desired_load
            .iter()
            .filter(|coord| !self.loaded_chunks.contains(coord))
            .count();
        let budget = if missing_nearby > 0 {
            self.chunk_build_budget_near.max(1) as usize
        } else {
            self.chunk_build_budget_far.max(1) as usize
        };
        if self.scenemanager.is_busy() {
            self.scenemanager.tick_batch(budget);
        }

        // Apply scene manager chunks
        let mut geometry_changed = false;
        while let Some(result) = self.scenemanager.receive() {
            match result {
                SceneManagerResult::Chunk(chunk, _togo, _total, billboards) => {
                    geometry_changed = true;
                    self.loaded_chunks.insert((chunk.origin.x, chunk.origin.y));
                    scene_handler.vm.execute(scenevm::Atom::RemoveChunkAt {
                        origin: chunk.origin,
                    });

                    scene_handler.vm.execute(scenevm::Atom::AddChunk {
                        id: Uuid::new_v4(),
                        chunk: chunk,
                    });

                    // Add billboards to scene_handler (indexed by GeoId)
                    for billboard in billboards {
                        scene_handler.billboards.insert(billboard.geo_id, billboard);
                    }
                }
                SceneManagerResult::Clear => {
                    geometry_changed = true;
                    self.loaded_chunks.clear();
                    scene_handler.vm.execute(scenevm::Atom::ClearGeometry);
                    scene_handler.billboards.clear();
                    scene_handler.billboard_anim_states.clear();
                }
                _ => {}
            }
        }

        // Geometry streaming/reset can happen before dynamics are visible on first game frames.
        // Force a fresh dynamics pass whenever static scene content changed.
        if geometry_changed {
            scene_handler.mark_dynamics_dirty();
            self.force_dynamics_rebuild = true;
        }

        self.apply_iso_sector_visibility(map, scene_handler, geometry_changed);

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, false);
            scene_handler.vm.set_layer_enabled(2, false);
        }

        if self.camera == PlayerCamera::D2 {
            self.prepare_d2(time, animation_frame, scene_handler);
        } else {
            self.prepare_d3(time, animation_frame, scene_handler);
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, true);
            scene_handler.vm.set_layer_enabled(2, true);
        }
    }

    fn prepare_d2(
        &mut self,
        time: &TheTime,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (width, height, _render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;

            // Allocate/resize upscale buffer if needed
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        let screen_size = Vec2::new(width as f32, height as f32);

        let bbox = self.map_bbox;

        let start = Vec2::new(bbox.x, bbox.y);
        let end = Vec2::new(bbox.x + bbox.z, bbox.y + bbox.w);

        let start_pixels = start * self.grid_size;
        let end_pixels = end * self.grid_size;

        // Ensure min < max even if grid_size has negative components
        let min_world = Vec2::new(
            start_pixels.x.min(end_pixels.x),
            start_pixels.y.min(end_pixels.y),
        );
        let max_world = Vec2::new(
            start_pixels.x.max(end_pixels.x),
            start_pixels.y.max(end_pixels.y),
        );

        let half_screen = screen_size / 2.0;

        // Compute unclamped camera center in world space
        let mut camera_pos = self.player_pos * self.grid_size;

        let map_width_px = max_world.x - min_world.x;
        let map_height_px = max_world.y - min_world.y;

        if map_width_px > screen_size.x {
            camera_pos.x = camera_pos
                .x
                .clamp(min_world.x + half_screen.x, max_world.x - half_screen.x);
        } else {
            // Center map horizontally
            camera_pos.x = (min_world.x + max_world.x) / 2.0;
        }

        if map_height_px > screen_size.y {
            camera_pos.y = camera_pos
                .y
                .clamp(min_world.y + half_screen.y, max_world.y - half_screen.y);
        } else {
            // Center map vertically
            camera_pos.y = (min_world.y + max_world.y) / 2.0;
        }

        let translation_matrix =
            Mat3::<f32>::translation_2d((screen_size / 2.0 - camera_pos).floor());

        self.top_left = (camera_pos - screen_size / 2.0).floor() / self.grid_size;

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

        let hour = time.to_f32();
        let scenevm_mode_2d = scene_handler.settings.scenevm_mode_2d();
        scene_handler.vm.set_active_vm(0);
        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));

        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_2d(&mut scene_handler.vm);
        // 2D should always render against black, independent of project sky color/simulation.
        scene_handler
            .vm
            .execute(scenevm::Atom::SetGP0(Vec4::zero()));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::zero()));

        // Draw Messages

        /*
        if let Some(font) = &self.messages_font {
            for (grid_pos, message, text_size, _) in self.messages_to_draw.values() {
                let position = map_grid_to_local(screen_size, *grid_pos, map);

                let tuple = (
                    position.x as isize - *text_size as isize / 2 - 5,
                    position.y as isize - self.messages_font_size as isize - map.grid_size as isize,
                    *text_size as isize + 10,
                    22,
                );

                self.draw2d.blend_rect_safe(
                    pixels,
                    &tuple,
                    width,
                    &[0, 0, 0, 128],
                    &(0, 0, width as isize, height as isize),
                );

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &self.messages_font_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );
            }
        }*/
    }

    fn prepare_d3(
        &mut self,
        time: &TheTime,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (_width, _height, _render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;

            // Allocate/resize upscale buffer if needed
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        let hour = time.to_f32();

        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_3d(&mut scene_handler.vm);

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::new(0.0, 0.0, 0.0, 1.0)));

        scene_handler.vm.execute(scenevm::Atom::SetRenderMode(
            scene_handler.settings.scenevm_mode_3d(),
        ));

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera_d3.as_scenevm_camera(),
        });

        // scene_handler.vm.print_geometry_stats();
    }

    fn render_prepared_d2(
        &mut self,
        _time: &TheTime,
        _animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;
        let (width, height, render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        if render_buffer {
            scene_handler.vm.render_frame(
                self.upscale_buffer.pixels_mut(),
                width as u32,
                height as u32,
            );
            Self::upscale_buffer_into(
                &self.upscale_buffer,
                &mut self.buffer,
                full_width,
                full_height,
            );
        } else {
            scene_handler
                .vm
                .render_frame(self.buffer.pixels_mut(), width as u32, height as u32);
        }
    }

    fn render_prepared_d3(
        &mut self,
        _time: &TheTime,
        _animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;
        let (width, height, render_buffer) = if self.upscale > 1.0 {
            let scaled_width = (full_width as f32 / self.upscale).round() as usize;
            let scaled_height = (full_height as f32 / self.upscale).round() as usize;
            if self.upscale_buffer.dim().width as usize != scaled_width
                || self.upscale_buffer.dim().height as usize != scaled_height
            {
                self.upscale_buffer = TheRGBABuffer::new(TheDim::new(
                    0,
                    0,
                    scaled_width as i32,
                    scaled_height as i32,
                ));
            }
            (scaled_width, scaled_height, true)
        } else {
            (full_width, full_height, false)
        };

        if render_buffer {
            scene_handler.vm.render_frame(
                self.upscale_buffer.pixels_mut(),
                width as u32,
                height as u32,
            );
            Self::upscale_buffer_into(
                &self.upscale_buffer,
                &mut self.buffer,
                full_width,
                full_height,
            );
        } else {
            scene_handler
                .vm
                .render_frame(self.buffer.pixels_mut(), width as u32, height as u32);
        }
    }

    fn apply_iso_sector_visibility(
        &mut self,
        map: &Map,
        scene_handler: &mut SceneHandler,
        force_reapply: bool,
    ) {
        const FADE_STEP: f32 = 0.08;
        // Outside ISO mode, always force canonical sector visibility/opacity.
        if self.camera != PlayerCamera::D3Iso {
            scene_handler.vm.set_active_vm(0);
            for sector in &map.sectors {
                scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                    id: scenevm::GeoId::Sector(sector.id),
                    opacity: 1.0,
                });
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: sector.properties.get_bool_default("visible", true),
                });
            }
            self.iso_hidden_sectors.clear();
            self.iso_sector_fade.clear();
            return;
        }

        fn matches_pattern(name: &str, pattern: &str) -> bool {
            let name = name.trim().to_ascii_lowercase();
            let pattern = pattern.trim().to_ascii_lowercase();
            if pattern.is_empty() {
                return false;
            }
            if let Some(prefix) = pattern.strip_suffix('*') {
                name.starts_with(prefix)
            } else {
                name == pattern
            }
        }

        let mut target_hidden: FxHashSet<u32> = FxHashSet::default();

        // Multiple sectors may overlap at player position (e.g. foundations, interiors).
        // Collect hide patterns from all matching sectors instead of only the first one.
        let mut hide_patterns: Vec<String> = Vec::new();
        for sector in map
            .sectors
            .iter()
            .filter(|s| s.layer.is_none() && s.is_inside(map, self.player_pos))
        {
            if let Some(Value::StrArray(patterns)) = sector.properties.get("iso_hide_on_enter") {
                hide_patterns.extend(patterns.iter().cloned());
            }
        }
        if hide_patterns.is_empty()
            && let Some(current_sector) = map
                .sectors
                .iter()
                .find(|sector| sector.name == self.current_sector_name)
            && let Some(Value::StrArray(patterns)) =
                current_sector.properties.get("iso_hide_on_enter")
        {
            hide_patterns.extend(patterns.iter().cloned());
        }

        if !hide_patterns.is_empty() {
            for sector in &map.sectors {
                let roof_name = sector
                    .properties
                    .get_str_default("roof_name", String::new());
                let is_match = hide_patterns.iter().any(|pattern| {
                    matches_pattern(&sector.name, pattern)
                        || (!roof_name.is_empty() && matches_pattern(&roof_name, pattern))
                });
                if is_match {
                    target_hidden.insert(sector.id);
                }
            }
        }

        let unchanged = target_hidden == self.iso_hidden_sectors;
        let mut has_active_fade = false;
        for sector in &map.sectors {
            let target_alpha = if target_hidden.contains(&sector.id) {
                0.0
            } else {
                1.0
            };
            let current = *self
                .iso_sector_fade
                .get(&sector.id)
                .unwrap_or(&target_alpha);
            if (current - target_alpha).abs() > 1e-3 {
                has_active_fade = true;
                break;
            }
        }
        if !force_reapply && unchanged && !has_active_fade {
            return;
        }

        scene_handler.vm.set_active_vm(0);
        for sector in &map.sectors {
            let was_hidden = self.iso_hidden_sectors.contains(&sector.id);
            let should_hide = target_hidden.contains(&sector.id);
            let base_visible = sector.properties.get_bool_default("visible", true);
            let target_alpha = if should_hide { 0.0 } else { 1.0 };

            let current_alpha = self
                .iso_sector_fade
                .get(&sector.id)
                .copied()
                .unwrap_or(if was_hidden { 0.0 } else { 1.0 });

            let next_alpha = if current_alpha < target_alpha {
                (current_alpha + FADE_STEP).min(target_alpha)
            } else if current_alpha > target_alpha {
                (current_alpha - FADE_STEP).max(target_alpha)
            } else {
                current_alpha
            };

            // Ensure geometry is visible while fading in.
            if target_alpha > 0.0 && next_alpha > 0.0 {
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: base_visible,
                });
            }

            scene_handler.vm.execute(scenevm::Atom::SetGeoOpacity {
                id: scenevm::GeoId::Sector(sector.id),
                opacity: next_alpha,
            });

            if next_alpha <= 0.001 {
                scene_handler.vm.execute(scenevm::Atom::SetGeoVisible {
                    id: scenevm::GeoId::Sector(sector.id),
                    visible: false,
                });
            }

            self.iso_sector_fade.insert(sector.id, next_alpha);
        }

        self.iso_hidden_sectors = target_hidden;
    }

    /// Upscale the source buffer into the destination buffer using nearest-neighbor sampling.
    fn upscale_buffer_into(
        src: &TheRGBABuffer,
        dst: &mut TheRGBABuffer,
        dst_width: usize,
        dst_height: usize,
    ) {
        let src_width = src.dim().width as usize;
        let src_height = src.dim().height as usize;
        let src_pixels = src.pixels();
        let dst_pixels = dst.pixels_mut();

        let x_ratio = src_width as f32 / dst_width as f32;
        let y_ratio = src_height as f32 / dst_height as f32;

        // Pre-compute source X indices for the row
        let mut src_x_indices: Vec<usize> = Vec::with_capacity(dst_width);
        for x in 0..dst_width {
            src_x_indices.push(((x as f32 * x_ratio) as usize).min(src_width - 1));
        }

        for y in 0..dst_height {
            let src_y = ((y as f32 * y_ratio) as usize).min(src_height - 1);
            let dst_row_start = y * dst_width * 4;
            let src_row_start = src_y * src_width * 4;

            for (x, &src_x) in src_x_indices.iter().enumerate() {
                let dst_idx = dst_row_start + x * 4;
                let src_idx = src_row_start + src_x * 4;

                dst_pixels[dst_idx..dst_idx + 4].copy_from_slice(&src_pixels[src_idx..src_idx + 4]);
            }
        }
    }
}
