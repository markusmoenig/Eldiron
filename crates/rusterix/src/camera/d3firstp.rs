use crate::Ray;
use vek::{Mat4, Vec2, Vec3};

use super::D3Camera;

#[derive(Clone)]
pub struct D3FirstPCamera {
    pub position: Vec3<f32>,
    pub center: Vec3<f32>,

    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl D3Camera for D3FirstPCamera {
    fn new() -> Self {
        Self {
            position: Vec3::zero(),
            center: Vec3::zero(),

            fov: 75.0,
            near: 0.01,
            far: 100.0,
        }
    }

    fn id(&self) -> String {
        "firstp".to_string()
    }

    fn fov(&self) -> f32 {
        self.fov
    }

    fn view_matrix(&self) -> Mat4<f32> {
        vek::Mat4::look_at_rh(self.position, self.center, Vec3::unit_y())
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        vek::Mat4::perspective_fov_rh_zo(self.fov.to_radians(), width, height, self.near, self.far)
    }

    fn zoom(&mut self, delta: f32) {
        let zoom_sensitivity = 0.5;
        self.fov = (self.fov - delta * zoom_sensitivity).clamp(20.0, 120.0);
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        match key {
            "fov" => {
                self.fov = value;
            }
            "near" => {
                self.near = value;
            }
            "far" => {
                self.far = value;
            }
            _ => {}
        }
    }

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        #[allow(clippy::single_match)]
        match key {
            "position" => {
                self.position = value;
            }
            "center" => {
                self.center = value;
            }
            _ => {}
        }
    }

    fn position(&self) -> Vec3<f32> {
        self.position
    }

    fn basis_vectors(&self) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>) {
        let eps = 1e-8_f32;

        // Forward: from position to center. Fallback if zero.
        let mut forward = self.center - self.position;
        if forward.magnitude_squared() < eps {
            forward = Vec3::unit_z(); // default look direction
        }
        forward = forward.normalized();

        // Choose an up reference. If forward is nearly parallel to world up, pick Z as up ref.
        let world_up = Vec3::unit_y();
        let mut right = forward.cross(world_up);
        if right.magnitude_squared() < eps {
            // forward parallel to Y; use Z to compute a stable right
            right = forward.cross(Vec3::unit_z());
        }
        right = right.normalized();

        // True camera up from the orthonormal triad
        let mut up = right.cross(forward);
        if up.magnitude_squared() < eps {
            // As a last resort, recompute using X axis
            right = forward.cross(Vec3::unit_x()).normalized();
            up = right.cross(forward);
        }
        up = up.normalized();

        (forward, right, up)
    }

    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, offset: Vec2<f32>) -> Ray {
        let aspect = screen.x / screen.y;
        let pixel_size = Vec2::new(1.0 / screen.x, 1.0 / screen.y);

        let half_height = (self.fov.to_radians() * 0.5).tan();
        let half_width = half_height * aspect;

        let forward = (self.center - self.position).normalized();
        let right = forward.cross(Vec3::unit_y()).normalized();
        let up = right.cross(forward);

        let lower_left = self.position + forward - right * half_width - up * half_height;

        let horizontal = right * (2.0 * half_width);
        let vertical = up * (2.0 * half_height);

        let sample_pos = lower_left
            + horizontal * (pixel_size.x * offset.x + uv.x)
            + vertical * (pixel_size.y * offset.y + uv.y);

        let dir = (sample_pos - self.position).normalized();

        Ray {
            origin: self.position,
            dir,
        }
    }

    /// Generate a SceneVM camera
    fn as_scenevm_camera(&self) -> scenevm::Camera3D {
        let basis = self.basis_vectors();
        scenevm::Camera3D {
            kind: scenevm::CameraKind::FirstPersonPersp,
            pos: self.position,
            forward: basis.0,
            right: basis.1,
            up: basis.2,
            vfov_deg: self.fov,
            near: self.near,
            far: self.far,
            ..Default::default()
        }
    }
}
