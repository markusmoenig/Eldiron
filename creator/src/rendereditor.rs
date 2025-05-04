use crate::editor::{PALETTE, RUSTERIX};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use shared::prelude::*;

use rusterix::{D3Camera, D3FirstPCamera, ValueContainer};

pub enum RenderMoveAction {
    Forward,
    Backward,
    Left,
    Right,
}

pub struct RenderEditor {
    pub move_action: Option<RenderMoveAction>,
    pub firstp_camera: D3FirstPCamera,

    terrain_hit: Option<Vec3<f32>>,
    drag_coord: Vec2<i32>,

    hud: Hud,

    pub first_draw: bool,

    edited: bool,
}

#[allow(clippy::new_without_default)]
impl RenderEditor {
    pub fn new() -> Self {
        Self {
            move_action: None,
            firstp_camera: D3FirstPCamera::new(),

            terrain_hit: None,
            drag_coord: Vec2::zero(),

            hud: Hud::new(HudMode::Terrain),

            first_draw: true,

            edited: false,
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        build_values: &mut ValueContainer,
    ) {
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            let mut rusterix = RUSTERIX.write().unwrap();

            rusterix.client.camera_d3 = Box::new(self.firstp_camera.clone());

            let speed = 0.2;
            let yaw_step = 4.0;
            if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                match &self.move_action {
                    Some(RenderMoveAction::Forward) => {
                        let (np, nl) = self.move_camera(
                            region.editing_position_3d,
                            region.editing_look_at_3d,
                            Vec3::new(0.0, 0.0, 1.0),
                            speed,
                        );
                        region.editing_position_3d = np;
                        region.editing_look_at_3d = nl;
                    }
                    Some(RenderMoveAction::Backward) => {
                        let (np, nl) = self.move_camera(
                            region.editing_position_3d,
                            region.editing_look_at_3d,
                            Vec3::new(0.0, 0.0, -1.0),
                            speed,
                        );
                        region.editing_position_3d = np;
                        region.editing_look_at_3d = nl;
                    }
                    Some(RenderMoveAction::Left) => {
                        let nl = self.rotate_camera_y(
                            region.editing_position_3d,
                            region.editing_look_at_3d,
                            yaw_step,
                        );
                        region.editing_look_at_3d = nl;
                    }
                    Some(RenderMoveAction::Right) => {
                        let nl = self.rotate_camera_y(
                            region.editing_position_3d,
                            region.editing_look_at_3d,
                            -yaw_step,
                        );
                        region.editing_look_at_3d = nl;
                    }
                    None => {}
                }

                let position = region.editing_position_3d + Vec3::new(0.0, 1.5, 0.0);
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("position", position);
                let center = region.editing_look_at_3d + Vec3::new(0.0, 1.5, 0.0);
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("center", center);

                region.map.properties.remove("fog_enabled");
                if self.first_draw {
                    rusterix.build_scene_d3(&region.map, build_values);
                    rusterix.build_terrain_d3(&mut region.map, &ValueContainer::default());
                    self.first_draw = false;
                }

                // let assets = rusterix.assets.clone();
                // rusterix
                //     .client
                //     .apply_entities_items_d3(&region.map, &assets);

                if let Some(hit) = self.terrain_hit {
                    rusterix.client.terrain_hover = Some(hit);
                }
                rusterix.client.draw_d3(
                    &region.map,
                    buffer.pixels_mut(),
                    dim.width as usize,
                    dim.height as usize,
                );
                rusterix.client.terrain_hover = None;

                self.hud.draw(
                    buffer,
                    &mut region.map,
                    ctx,
                    server_ctx,
                    None,
                    &PALETTE.read().unwrap(),
                );
            }
        }
    }

    pub fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut hover = |coord: Vec2<i32>| {
            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();

                let rusterix = RUSTERIX.read().unwrap();
                let ray = rusterix.client.camera_d3.create_ray(
                    Vec2::new(
                        coord.x as f32 / dim.width as f32,
                        coord.y as f32 / dim.height as f32,
                    ),
                    Vec2::new(dim.width as f32, dim.height as f32),
                    Vec2::zero(),
                );

                self.terrain_hit = None;
                if let Some(hit) = map.terrain.ray_terrain_hit(&ray, 100.0) {
                    let p = self.world_to_editor(map.terrain.scale, hit.world_pos);
                    server_ctx.hover_cursor = Some(p);
                    self.terrain_hit = Some(hit.world_pos);
                    server_ctx.hover_height =
                        Some(map.terrain.sample_height(hit.world_pos.x, hit.world_pos.z));
                }
            }
        };

        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.edited = false;
            }
            MapEvent::MapUp(_coord) => {}
            MapEvent::MapDragged(coord) => {
                hover(*coord);
                self.drag_coord = *coord;
            }
            MapEvent::MapHover(coord) => hover(*coord),
            _ => {}
        }

        None
    }

    fn world_to_editor(&self, grid_scale: Vec2<f32>, world_pos: Vec3<f32>) -> Vec2<f32> {
        Vec2::new(world_pos.x / grid_scale.x, -world_pos.z / grid_scale.y)
    }

    pub fn scroll_by(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
        _coord: Vec2<i32>,
    ) {
        // self.orbit_camera.zoom(coord.y as f32);
    }

    fn camera_axes(&self, pos: Vec3<f32>, look_at: Vec3<f32>) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>) {
        let forward = (look_at - pos).normalized();
        let world_up = Vec3::unit_y();
        let right = forward.cross(world_up).normalized();
        let up = right.cross(forward);
        (forward, right, up)
    }

    fn move_camera(
        &self,
        mut pos: Vec3<f32>,
        mut look_at: Vec3<f32>,
        dir: Vec3<f32>, // e.g. (0,0,1) for “W”, (1,0,0) for “D” …
        speed: f32,
    ) -> (Vec3<f32>, Vec3<f32>) {
        let (fwd, right, up) = self.camera_axes(pos, look_at);
        let world_move = right * dir.x + up * dir.y + fwd * dir.z;
        let world_move = world_move * speed;
        pos += world_move;
        look_at += world_move;
        (pos, look_at)
    }

    pub fn rotate_camera_y(&self, pos: Vec3<f32>, look_at: Vec3<f32>, yaw_deg: f32) -> Vec3<f32> {
        let dir = look_at - pos; // current forward
        let r = yaw_deg.to_radians();
        let (s, c) = r.sin_cos();
        let new_dir = Vec3::new(dir.x * c + dir.z * s, dir.y, -dir.x * s + dir.z * c);
        pos + new_dir
    }

    pub fn rotate_camera_pitch(
        &self,
        pos: Vec3<f32>,
        look_at: Vec3<f32>,
        pitch_deg: f32,
    ) -> Vec3<f32> {
        let dir = look_at - pos; // current forward
        let len = dir.magnitude();
        if len == 0.0 {
            return look_at; // degeneracy guard
        }

        let forward = dir / len;
        let right = forward.cross(Vec3::unit_y()).normalized();

        let r = pitch_deg.to_radians();
        let (s, c) = r.sin_cos();

        let new_fwd =
            forward * c + right.cross(forward) * s + right * right.dot(forward) * (1.0 - c);

        pos + new_fwd * len // same distance, new dir
    }

    pub fn clamp_pitch(&self, old_pos: Vec3<f32>, new_look: Vec3<f32>, max_deg: f32) -> Vec3<f32> {
        let dir = (new_look - old_pos).normalized();
        let pitch = dir.y.asin().to_degrees(); // +90 top, -90 bottom
        let clamped = pitch.clamp(-max_deg, max_deg);

        if (pitch - clamped).abs() < 0.0001 {
            new_look
        } else {
            self.rotate_camera_pitch(old_pos, new_look, clamped - pitch)
        }
    }
}
