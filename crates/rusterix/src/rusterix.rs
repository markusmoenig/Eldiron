use crate::{
    Command, EntityAction, OrthographicBakeController, PlayerCamera, SceneHandler, Surface,
    prelude::*,
};
use indexmap::IndexMap;
use scenevm::Atom;
use theframework::prelude::*;
use vek::Vec2;

#[derive(PartialEq)]
pub enum ClientDrawMode {
    D2,
    D3,
}

use ClientDrawMode::*;

/// Rusterix can server as a server or client or both for solo games.
pub struct Rusterix {
    pub assets: Assets,
    pub server: Server,
    pub client: Client,
    pub audio: Option<AudioEngine>,

    pub is_dirty_d2: bool,
    pub is_dirty_d3: bool,
    pub draw_mode: ClientDrawMode,

    pub player_camera: PlayerCamera,

    pub scene_handler: SceneHandler,

    pub editor_preview_post_enabled: bool,
    pub editor_preview_lighting_enabled: bool,
    pub orthographic_bake: OrthographicBakeController,
    orthographic_bake_work_pixels: Vec<u8>,
    orthographic_bake_overlay_pixels: Vec<u8>,
}

impl Default for Rusterix {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusterix {
    fn apply_runtime_render_state(&mut self, map: &Map, apply_editor_preview: bool) {
        let mut state = self.server.get_render_state(&map.id);
        if apply_editor_preview && !self.editor_preview_post_enabled {
            state.post.set("enabled", Value::Bool(false));
        }
        if apply_editor_preview && !self.editor_preview_lighting_enabled {
            state.render.set("sun_enabled", Value::Bool(false));
            state.render.set("shadow_enabled", Value::Bool(false));
            state.render.set("ambient_strength", Value::Float(1.0));
        }
        self.scene_handler.runtime_render_state = state;
    }

    pub fn new() -> Self {
        Self::new_with_audio(true)
    }

    /// Construct an offscreen renderer without opening an audio device.
    pub fn new_without_audio() -> Self {
        Self::new_with_audio(false)
    }

    fn new_with_audio(enable_audio: bool) -> Self {
        let mut scene_handler = SceneHandler::default();

        if let Some(bytes) = crate::Embedded::get("shader/2d_shader.wgsl") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                scene_handler.vm.execute(Atom::SetSource2D(source.into()));
            }
        }

        // if let Some(bytes) = crate::Embedded::get("shader/3d_shader.wgsl") {
        //     if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
        //         scene_handler.vm.execute(Atom::SetSource3D(source.into()));
        //     }
        // }

