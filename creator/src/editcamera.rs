use crate::prelude::*;
use theframework::prelude::*;

use rusterix::{D3Camera, D3FirstPCamera, D3IsoCamera, D3OrbitCamera, Rusterix};

pub enum CustomMoveAction {
    Forward,
    Backward,
    Left,
    Right,
    StrafeLeft,
    StrafeRight,
    Up,
    Down,
}

pub struct EditCamera {
    pub orbit_camera: D3OrbitCamera,
    pub iso_camera: D3IsoCamera,
    pub firstp_camera: D3FirstPCamera,

    pub move_action: Option<CustomMoveAction>,
    last_mouse: Option<Vec2<i32>>,
    fly_pointer: Option<(Vec2<i32>, Vec2<i32>)>,
    last_action_time_ms: Option<u128>,
}

#[allow(clippy::new_without_default)]
impl EditCamera {
    pub fn new() -> Self {
        Self {
            orbit_camera: D3OrbitCamera::new(),
            iso_camera: D3IsoCamera::new(),
            firstp_camera: D3FirstPCamera::new(),

            move_action: None,
            last_mouse: None,
            fly_pointer: None,
            last_action_time_ms: None,
        }
    }

    pub fn setup_toolbar(
        &mut self,
        layout: &mut dyn TheHLayoutTrait,
        _ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        let mut view_switch = TheGroupButton::new(TheId::named("Editor View Switch"));
        view_switch.add_text_status("2D".to_string(), "Edit the map in 2D.".to_string());
        if cfg!(target_os = "macos") {
            view_switch.add_text_status(
                    "Orbit".to_string(),
                    "Edit the map with a 3D orbit camera. Scroll to move. Cmd + Scroll to zoom. Alt + Scroll to rotate.".to_string(),
                );
        } else {
            view_switch.add_text_status(
                    "Orbit".to_string(),
                    "Edit the map with a 3D orbit camera. Scroll to move. Ctrl + Scroll to zoom. Alt + Scroll to rotate.".to_string(),
                );
        }
        if cfg!(target_os = "macos") {
            view_switch.add_text_status(
                "Iso".to_string(),
                "Edit the map in 3D isometric view. Scroll to move. Cmd + Scroll to zoom. "
                    .to_string(),
            );
        } else {
            view_switch.add_text_status(
                "Iso".to_string(),
                "Edit the map in 3D isometric view. Scroll to move. Ctrl + Scroll to zoom."
                    .to_string(),
            );
        }
        view_switch.add_text_status(
            "FirstP".to_string(),
            "Edit the map in 3D first person view.".to_string(),
        );
        view_switch.set_index(server_ctx.editor_view_mode.to_index());
        layout.add_widget(Box::new(view_switch));
        layout.set_reverse_index(Some(1));
    }

    /// Update client camera
    pub fn update_camera(
        &mut self,
        region: &mut Region,
        server_ctx: &mut ServerContext,
        rusterix: &mut Rusterix,
    ) {
        if server_ctx.editor_view_mode == EditorViewMode::FirstP {
            rusterix.client.camera_d3 = Box::new(self.firstp_camera.clone());

            let height = 1.5;

            // let h = region.map.terrain.get_height_unprocessed(
            //     region.editing_position_3d.x as i32,
            //     region.editing_position_3d.z as i32,
            // );

            // println!("{} {:?}", height, h);

            let position = region.editing_position_3d + Vec3::new(0.0, height, 0.0);

            rusterix
                .client
                .camera_d3
                .set_parameter_vec3("position", position);
            let center = region.editing_look_at_3d + Vec3::new(0.0, 1.5, 0.0);
            rusterix
                .client
                .camera_d3
                .set_parameter_vec3("center", center);
        } else if server_ctx.editor_view_mode == EditorViewMode::Iso {
            rusterix.client.camera_d3 = Box::new(self.iso_camera.clone());

            rusterix.client.camera_d3.set_parameter_f32(
                "azimuth_deg",
                self.iso_camera.get_parameter_f32("azimuth_deg"),
            );

            rusterix.client.camera_d3.set_parameter_f32(
                "elevation_deg",
                self.iso_camera.get_parameter_f32("elevation_deg"),
            );

            rusterix
                .client
                .camera_d3
                .set_parameter_vec3("center", region.editing_position_3d);
            rusterix.client.camera_d3.set_parameter_vec3(
                "position",
                region.editing_position_3d + vek::Vec3::new(-20.0, 20.0, 20.0),
            );
        } else if server_ctx.editor_view_mode == EditorViewMode::Orbit {
            rusterix.client.camera_d3 = Box::new(self.orbit_camera.clone());

            rusterix
                .client
                .camera_d3
                .set_parameter_vec3("center", region.editing_position_3d);
        }
    }

