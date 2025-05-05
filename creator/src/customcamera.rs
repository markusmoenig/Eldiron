use crate::editor::RUSTERIX;
use crate::prelude::*;
use theframework::prelude::*;

use rusterix::{D3Camera, D3FirstPCamera, D3IsoCamera};

pub enum CustomMoveAction {
    Forward,
    Backward,
    Left,
    Right,
}

pub struct CustomCamera {
    firstp_camera: D3FirstPCamera,
    iso_camera: D3IsoCamera,

    pub move_action: Option<CustomMoveAction>,
    last_mouse: Option<Vec2<i32>>,
}

#[allow(clippy::new_without_default)]
impl CustomCamera {
    pub fn new() -> Self {
        Self {
            firstp_camera: D3FirstPCamera::new(),
            iso_camera: D3IsoCamera::new(),

            move_action: None,
            last_mouse: None,
        }
    }

    pub fn setup_toolbar(
        &mut self,
        layout: &mut dyn TheHLayoutTrait,
        _ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        let mut camera_switch = TheGroupButton::new(TheId::named("Custom Camera Helper Switch"));
        camera_switch.add_text_status("FirstP".to_string(), str!("First Person camera."));
        camera_switch.add_text_status("Iso".to_string(), "Isometric camera.".to_string());
        camera_switch.set_index(server_ctx.curr_custom_tool_camera as i32);
        layout.add_widget(Box::new(camera_switch));

        layout.set_reverse_index(Some(1));
    }

    /// Update client camera
    pub fn update_camera(&mut self, region: &mut Region, server_ctx: &mut ServerContext) {
        let mut rusterix = RUSTERIX.write().unwrap();
        if server_ctx.curr_custom_tool_camera == CustomToolCamera::FirstP {
            rusterix.client.camera_d3 = Box::new(self.firstp_camera.clone());

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
        } else if server_ctx.curr_custom_tool_camera == CustomToolCamera::Isometric {
            rusterix.client.camera_d3 = Box::new(self.iso_camera.clone());

            rusterix
                .client
                .camera_d3
                .set_parameter_vec3("center", region.editing_position_3d);
            rusterix.client.camera_d3.set_parameter_vec3(
                "position",
                region.editing_position_3d + vek::Vec3::new(-20.0, 20.0, 20.0),
            );
        }
    }

    /// Update move actions
    pub fn update_action(&mut self, region: &mut Region, server_ctx: &mut ServerContext) {
        let speed = 0.2;
        let yaw_step = 4.0;
        if server_ctx.curr_custom_tool_camera == CustomToolCamera::FirstP {
            match &self.move_action {
                Some(CustomMoveAction::Forward) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(0.0, 0.0, 1.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::Backward) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(0.0, 0.0, -1.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::Left) => {
                    let nl = self.rotate_camera_y(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        yaw_step,
                    );
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::Right) => {
                    let nl = self.rotate_camera_y(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        -yaw_step,
                    );
                    region.editing_look_at_3d = nl;
                }
                None => {}
            }
        }
    }

    pub fn mouse_dragged(
        &mut self,
        region: &mut Region,
        server_ctx: &mut ServerContext,
        coord: &Vec2<i32>,
    ) {
        if server_ctx.curr_custom_tool_camera == CustomToolCamera::FirstP {
            let sens_yaw = 0.15; // deg per pixel horizontally
            let sens_pitch = 0.15; // deg per pixel vertically
            let max_pitch = 85.0; // don’t let camera flip

            let curr = *coord;

            if let Some(prev) = self.last_mouse {
                let dx = (curr.x - prev.x) as f32;
                let dy = (curr.y - prev.y) as f32;

                // Yaw   (left / right)
                if dx.abs() > 0.0 {
                    region.editing_look_at_3d = self.rotate_camera_y(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        -dx * sens_yaw, // screen → world: left = +yaw
                    );
                }
                // Pitch (up / down)
                if dy.abs() > 0.0 {
                    let look = self.rotate_camera_pitch(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        -dy * sens_pitch, // screen up = pitch up
                    );
                    region.editing_look_at_3d =
                        self.clamp_pitch(region.editing_position_3d, look, max_pitch);
                }
            }
            self.last_mouse = Some(curr);
        }
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