        Self {
            assets: Assets::default(),
            server: Server::default(),
            client: Client::default(),
            audio: enable_audio.then(|| AudioEngine::new().ok()).flatten(),

            is_dirty_d2: true,
            is_dirty_d3: true,
            draw_mode: ClientDrawMode::D3,

            player_camera: PlayerCamera::D2,

            scene_handler,
            editor_preview_post_enabled: true,
            editor_preview_lighting_enabled: true,
            orthographic_bake: OrthographicBakeController::default(),
            orthographic_bake_work_pixels: Vec::new(),
            orthographic_bake_overlay_pixels: Vec::new(),
        }
    }

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

    /// Set to 2D mode.
    pub fn set_d2(&mut self) {
        self.draw_mode = D2;
    }

    /// Set to 3D mode.
    pub fn set_d3(&mut self) {
        self.draw_mode = D3;
    }

    /// Set the dirty flag, i.e. scene needs to be rebuild.
    pub fn set_dirty(&mut self) {
        self.is_dirty_d2 = true;
        self.is_dirty_d3 = true;
        self.scene_handler.mark_dynamics_dirty();
    }

    /// Invalidate only dynamic overlays (entities/items/lights), keep static geometry intact.
    pub fn set_overlay_dirty(&mut self) {
        self.scene_handler.mark_dynamics_dirty();
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets;
        self.load_audio_assets();
    }

    fn ensure_audio_engine(&mut self) {
        if self.audio.is_none() {
            self.audio = AudioEngine::new().ok();
        }
    }

    /// Load all audio assets into the runtime audio engine cache.
    pub fn load_audio_assets(&mut self) {
        self.ensure_audio_engine();
        let Some(engine) = self.audio.as_ref() else {
            return;
        };
        engine.clear_clips();
        for (name, bytes) in &self.assets.audio {
            let _ = engine.load_clip_from_bytes(name, bytes);
        }
        for name in crate::audio::list_audio_fx_names(&self.assets.audio_fx_src) {
            if let Ok(bytes) =
                crate::audio::synthesize_audio_fx_wav(&self.assets.audio_fx_src, &name)
            {
                let _ = engine.load_clip_from_bytes(&name, &bytes);
            }
        }
    }

    /// Play one-shot audio by asset name.
    pub fn play_audio(&mut self, name: &str) -> bool {
        self.ensure_audio_engine();
        let Some(engine) = self.audio.as_ref() else {
            return false;
        };
        engine.play_one_shot(name, 1.0)
    }

    /// Play audio on a given bus/layer (e.g. "music", "sfx"), optionally looping.
    pub fn play_audio_on_bus(&mut self, name: &str, bus: &str, gain: f32, looping: bool) -> bool {
        self.ensure_audio_engine();
        let Some(engine) = self.audio.as_ref() else {
            return false;
        };
        engine.play_on_bus(name, bus, gain, looping)
    }

    /// Set per-bus volume.
    pub fn set_audio_bus_volume(&mut self, bus: &str, volume: f32) {
        self.ensure_audio_engine();
        if let Some(engine) = self.audio.as_ref() {
            engine.set_bus_volume(bus, volume);
        }
    }

    /// Get per-bus volume.
    pub fn audio_bus_volume(&self, bus: &str) -> f32 {
        if let Some(engine) = self.audio.as_ref() {
            return engine.bus_volume(bus);
        }
        1.0
    }

    /// Stop all currently playing voices on a bus/layer.
    pub fn clear_audio_bus(&mut self, bus: &str) {
        self.ensure_audio_engine();
        if let Some(engine) = self.audio.as_ref() {
            engine.clear_bus(bus);
        }
    }

    /// Stop all currently playing clip voices across all buses/layers.
    pub fn clear_all_audio(&mut self) {
        self.ensure_audio_engine();
        if let Some(engine) = self.audio.as_ref() {
            engine.clear_all_buses();
        }
    }

    /// Create the server regions.
    pub fn create_regions(&mut self) {
        for (name, map) in &self.assets.maps {
            self.server
                .create_region_instance(name.clone(), map.clone(), &self.assets, "".into());
        }
        self.server.set_state(crate::ServerState::Running);
    }

    /// Process messages from the server to be displayed on the client.
    pub fn process_messages(&mut self, map: &Map, says: Vec<crate::server::Say>) {
        self.client.process_messages(map, says);
    }

    /*
    /// Build the client scene based on the maps camera mode, or, if the game is running on the PlayerCamera.
    pub fn build_scene(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        values: &ValueContainer,
        game_mode: bool,
    ) {
        if game_mode {
            if self.player_camera == PlayerCamera::D2 {
                if self.is_dirty_d2 {
                    self.client
                        .build_scene_d2(screen_size, map, &self.assets, values);
                    self.is_dirty_d2 = false;
                }
                self.set_d2();
            } else {
                if self.is_dirty_d3 {
                    self.client.build_scene_d3(map, &self.assets, values);
                    self.is_dirty_d3 = false;
                }
                self.set_d3();
            }
        } else {
            #[allow(clippy::collapsible_if)]
            if map.camera == MapCamera::TwoD {
                if self.is_dirty_d2 {
                    self.client
                        .build_scene_d2(screen_size, map, &self.assets, values);
                    self.is_dirty_d2 = false;
                }
                self.set_d2();
            } else {
                if self.is_dirty_d3 {
                    self.client.build_scene_d3(map, &self.assets, values);
                    self.is_dirty_d3 = false;
                }
                self.set_d3();
            }
        }
    }*/

    /// Apply the entities to the 3D scene.
    pub fn apply_entities_items(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        edit_surface: &Option<Surface>,
        draw_sectors: bool,
    ) {
        for e in map.entities.iter() {
            if e.is_player() {
                if let Some(Value::PlayerCamera(camera)) = e.attributes.get("player_camera") {
                    if *camera != self.player_camera {
                        self.player_camera = camera.clone();
                        match self.player_camera {
                            PlayerCamera::D3Iso => {
                                self.client.camera_d3 = Box::new(D3IsoCamera::new())
                            }
                            PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid => {
                                self.client.camera_d3 = Box::new(D3FirstPCamera::new());
                            }
                            PlayerCamera::D2 | PlayerCamera::D2Grid => {}
                        }
                    }
                    break;
                }
            }
        }
        if self.draw_mode == ClientDrawMode::D2 {
            self.apply_runtime_render_state(map, true);
            self.client.apply_entities_items_d2(
                screen_size,
                map,
                &self.assets,
                edit_surface,
                &mut self.scene_handler,
                draw_sectors,
            );
        } else if self.draw_mode == ClientDrawMode::D3 {
            self.client
                .apply_entities_items_d3(map, &self.assets, &mut self.scene_handler);
        }
    }

    /// Build the client scene in D2.
    pub fn build_custom_scene_d2(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        values: &ValueContainer,
        edit_surface: &Option<Surface>,
        draw_sectors: bool,
    ) {
        self.client.build_custom_scene_d2(
            screen_size,
            map,
            &self.assets,
            values,
            edit_surface,
            &mut self.scene_handler,
            draw_sectors,
        );
    }

    /// Builds the entities and items w/o changing char positions
    pub fn build_entities_items_d3(&mut self, map: &Map) {
        self.client.builder_d3.build_entities_items(
            map,
            self.client.camera_d3.as_ref(),
            &self.assets,
            &mut self.client.scene,
            &mut self.scene_handler,
        );
    }

    /// Build runtime 3D dynamic overlays (characters/items/lights) into SceneVM.
    pub fn build_dynamics_3d(&mut self, map: &Map, animation_frame: usize) {
        let camera = self.client.camera_d3.as_ref();
        self.scene_handler.build_dynamics_3d(
            map,
            camera,
            animation_frame,
            &self.assets,
            &Default::default(),
        );
    }

    /// Build runtime 2D dynamic overlays (characters/items/lights) into SceneVM.
    pub fn build_dynamics_2d(&mut self, map: &Map, animation_frame: usize) {
        self.scene_handler.build_dynamics_2d(
            map,
            animation_frame,
            &self.assets,
            &Default::default(),
        );
    }

    /// Build the client scene in D3.
    pub fn build_custom_scene_d3(&mut self, map: &Map, values: &ValueContainer) {
        self.client.build_custom_scene_d3(map, &self.assets, values);
    }

    /// Draw the client custom scene in 2D.
    pub fn draw_custom_d2(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        self.apply_runtime_render_state(map, true);
        self.client.draw_custom_d2(
            map,
            pixels,
            width,
            height,
            &self.assets,
            &mut self.scene_handler,
        );
    }

    /// Draw the client scene in 2D.
    pub fn draw_d2(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        self.apply_runtime_render_state(map, true);
        self.client.draw_d2(
            map,
            pixels,
            width,
            height,
            &self.assets,
            &mut self.scene_handler,
        );
    }

    /// Draw the client scene in 3D
    pub fn draw_d3(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        self.draw_d3_with_editor_background(map, pixels, width, height, false);
    }

    /// Draw the client scene in 3D, optionally using the neutral construction background.
    pub fn draw_d3_with_editor_background(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        editor_neutral_background: bool,
    ) {
        // Baking always uses the authored/game lighting state. The editor's
        // optional flat-lighting preview must not silently disable the sun in
        // the persisted result.
        let bake_rendering = self.orthographic_bake.is_rendering();
        self.apply_runtime_render_state(map, !bake_rendering);

        let camera = self
            .client
            .camera_d3
            .as_scenevm_camera_for_surface(width as f32, height as f32);
        self.orthographic_bake
            .sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
        let scene_bounds = self.scene_handler.vm.layer_scene_projected_bounds_3d(
            0,
            camera.right.normalized(),
            camera.up.normalized(),
        );
        let bake_work = self.orthographic_bake.prepare_work(
            map.id,
            width as u32,
            height as u32,
            camera,
            scene_bounds,
        );

        let bake_work = match bake_work {
            Ok(work) => work,
            Err(error) => {
                self.orthographic_bake.fail(error);
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
            // The bake deliberately excludes characters, editor preview icons,
            // particles, and doors. Rebuild them for the next live overlay pass.
            self.scene_handler.mark_dynamics_dirty();

            if self.orthographic_bake.finish_sample() {
                let captured = self.scene_handler.vm.capture_layer_rgba(0);
                self.scene_handler
                    .vm
                    .set_layer_progressive_sample_index(0, None);
                self.scene_handler.vm.set_layer_ping_pong_enabled(0, false);

                match captured {
                    Some((_width, _height, rgba)) => {
                        if let Err(error) = self.orthographic_bake.commit_current_tile(rgba) {
                            self.orthographic_bake.fail(error);
                        }
                    }
                    None => self
                        .orthographic_bake
                        .fail("The rendered bake could not be read back from the GPU."),
                }
            }

            if let Some(composed) =
                self.orthographic_bake
                    .compose_rgba(map.id, width as u32, height as u32, &camera)
            {
                let copy_len = pixels.len().min(composed.len());
                pixels[..copy_len].copy_from_slice(&composed[..copy_len]);
            } else {
                // Do not leave the previous committed bake (and any old baked
                // editor symbols) in the viewport before the first new tile is ready.
                pixels.fill(0);
            }
            return;
        }

        if let Some(mut composed) =
            self.orthographic_bake
                .compose_rgba(map.id, width as u32, height as u32, &camera)
        {
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
            blend_rgba_over(&mut composed, &self.orthographic_bake_overlay_pixels);
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

    /// Draw the client scene.
    pub fn draw_scene(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        match self.draw_mode {
            D2 => {
                self.apply_runtime_render_state(map, true);
                self.client.draw_d2(
                    map,
                    pixels,
                    width,
                    height,
                    &self.assets,
                    &mut self.scene_handler,
                );
            }
            D3 => {
                self.apply_runtime_render_state(map, true);
                self.client.draw_d3(
                    map,
                    pixels,
                    width,
                    height,
                    &self.assets,
                    &mut self.scene_handler,
                    false,
                );
            }
        }
    }

    /// Set up the client for processing the game.
    pub fn setup_client(&mut self) -> Vec<Command> {
        let cmds = self.client.setup(&mut self.assets, &mut self.scene_handler);
        self.load_audio_assets();
        cmds
    }

    /// Draw the game as the client sees it.
    pub fn draw_game(
        &mut self,
        map: &Map,
        messages: Vec<crate::server::Message>,
        says: Vec<crate::server::Say>,
        choices: Vec<crate::MultipleChoice>,
    ) {
        self.draw_game_with_widget_overlays(map, messages, says, choices, |_, _| false, |_, _| {});
    }

    /// Draw the game as the client sees it, with a callback that can update the
    /// 3D widget render state before the widget is copied into the client target.
    pub fn draw_game_with_widget_overlay<F>(
        &mut self,
        map: &Map,
        messages: Vec<crate::server::Message>,
        says: Vec<crate::server::Say>,
        choices: Vec<crate::MultipleChoice>,
        widget_overlay: F,
    ) where
        F: FnMut(&mut crate::client::widget::game::GameWidget, &mut SceneHandler) -> bool,
    {
        self.draw_game_with_widget_overlays(
            map,
            messages,
            says,
            choices,
            widget_overlay,
            |_, _| {},
        );
    }

    /// Draw the game with a pre-render SceneVM update and a post-render widget pixel overlay.
    pub fn draw_game_with_widget_overlays<F, G>(
        &mut self,
        map: &Map,
        messages: Vec<crate::server::Message>,
        says: Vec<crate::server::Say>,
        choices: Vec<crate::MultipleChoice>,
        mut widget_overlay: F,
        mut post_widget_overlay: G,
    ) where
        F: FnMut(&mut crate::client::widget::game::GameWidget, &mut SceneHandler) -> bool,
        G: FnMut(&mut crate::client::widget::game::GameWidget, &mut SceneHandler),
    {
        self.apply_runtime_render_state(map, false);
        let open_container_requests = self.server.get_open_container_requests(&map.id);
        self.client
            .process_open_container_requests(open_container_requests);
        self.client.process_messages(map, says);
        self.orthographic_bake
            .sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
        let bake = &self.orthographic_bake;
        let region_id = map.id;
        self.client.draw_game_with_widget_overlays(
            map,
            &self.assets,
            messages,
            choices,
            &mut self.scene_handler,
            |widget, scene_handler| {
                let result = widget_overlay(widget, scene_handler);
                let dim = *widget.buffer.dim();
                if dim.width > 0 && dim.height > 0 {
                    let camera = widget
                        .camera_d3
                        .as_scenevm_camera_for_surface(dim.width as f32, dim.height as f32);
                    if bake.can_compose(region_id, &camera) {
                        scene_handler.vm.clear_layer_frozen_rgba(0);
                        scene_handler
                            .vm
                            .set_layer_raster3d_static_geometry_enabled(0, false);
                        scene_handler.vm.set_active_vm(0);
                        scene_handler
                            .vm
                            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Raster3D));
                        scene_handler
                            .vm
                            .execute(scenevm::Atom::SetBackground(vek::Vec4::zero()));
                    }
                }
                result
            },
            |widget, scene_handler| {
                let dim = *widget.buffer.dim();
                if dim.width > 0 && dim.height > 0 {
                    let camera = widget
                        .camera_d3
                        .as_scenevm_camera_for_surface(dim.width as f32, dim.height as f32);
                    if let Some(mut composed) =
                        bake.compose_rgba(region_id, dim.width as u32, dim.height as u32, &camera)
                    {
                        let dynamics = widget.buffer.pixels().to_vec();
                        blend_rgba_over(&mut composed, &dynamics);
                        let pixels = widget.buffer.pixels_mut();
                        let copy_len = pixels.len().min(composed.len());
                        pixels[..copy_len].copy_from_slice(&composed[..copy_len]);
                    }
                }
                scene_handler
                    .vm
                    .set_layer_raster3d_static_geometry_enabled(0, true);
                post_widget_overlay(widget, scene_handler);
            },
        );
    }

    /// Prepare the game scene in SceneVM for direct window presentation.
    /// This avoids CPU readback and lets a caller present with SceneVM's native window path.
    pub fn prepare_game_scene_for_present(&mut self, map: &Map, size: (u32, u32)) -> bool {
        self.apply_runtime_render_state(map, false);
        self.client
            .prepare_scenevm_direct(map, &self.assets, &mut self.scene_handler, size)
    }

    /// Render only UI/screen widgets into a transparent overlay RGBA buffer.
    pub fn draw_ui_overlay_only(
        &mut self,
        map: &Map,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
        width: u32,
        height: u32,
    ) -> &TheRGBABuffer {
        self.apply_runtime_render_state(map, false);
        self.client
            .draw_ui_overlay_only(map, &self.assets, messages, choices, width, height)
    }

    /// Get first game widget rect in viewport coordinates.
    pub fn game_widget_rect(&self) -> Option<crate::Rect> {
        self.client.game_widget_rect()
    }

    pub fn for_each_game_widget_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut crate::client::widget::game::GameWidget),
    {
        self.client.for_each_game_widget_mut(f);
    }

    /// Get presentation transform from viewport coordinates into a target surface.
    pub fn presentation_transform_for_surface(&self, size: (u32, u32)) -> (f32, f32, f32) {
        self.client
            .presentation_transform_for_surface(size.0, size.1)
    }

    /// Clear active say bubbles from the client.
    pub fn clear_say_messages(&mut self) {
        self.client.clear_say_messages();
    }

    /// Send a touch down event to the client.
    pub fn client_touch_down(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        let action = self
            .client
            .touch_down(coord, map, &self.assets, &mut self.scene_handler);
        let commands = self
            .client
            .process_pending_runtime_commands(&mut self.assets, &mut self.scene_handler);
        if !commands.is_empty() {
            self.server.process_client_commands(commands);
        }
        action
    }

    /// Send a touch dragged event to the client.
    pub fn client_touch_dragged(&mut self, coord: Vec2<i32>, map: &Map) {
        self.client
            .touch_dragged(coord, map, &mut self.scene_handler);
    }

    /// Send a touch up event to the client.
    pub fn client_touch_up(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        self.client.touch_up(coord, map, &self.assets)
    }

    /// Send a touch hover event to the client.
    pub fn client_touch_hover(&mut self, coord: Vec2<i32>, map: &Map) {
        self.client
            .touch_hover(coord, map, &self.assets, &mut self.scene_handler);
    }

    /// Update the server messages.
    pub fn update_server(&mut self) -> Option<String> {
        self.server.update(&mut self.assets)
    }

    /// Update the tiles
    pub fn set_tiles(&mut self, textures: IndexMap<Uuid, Tile>, editor: bool) {
        let mut all_tiles = self.tiles_with_palette(textures);
        self.assets
            .materialize_geometry_material_tiles(&mut all_tiles);
        self.apply_tiles(all_tiles, editor);
    }

    pub fn set_block_props(&mut self, block_props: IndexMap<Uuid, BlockPropAsset>) {
        self.assets.set_block_props(block_props);
    }

    pub fn set_tiles_for_maps<'a, I>(
        &mut self,
        textures: IndexMap<Uuid, Tile>,
        editor: bool,
        maps: I,
    ) where
        I: IntoIterator<Item = &'a Map>,
    {
        let mut all_tiles = self.tiles_with_palette(textures);
        self.assets
            .materialize_geometry_material_tiles_for_maps(&mut all_tiles, maps);
        self.apply_tiles(all_tiles, editor);
    }

    fn tiles_with_palette(&self, textures: IndexMap<Uuid, Tile>) -> IndexMap<Uuid, Tile> {
        let mut all_tiles = textures;

        // Register synthetic 1x1 tiles for all palette indices so PaletteIndex sources
        // always resolve to real atlas/tile-list entries.
        for (idx, col_opt) in self.assets.palette.colors.iter().enumerate() {
            let Some(col) = col_opt else {
                continue;
            };
            let tile_id = Uuid::from_u128(0x50414C455454455F0000000000000000u128 | idx as u128);
            let material_id = self
                .assets
                .palette_material_ids
                .get(idx)
                .copied()
                .unwrap_or(0);
            let tile = all_tiles.entry(tile_id).or_insert_with(|| {
                let mut t = Tile::from_texture(Texture::from_color(col.to_u8_array()));
                t.id = tile_id;
                t
            });
            for texture in &mut tile.textures {
                texture.set_material_id_all(material_id);
            }
        }

        all_tiles
    }

    fn apply_tiles(&mut self, all_tiles: IndexMap<Uuid, Tile>, editor: bool) {
        self.scene_handler.build_atlas(&all_tiles, editor);
        self.assets.set_tiles(all_tiles);
        let palette: Vec<vek::Vec4<f32>> = self
            .assets
            .palette
            .colors
            .iter()
            .map(|entry| {
                if let Some(col) = entry {
                    let [r, g, b, a] = col.to_array();
                    let to_linear = |c: f32| c.max(0.0).powf(2.2);
                    vek::Vec4::new(to_linear(r), to_linear(g), to_linear(b), a)
                } else {
                    vek::Vec4::zero()
                }
            })
            .collect();
        self.scene_handler.vm.execute(Atom::SetPalette(palette));
        self.scene_handler.clear_runtime_scene();
    }

    pub fn set_tile_groups(&mut self, tile_groups: IndexMap<Uuid, TileGroup>) {
        self.assets.set_tile_groups(tile_groups);
    }
}

fn blend_rgba_over(base: &mut [u8], overlay: &[u8]) {
    for (dst, src) in base.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
        let alpha = src[3] as f32 / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        let inverse = 1.0 - alpha;
        // SceneVM's layer compositor has already premultiplied RGB while
        // rendering onto its transparent surface.
        for channel in 0..3 {
            dst[channel] = (src[channel] as f32 + dst[channel] as f32 * inverse)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        dst[3] = ((src[3] as f32) + dst[3] as f32 * inverse)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod bake_overlay_tests {
    use super::blend_rgba_over;

    #[test]
    fn blends_scenevm_premultiplied_overlay_over_bake() {
        let mut base = [100, 100, 100, 255];
        let overlay = [50, 25, 0, 128];
        blend_rgba_over(&mut base, &overlay);
        assert_eq!(base, [100, 75, 50, 255]);
    }
}