    /// Update move actions
    pub fn update_action(
        &mut self,
        region: &mut Region,
        server_ctx: &mut ServerContext,
        now_ms: u128,
    ) {
        let dt = if let Some(prev) = self.last_action_time_ms {
            (((now_ms.saturating_sub(prev)) as f32) / 1000.0).clamp(1.0 / 240.0, 1.0 / 20.0)
        } else {
            1.0 / 60.0
        };
        self.last_action_time_ms = Some(now_ms);

        let speed = 5.5 * dt;
        let yaw_step = 120.0 * dt;
        if server_ctx.editor_view_mode == EditorViewMode::FirstP {
            if server_ctx.editor_fly_nav_active {
                self.update_fly_pointer_look(region, dt);
            }
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
                Some(CustomMoveAction::StrafeLeft) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(-1.0, 0.0, 0.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::StrafeRight) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(1.0, 0.0, 0.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::Up) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(0.0, 1.0, 0.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                Some(CustomMoveAction::Down) => {
                    let (np, nl) = self.move_camera(
                        region.editing_position_3d,
                        region.editing_look_at_3d,
                        Vec3::new(0.0, -1.0, 0.0),
                        speed,
                    );
                    region.editing_position_3d = np;
                    region.editing_look_at_3d = nl;
                }
                None => {
                    self.last_action_time_ms = None;
                }
            }
        } else {
            self.last_action_time_ms = None;
        }
    }

    pub fn reset_mouse_tracking(&mut self) {
        self.last_mouse = None;
        self.fly_pointer = None;
    }

    pub fn set_fly_pointer(&mut self, coord: &Vec2<i32>, view_size: Vec2<i32>) {
        self.fly_pointer = Some((*coord, view_size));
    }

    pub fn mouse_dragged_firstp(&mut self, region: &mut Region, coord: &Vec2<i32>) {
        let sens_yaw = 0.05; // deg per pixel horizontally
        let sens_pitch = 0.05; // deg per pixel vertically
        let max_pitch = 85.0; // do not let the camera flip

        let curr = *coord;

        if let Some(prev) = self.last_mouse {
            let dx = (curr.x - prev.x) as f32;
            let dy = (curr.y - prev.y) as f32;

            if dx.abs() > 0.0 {
                region.editing_look_at_3d = self.rotate_camera_y(
                    region.editing_position_3d,
                    region.editing_look_at_3d,
                    -dx * sens_yaw,
                );
            }
            if dy.abs() > 0.0 {
                let look = self.rotate_camera_pitch(
                    region.editing_position_3d,
                    region.editing_look_at_3d,
                    -dy * sens_pitch,
                );
                region.editing_look_at_3d =
                    self.clamp_pitch(region.editing_position_3d, look, max_pitch);
            }
        }
        self.last_mouse = Some(curr);
    }

    fn update_fly_pointer_look(&mut self, region: &mut Region, dt: f32) {
        let Some((coord, view_size)) = self.fly_pointer else {
            return;
        };
        let half_w = (view_size.x.max(1) as f32) * 0.5;
        let half_h = (view_size.y.max(1) as f32) * 0.5;
        let x = ((coord.x as f32 - half_w) / half_w).clamp(-1.0, 1.0);
        let y = ((coord.y as f32 - half_h) / half_h).clamp(-1.0, 1.0);

        fn response(value: f32) -> f32 {
            let dead_zone = 0.12;
            let abs = value.abs();
            if abs <= dead_zone {
                0.0
            } else {
                value.signum() * ((abs - dead_zone) / (1.0 - dead_zone)).powf(1.35)
            }
        }

        let yaw = response(x) * 80.0 * dt;
        let pitch = response(y) * 55.0 * dt;
        if yaw != 0.0 {
            region.editing_look_at_3d =
                self.rotate_camera_y(region.editing_position_3d, region.editing_look_at_3d, -yaw);
        }
        if pitch != 0.0 {
            let look = self.rotate_camera_pitch(
                region.editing_position_3d,
                region.editing_look_at_3d,
                -pitch,
            );
            region.editing_look_at_3d = self.clamp_pitch(region.editing_position_3d, look, 85.0);
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

    pub fn scroll_by(&mut self, coord: f32, server_ctx: &mut ServerContext) {
        if server_ctx.editor_view_mode == EditorViewMode::Iso {
            self.iso_camera.zoom(coord);
        } else if server_ctx.editor_view_mode == EditorViewMode::Orbit {
            self.orbit_camera.zoom(coord);
        } else if server_ctx.editor_view_mode == EditorViewMode::FirstP {
            self.firstp_camera.zoom(coord);
        }
    }

    pub fn rotate(&mut self, delta: Vec2<f32>, server_ctx: &mut ServerContext) {
        if server_ctx.editor_view_mode == EditorViewMode::Orbit {
            self.orbit_camera.rotate(delta);
        }
    }
}
