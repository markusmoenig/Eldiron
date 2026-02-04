use crate::prelude::*;
use crate::{PlayerCamera, Rect, SceneHandler};
use crate::{ValueGroups, ValueTomlLoader};
use theframework::prelude::*;
use vek::Vec2;

pub struct GameWidget {
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
}

impl Default for GameWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWidget {
    pub fn new() -> Self {
        Self {
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
        }
    }

    pub fn init(&mut self) {
        // Parse UI settings via the shared TOML loader to stay consistent.
        if let Ok(groups) = ValueTomlLoader::from_str(&self.toml_str) {
            if let Some(ui) = groups.get("ui") {
                self.grid_size = ui.get_float_default("grid_size", self.grid_size);
                self.upscale = ui.get_float_default("upscale", 1.0).max(1.0);
            }
            if let Some(camera) = groups.get("camera") {
                let camera_type = camera.get_str_default("type".into(), "2d".into());
                if camera_type == "iso" {
                    self.camera = PlayerCamera::D3Iso;
                    self.camera_d3 = Box::new(D3IsoCamera::new());
                } else if camera_type == "firstp" {
                    self.camera = PlayerCamera::D3FirstP;
                }
            }
            self.table = groups;
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, _scene_handler: &mut SceneHandler) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }

        self.scenemanager
            .set_tile_list(assets.tile_list.clone(), assets.tile_indices.clone());

        self.scenemanager.send(SceneManagerCmd::SetMap(map.clone()));
        self.build_region_name = map.name.clone();
    }

    pub fn apply_entities(
        &mut self,
        map: &Map,
        assets: &Assets,
        animation_frame: usize,
        scene_handler: &mut SceneHandler,
    ) {
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
                    entity.apply_to_camera(&mut self.camera_d3);
                }

                self.player_pos = entity.get_pos_xz();
                break;
            }
        }

        if self.camera == PlayerCamera::D2 {
            scene_handler.build_dynamics_2d(map, assets);
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
        if map.name != self.build_region_name {
            self.build(map, assets, scene_handler);
        }
        self.scenemanager.tick();

        // Apply scene manager chunks
        while let Some(result) = self.scenemanager.receive() {
            match result {
                SceneManagerResult::Chunk(chunk, _togo, _total, billboards) => {
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
                    scene_handler.vm.execute(scenevm::Atom::ClearGeometry);
                    scene_handler.billboards.clear();
                    scene_handler.billboard_anim_states.clear();
                }
                _ => {}
            }
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, false);
            scene_handler.vm.set_layer_enabled(2, false);
        }

        if self.camera == PlayerCamera::D2 {
            self.draw_d2(map, time, animation_frame, assets, scene_handler);
        } else {
            self.draw_d3(map, time, animation_frame, assets, scene_handler);
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, true);
            scene_handler.vm.set_layer_enabled(2, true);
        }
    }

    /// Draw the 2D scene.
    pub fn draw_d2(
        &mut self,
        _map: &Map,
        time: &TheTime,
        animation_frame: usize,
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (width, height, render_buffer) = if self.upscale > 1.0 {
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

        scene_handler
            .vm
            .execute(scenevm::Atom::SetGP0(Vec4::zero()));

        let hour = time.to_f32();

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));

        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_2d(&mut scene_handler.vm);

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::zero()));

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

    pub fn draw_d3(
        &mut self,
        _map: &Map,
        time: &TheTime,
        animation_frame: usize,
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        let full_width = self.buffer.dim().width as usize;
        let full_height = self.buffer.dim().height as usize;

        // Determine render dimensions based on upscale factor
        let (width, height, render_buffer) = if self.upscale > 1.0 {
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

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera_d3.as_scenevm_camera(),
        });

        // scene_handler.vm.print_geometry_stats();

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
