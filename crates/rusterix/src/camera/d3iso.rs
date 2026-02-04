use crate::{D3Camera, Ray};
use vek::{FrustumPlanes, Mat4, Vec2, Vec3};

// // Classic ISO
// camera.set_parameter_f32("azimuth_deg", 135.0); // 45.0
// camera.set_parameter_f32("elevation_deg", 35.2643897);

// // Ultima Online
// camera.set_parameter_f32("azimuth_deg", 135.0); // 45.0
// camera.set_parameter_f32("elevation_deg", 25.0);

// // Top-Down
// camera.set_parameter_f32("azimuth_deg", 0.0);
// camera.set_parameter_f32("elevation_deg", 90.0);

/// A configurable orthographic isometric camera.
#[derive(Clone)]
pub struct D3IsoCamera {
    pub center: Vec3<f32>,

    /// Azimuth (yaw) around +Y in **degrees** (0° = +X, 90° = +Z)
    pub azimuth_deg: f32,
    /// Elevation (pitch) from the XZ-plane in **degrees** (0° = horizontal, +90° = +Y)
    pub elevation_deg: f32,

    /// Distance from the center along -forward to the camera position
    pub distance: f32,
    /// Half-height of the ortho frustum in world units
    pub scale: f32,

    /// Ortho near/far planes
    pub near: f32,
    pub far: f32,
}

impl D3IsoCamera {
    #[inline]
    fn basis(&self) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>) {
        // Convert angles to radians
        let yaw = self.azimuth_deg.to_radians();
        let pitch = self.elevation_deg.to_radians();

        // Forward: standard yaw/pitch with Y as up
        // yaw=0 -> +X, yaw=90 -> +Z; pitch=0 -> horizontal, pitch=+90 -> +Y
        let cp = pitch.cos();
        let sp = pitch.sin();
        let cy = yaw.cos();
        let sy = yaw.sin();
        let forward = Vec3::new(cy * cp, sp, sy * cp).normalized();

        // Build right/up to match a right-handed look_at_rh basis
        let mut right = forward.cross(Vec3::unit_y());
        if right.magnitude_squared() < 1e-6 {
            right = Vec3::unit_x();
        }
        let right = right.normalized();
        let up = right.cross(forward).normalized();

        (forward, right, up)
    }

    #[inline]
    fn position(&self) -> Vec3<f32> {
        let (forward, _right, _up) = self.basis();
        // Camera position is behind the center along -forward
        self.center + forward * self.distance
    }
}

impl D3Camera for D3IsoCamera {
    fn new() -> Self {
        Self {
            center: Vec3::zero(),
            // Classic isometric defaults: yaw=135°, pitch≈35.264° (atan(1/sqrt(2)))
            azimuth_deg: 135.0, //45.0,
            elevation_deg: 35.2643897,

            distance: 20.0,
            scale: 4.0,

            near: 0.1,
            far: 100.0,
        }
    }

    fn id(&self) -> String {
        "iso".to_string()
    }

    fn scale(&self) -> f32 {
        self.scale
    }

    /// Zoom the camera in or out based on vertical mouse delta (changes ortho half-height)
    fn zoom(&mut self, delta: f32) {
        let zoom_sensitivity = 0.05;
        let zoom_factor = (1.0 - delta * zoom_sensitivity).clamp(0.5, 2.0);
        self.scale *= zoom_factor;
        self.scale = self.scale.clamp(2.0, 70.0);
    }

    fn view_matrix(&self) -> Mat4<f32> {
        let (_forward, _right, up) = self.basis();
        let pos = self.position();
        let target = self.center; // pos + forward * self.distance also equals center
        Mat4::look_at_rh(pos, target, up)
    }

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32> {
        let half_h = self.scale;
        let half_w = half_h * (width / height).max(1e-6);
        Mat4::orthographic_rh_no(FrustumPlanes {
            left: -half_w,
            right: half_w,
            bottom: -half_h,
            top: half_h,
            near: self.near,
            far: self.far,
        })
    }

    fn get_parameter_f32(&mut self, key: &str) -> f32 {
        match key {
            "azimuth_deg" | "yaw_deg" => self.azimuth_deg,
            "elevation_deg" | "pitch_deg" => self.elevation_deg,
            _ => 0.0,
        }
    }

    fn set_parameter_f32(&mut self, key: &str, value: f32) {
        match key {
            "scale" => self.scale = value.max(0.001),
            "distance" => self.distance = value.max(0.001),
            "azimuth_deg" | "yaw_deg" => self.azimuth_deg = value,
            "elevation_deg" | "pitch_deg" => self.elevation_deg = value.clamp(-89.9, 89.9),
            "near" => self.near = value.max(1e-4),
            "far" => self.far = value.max(self.near + 1e-3),
            _ => {}
        }
    }

    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {
        if key == "center" {
            self.center = value;
        }
    }

    fn position(&self) -> Vec3<f32> {
        self.position()
    }

    fn basis_vectors(&self) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>) {
        let (forward_to_camera, right_to_camera, up) = self.basis();
        let forward = -forward_to_camera; // from eye to center
        let right = -right_to_camera; // align right with camera X+
        (forward.normalized(), right.normalized(), up)
    }

    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, jitter: Vec2<f32>) -> Ray {
        // Build the exact same basis & position used by view_matrix()
        let (_forward, right, up) = self.basis();
        let cam_origin = self.position();

        // Orthographic extents
        let half_h = self.scale;
        let half_w = half_h * (screen.x / screen.y).max(1e-6);

        // Full-viewport vectors in world space
        let horizontal = -right * (2.0 * half_w);
        let vertical = up * (2.0 * half_h);

        // Pixel scale for TAA/jitter
        let pixel_size = Vec2::new(1.0 / screen.x.max(1.0), 1.0 / screen.y.max(1.0));

        // Map uv in [0,1]² to world-space offset on the ortho plane, centered at 0.5
        let origin = cam_origin
            + horizontal * (pixel_size.x * jitter.x + uv.x - 0.5)
            + vertical * (pixel_size.y * jitter.y + uv.y - 0.5);

        // Orthographic rays share the same direction
        Ray::new(origin, (self.center - cam_origin).normalized())
    }

    /// Generate a SceneVM camera
    fn as_scenevm_camera(&self) -> scenevm::Camera3D {
        let (forward_to_camera, right_to_camera, up_orig) = self.basis();
        let forward_to_camera = forward_to_camera.normalized();
        let right_to_camera = right_to_camera.normalized();
        let up = up_orig.normalized();

        // Ensure the exported camera stays far enough so that the bottom of the ortho slab
        // does not drop below the focus point when zooming out.
        let axis = Vec3::unit_y();
        let up_proj = up.dot(axis).abs();
        let forward_proj = forward_to_camera.dot(axis).abs();
        let required_distance = if forward_proj > 1e-4 {
            self.distance.max((self.scale * up_proj) / forward_proj)
        } else {
            self.distance
        };

        let pos = self.center + forward_to_camera * required_distance;
        let forward = -forward_to_camera;
        let right = -right_to_camera;

        scenevm::Camera3D {
            kind: scenevm::CameraKind::OrthoIso,
            pos,
            forward,
            right,
            up,
            ortho_half_h: self.scale,
            near: self.near,
            far: self.far,
            ..Default::default()
        }
    }
}
