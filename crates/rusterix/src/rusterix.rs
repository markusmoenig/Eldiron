use crate::{AccumBuffer, Command, PlayerCamera, SceneHandler, Surface, prelude::*};
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

    pub is_dirty_d2: bool,
    pub is_dirty_d3: bool,
    pub draw_mode: ClientDrawMode,

    pub player_camera: PlayerCamera,

    pub scene_handler: SceneHandler,
}

impl Default for Rusterix {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusterix {
    pub fn new() -> Self {
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

            is_dirty_d2: true,
            is_dirty_d3: true,
            draw_mode: ClientDrawMode::D3,

            player_camera: PlayerCamera::D2,

            scene_handler,
        }
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
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets
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
    pub fn process_messages(&mut self, map: &Map, messages: Vec<crate::server::Message>) {
        self.client.process_messages(map, messages);
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
                        if self.player_camera == PlayerCamera::D3Iso {
                            self.client.camera_d3 = Box::new(D3IsoCamera::new())
                        } else if self.player_camera == PlayerCamera::D3FirstP {
                            self.client.camera_d3 = Box::new(D3FirstPCamera::new());
                        }
                    }
                    break;
                }
            }
        }
        if self.draw_mode == ClientDrawMode::D2 {
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

    /// Build the client scene in D3.
    pub fn build_custom_scene_d3(&mut self, map: &Map, values: &ValueContainer) {
        self.client.build_custom_scene_d3(map, &self.assets, values);
    }

    /// Draw the client custom scene in 2D.
    pub fn draw_custom_d2(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
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
        self.client.draw_d3(
            map,
            pixels,
            width,
            height,
            &self.assets,
            &mut self.scene_handler,
        );
    }

    /// Draw the client scene.
    pub fn draw_scene(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        match self.draw_mode {
            D2 => {
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
                self.client.draw_d3(
                    map,
                    pixels,
                    width,
                    height,
                    &self.assets,
                    &mut self.scene_handler,
                );
            }
        }
    }

    pub fn trace_scene(&mut self, accum: &mut AccumBuffer) {
        self.client.trace(accum, &self.assets);
    }

    /// Set up the client for processing the game.
    pub fn setup_client(&mut self) -> Vec<Command> {
        self.client.setup(&mut self.assets, &mut self.scene_handler)
    }

    /// Draw the game as the client sees it.
    pub fn draw_game(
        &mut self,
        map: &Map,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
    ) {
        self.client.draw_game(
            map,
            &self.assets,
            messages,
            choices,
            &mut self.scene_handler,
        );
    }

    /// Send a touch dragged event to the client.
    pub fn client_touch_dragged(&mut self, coord: Vec2<i32>, map: &Map) {
        self.client
            .touch_dragged(coord, map, &mut self.scene_handler);
    }

    /// Send a touch hover event to the client.
    pub fn client_touch_hover(&mut self, coord: Vec2<i32>, map: &Map) {
        self.client.touch_hover(coord, map, &mut self.scene_handler);
    }

    /// Update the server messages.
    pub fn update_server(&mut self) -> Option<String> {
        self.server.update(&mut self.assets)
    }

    /// Update the tiles
    pub fn set_tiles(&mut self, textures: IndexMap<Uuid, Tile>, editor: bool) {
        self.scene_handler.build_atlas(&textures, editor);
        self.assets.set_tiles(textures);
    }
}
